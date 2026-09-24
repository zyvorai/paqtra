// Native Hubble client: talks to the Observer gRPC API of Hubble Relay (or an
// agent) directly, so the `hubble` binary is not needed.
//
// The protos are vendored in `proto/cilium` and compiled by `build.rs`.

use crate::models::flow::{Flow, FlowEndpoint};
use anyhow::{Context, Result};
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};

/// Generated protobuf types. The generated files refer to each other as
/// `super::flow`, `super::relay`, so they must be sibling modules.
#[allow(clippy::all, dead_code, unused_qualifications)]
pub mod pb {
    pub mod flow {
        include!(concat!(env!("OUT_DIR"), "/flow.rs"));
    }
    pub mod relay {
        include!(concat!(env!("OUT_DIR"), "/relay.rs"));
    }
    pub mod observer {
        include!(concat!(env!("OUT_DIR"), "/observer.rs"));
    }
}

use pb::flow::{layer4, layer7, Flow as PbFlow, FlowFilter, Verdict};
use pb::observer::{
    get_flows_response::ResponseTypes, observer_client::ObserverClient, GetFlowsRequest,
    GetFlowsResponse, ServerStatusRequest,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Bound for one-shot calls. A snapshot of 10,000 flows from a large cluster
/// can take a while, but not unboundedly.
const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(20);
const STATUS_TIMEOUT: Duration = Duration::from_secs(3);

/// Connect to an Observer at `host:port` (plaintext gRPC, which is how Hubble
/// Relay serves inside a cluster).
async fn connect(address: &str) -> Result<ObserverClient<Channel>> {
    let channel = Endpoint::from_shared(format!("http://{address}"))
        .with_context(|| format!("invalid Hubble address '{address}'"))?
        .connect_timeout(CONNECT_TIMEOUT)
        .tcp_nodelay(true)
        .connect()
        .await
        .with_context(|| format!("cannot connect to Hubble at {address}"))?;
    Ok(ObserverClient::new(channel))
}

/// Whether the Observer at `address` answers `ServerStatus`.
pub async fn is_healthy(address: &str) -> bool {
    let call = async {
        let mut client = connect(address).await?;
        client
            .server_status(ServerStatusRequest {})
            .await
            .map(|_| ())
            .map_err(anyhow::Error::from)
    };
    matches!(tokio::time::timeout(STATUS_TIMEOUT, call).await, Ok(Ok(())))
}

/// The filter the `hubble observe --namespace` flag builds: a flow matches when
/// either end is in the namespace. `whitelist` entries are OR-ed.
fn namespace_filters(namespace: Option<&str>) -> Vec<FlowFilter> {
    let Some(ns) = namespace else {
        return Vec::new();
    };
    let prefix = format!("{ns}/");
    vec![
        FlowFilter {
            source_pod: vec![prefix.clone()],
            ..Default::default()
        },
        FlowFilter {
            destination_pod: vec![prefix],
            ..Default::default()
        },
    ]
}

/// The most recent `limit` flows, oldest first.
pub async fn last_flows(
    address: &str,
    limit: usize,
    namespace: Option<&str>,
    cluster: Option<&str>,
) -> Result<Vec<Flow>> {
    let call = async {
        let mut client = connect(address).await?;
        let mut stream = client
            .get_flows(GetFlowsRequest {
                number: limit as u64,
                follow: false,
                whitelist: namespace_filters(namespace),
                ..Default::default()
            })
            .await
            .context("GetFlows failed")?
            .into_inner();
        let mut flows = Vec::new();
        while let Some(msg) = stream.message().await.context("GetFlows stream failed")? {
            if let Some(mut flow) = flow_from_response(&msg) {
                if let Some(c) = cluster {
                    flow.cluster = Some(c.to_string());
                }
                flows.push(flow);
            }
        }
        Ok::<_, anyhow::Error>(flows)
    };
    tokio::time::timeout(SNAPSHOT_TIMEOUT, call)
        .await
        .context("timed out reading flows from Hubble")?
}

/// Follow new flows. Connection and request errors surface here; errors after
/// the stream has started arrive as `Err` items.
pub async fn follow_flows(
    address: &str,
    namespace: Option<&str>,
) -> Result<tonic::Streaming<GetFlowsResponse>> {
    let mut client = connect(address).await?;
    let stream = client
        .get_flows(GetFlowsRequest {
            follow: true,
            whitelist: namespace_filters(namespace),
            ..Default::default()
        })
        .await
        .context("GetFlows failed")?
        .into_inner();
    Ok(stream)
}

/// A flow in a stream message, or None for lost-event, node-status and agent
/// messages (which carry no flow).
pub fn flow_from_response(msg: &GetFlowsResponse) -> Option<Flow> {
    let ResponseTypes::Flow(flow) = msg.response_types.as_ref()? else {
        return None;
    };
    let mut out = flow_to_model(flow);
    if out.timestamp.is_empty() {
        // The response envelope carries a time as well.
        if let Some(t) = &msg.time {
            out.timestamp = format_timestamp(t);
        }
    }
    Some(out)
}

fn format_timestamp(t: &prost_types::Timestamp) -> String {
    use chrono::{DateTime, SecondsFormat};
    // A zero Timestamp is how protobuf spells "unset"; treat it as absent.
    if t.seconds == 0 && t.nanos == 0 {
        return String::new();
    }
    DateTime::from_timestamp(t.seconds, t.nanos.max(0) as u32)
        .map(|d| d.to_rfc3339_opts(SecondsFormat::AutoSi, true))
        .unwrap_or_default()
}

/// Convert a Hubble flow into our canonical `Flow`.
pub fn flow_to_model(f: &PbFlow) -> Flow {
    let endpoint = |ep: &Option<pb::flow::Endpoint>, ip: &str| FlowEndpoint {
        namespace: ep.as_ref().map(|e| e.namespace.clone()).unwrap_or_default(),
        pod: ep.as_ref().map(|e| e.pod_name.clone()).unwrap_or_default(),
        ip: ip.to_string(),
    };
    let (src_ip, dst_ip) =
        f.ip.as_ref()
            .map(|ip| (ip.source.as_str(), ip.destination.as_str()))
            .unwrap_or(("", ""));
    let source = endpoint(&f.source, src_ip);
    let destination = endpoint(&f.destination, dst_ip);

    let verdict = Verdict::try_from(f.verdict)
        .map(|v| v.as_str_name())
        .unwrap_or("UNKNOWN")
        .to_string();

    let (protocol, port) = match f.l4.as_ref().and_then(|l| l.protocol.as_ref()) {
        Some(layer4::Protocol::Tcp(t)) => ("TCP", t.destination_port),
        Some(layer4::Protocol::Udp(u)) => ("UDP", u.destination_port),
        Some(layer4::Protocol::Sctp(s)) => ("SCTP", s.destination_port),
        Some(layer4::Protocol::IcmPv4(_)) => ("ICMPv4", 0),
        Some(layer4::Protocol::IcmPv6(_)) => ("ICMPv6", 0),
        Some(layer4::Protocol::Vrrp(_)) => ("VRRP", 0),
        Some(layer4::Protocol::Igmp(_)) => ("IGMP", 0),
        None => ("UNKNOWN", 0),
    };
    let port = u16::try_from(port).unwrap_or(0);
    let protocol = protocol.to_string();

    let http = match f.l7.as_ref().and_then(|l| l.record.as_ref()) {
        Some(layer7::Record::Http(h)) => Some(h),
        _ => None,
    };
    let (dns, latency_ns) = match f.l7.as_ref() {
        Some(l7) => {
            let dns = match l7.record.as_ref() {
                Some(layer7::Record::Dns(d)) => Some(d),
                _ => None,
            };
            (dns, Some(l7.latency_ns).filter(|&n| n > 0))
        }
        None => (None, None),
    };

    let timestamp = f.time.as_ref().map(format_timestamp).unwrap_or_default();

    // Hubble's own id when it sends one; otherwise one derived from the flow's
    // content, so the same flow has the same id on every poll.
    let id = if f.uuid.is_empty() {
        super::hubble::content_id(&timestamp, &source, &destination, &verdict, &protocol, port)
    } else {
        f.uuid.clone()
    };

    let drop_reason = {
        use pb::flow::DropReason;
        DropReason::try_from(f.drop_reason_desc)
            .ok()
            .filter(|d| *d != DropReason::Unknown)
            .map(|d| d.as_str_name().to_string())
    };

    let (dns_query, dns_qtypes, dns_rcode, dns_rcode_name, dns_ips) = if let Some(d) = dns {
        (
            Some(d.query.clone()).filter(|q| !q.is_empty()),
            Some(d.qtypes.clone()).filter(|q| !q.is_empty()),
            Some(d.rcode),
            Some(crate::models::flow::dns_rcode_name(d.rcode).to_string()),
            Some(d.ips.clone()).filter(|i| !i.is_empty()),
        )
    } else {
        (None, None, None, None, None)
    };

    Flow {
        id,
        timestamp,
        source,
        destination,
        verdict,
        protocol,
        port,
        http_method: http.map(|h| h.method.clone()),
        http_url: http.map(|h| h.url.clone()),
        http_code: http.map(|h| u16::try_from(h.code).unwrap_or(0)),
        cluster: None,
        dns_query,
        dns_qtypes,
        dns_rcode,
        dns_rcode_name,
        dns_ips,
        dns_latency_ns: latency_ns,
        drop_reason,
    }
}

/// An in-process fake Observer, shared with the service-level tests.
#[cfg(test)]
pub(crate) mod testing {
    use super::pb::flow::{Endpoint, Ip, Layer4, Tcp};
    use super::pb::observer::{
        observer_server::{Observer, ObserverServer},
        ServerStatusResponse,
    };
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio_stream::wrappers::TcpListenerStream;

    pub fn ts(seconds: i64, nanos: i32) -> Option<prost_types::Timestamp> {
        Some(prost_types::Timestamp { seconds, nanos })
    }

    pub fn ep(ns: &str, pod: &str) -> Option<Endpoint> {
        Some(Endpoint {
            namespace: ns.into(),
            pod_name: pod.into(),
            ..Default::default()
        })
    }

    pub fn tcp_flow(uuid: &str, verdict: Verdict, port: u32) -> PbFlow {
        PbFlow {
            time: ts(1_700_000_000, 123_456_000),
            uuid: uuid.into(),
            verdict: verdict as i32,
            ip: Some(Ip {
                source: "10.0.0.1".into(),
                destination: "10.0.0.2".into(),
                ..Default::default()
            }),
            l4: Some(Layer4 {
                protocol: Some(layer4::Protocol::Tcp(Tcp {
                    source_port: 40000,
                    destination_port: port,
                    flags: None,
                })),
            }),
            source: ep("shop", "web-1"),
            destination: ep("db", "pg-0"),
            ..Default::default()
        }
    }

    // ---- against an in-process fake Observer ----

    /// Serves a fixed set of flows and records the requests it received.
    pub struct Fake {
        pub flows: Vec<PbFlow>,
        pub seen: Arc<Mutex<Vec<GetFlowsRequest>>>,
    }

    type FlowStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = Result<GetFlowsResponse, tonic::Status>> + Send>,
    >;

    #[tonic::async_trait]
    impl Observer for Fake {
        type GetFlowsStream = FlowStream;
        async fn get_flows(
            &self,
            request: tonic::Request<GetFlowsRequest>,
        ) -> Result<tonic::Response<Self::GetFlowsStream>, tonic::Status> {
            let req = request.into_inner();
            self.seen.lock().unwrap().push(req.clone());
            let mut items: Vec<Result<GetFlowsResponse, tonic::Status>> = Vec::new();
            // A node-status message first, as Relay sends: must not become a flow.
            items.push(Ok(GetFlowsResponse::default()));
            let take = if req.number == 0 {
                self.flows.len()
            } else {
                (req.number as usize).min(self.flows.len())
            };
            for f in &self.flows[self.flows.len() - take..] {
                items.push(Ok(GetFlowsResponse {
                    response_types: Some(ResponseTypes::Flow(f.clone())),
                    ..Default::default()
                }));
            }
            Ok(tonic::Response::new(Box::pin(tokio_stream::iter(items))))
        }
        async fn server_status(
            &self,
            _: tonic::Request<ServerStatusRequest>,
        ) -> Result<tonic::Response<ServerStatusResponse>, tonic::Status> {
            Ok(tonic::Response::new(ServerStatusResponse::default()))
        }

        type GetAgentEventsStream = std::pin::Pin<
            Box<
                dyn tokio_stream::Stream<
                        Item = Result<pb::observer::GetAgentEventsResponse, tonic::Status>,
                    > + Send,
            >,
        >;
        async fn get_agent_events(
            &self,
            _: tonic::Request<pb::observer::GetAgentEventsRequest>,
        ) -> Result<tonic::Response<Self::GetAgentEventsStream>, tonic::Status> {
            Err(tonic::Status::unimplemented(""))
        }
        type GetDebugEventsStream = std::pin::Pin<
            Box<
                dyn tokio_stream::Stream<
                        Item = Result<pb::observer::GetDebugEventsResponse, tonic::Status>,
                    > + Send,
            >,
        >;
        async fn get_debug_events(
            &self,
            _: tonic::Request<pb::observer::GetDebugEventsRequest>,
        ) -> Result<tonic::Response<Self::GetDebugEventsStream>, tonic::Status> {
            Err(tonic::Status::unimplemented(""))
        }
        async fn get_nodes(
            &self,
            _: tonic::Request<pb::observer::GetNodesRequest>,
        ) -> Result<tonic::Response<pb::observer::GetNodesResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented(""))
        }
        async fn get_namespaces(
            &self,
            _: tonic::Request<pb::observer::GetNamespacesRequest>,
        ) -> Result<tonic::Response<pb::observer::GetNamespacesResponse>, tonic::Status> {
            Err(tonic::Status::unimplemented(""))
        }
    }

    pub async fn serve(flows: Vec<PbFlow>) -> (String, Arc<Mutex<Vec<GetFlowsRequest>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let svc = ObserverServer::new(Fake {
            flows,
            seen: seen.clone(),
        });
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(svc)
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .ok();
        });
        (addr.to_string(), seen)
    }
}

#[cfg(test)]
mod tests {
    use super::pb::flow::{Http, Layer4, Layer7, Udp};
    use super::testing::{serve, tcp_flow, ts};
    use super::*;

    #[test]
    fn converts_a_tcp_drop() {
        let f = flow_to_model(&tcp_flow("u-1", Verdict::Dropped, 5432));
        assert_eq!(f.id, "u-1");
        assert_eq!(f.verdict, "DROPPED");
        assert_eq!(f.protocol, "TCP");
        assert_eq!(f.port, 5432);
        assert_eq!(f.source.namespace, "shop");
        assert_eq!(f.source.pod, "web-1");
        assert_eq!(f.source.ip, "10.0.0.1");
        assert_eq!(f.destination.pod, "pg-0");
        assert_eq!(f.destination.ip, "10.0.0.2");
        assert_eq!(f.timestamp, "2023-11-14T22:13:20.123456Z");
    }

    #[test]
    fn udp_and_icmp_protocols() {
        let mut udp = tcp_flow("", Verdict::Forwarded, 0);
        udp.l4 = Some(Layer4 {
            protocol: Some(layer4::Protocol::Udp(Udp {
                source_port: 1,
                destination_port: 53,
            })),
        });
        let f = flow_to_model(&udp);
        assert_eq!((f.protocol.as_str(), f.port), ("UDP", 53));

        let mut icmp = tcp_flow("", Verdict::Forwarded, 0);
        icmp.l4 = Some(Layer4 {
            protocol: Some(layer4::Protocol::IcmPv4(Default::default())),
        });
        let f = flow_to_model(&icmp);
        assert_eq!((f.protocol.as_str(), f.port), ("ICMPv4", 0));

        let mut none = tcp_flow("", Verdict::Forwarded, 0);
        none.l4 = None;
        assert_eq!(flow_to_model(&none).protocol, "UNKNOWN");
    }

    #[test]
    fn out_of_range_port_and_unknown_verdict_do_not_panic() {
        let mut f = tcp_flow("u", Verdict::Forwarded, 70_000);
        f.verdict = 999;
        let m = flow_to_model(&f);
        assert_eq!(m.port, 0);
        assert_eq!(m.verdict, "UNKNOWN");
    }

    #[test]
    fn http_fields_come_from_l7() {
        let mut f = tcp_flow("u", Verdict::Forwarded, 80);
        f.l7 = Some(Layer7 {
            record: Some(layer7::Record::Http(Http {
                code: 503,
                method: "GET".into(),
                url: "http://api/x".into(),
                ..Default::default()
            })),
            ..Default::default()
        });
        let m = flow_to_model(&f);
        assert_eq!(m.http_method.as_deref(), Some("GET"));
        assert_eq!(m.http_url.as_deref(), Some("http://api/x"));
        assert_eq!(m.http_code, Some(503));
        assert!(flow_to_model(&tcp_flow("u", Verdict::Forwarded, 80))
            .http_method
            .is_none());
    }

    #[test]
    fn dns_fields_come_from_l7_not_from_drop_verdict() {
        use super::pb::flow::Dns;
        let mut f = tcp_flow("u", Verdict::Dropped, 53);
        f.l7 = Some(Layer7 {
            latency_ns: 2_500_000,
            record: Some(layer7::Record::Dns(Dns {
                query: "payments.shop.svc.cluster.local.".into(),
                ips: vec!["10.0.0.9".into()],
                rcode: 3,
                qtypes: vec!["A".into()],
                ..Default::default()
            })),
            ..Default::default()
        });
        let m = flow_to_model(&f);
        assert_eq!(m.dns_query.as_deref(), Some("payments.shop.svc.cluster.local."));
        assert_eq!(m.dns_rcode, Some(3));
        assert_eq!(m.dns_rcode_name.as_deref(), Some("NXDOMAIN"));
        assert_eq!(m.dns_ips.as_ref().map(|v| v.as_slice()), Some(&["10.0.0.9".to_string()][..]));
        assert_eq!(m.dns_latency_ns, Some(2_500_000));
        // A drop without L7 DNS must not invent an rcode.
        let bare = flow_to_model(&tcp_flow("d", Verdict::Dropped, 53));
        assert!(bare.dns_query.is_none());
        assert!(bare.dns_rcode.is_none());
    }

    #[test]
    fn id_is_stable_without_uuid_and_matches_the_json_path() {
        let a = flow_to_model(&tcp_flow("", Verdict::Dropped, 5432));
        let b = flow_to_model(&tcp_flow("", Verdict::Dropped, 5432));
        assert_eq!(a.id, b.id);
        assert!(a.id.starts_with("h-"));
        assert_ne!(
            a.id,
            flow_to_model(&tcp_flow("", Verdict::Dropped, 5433)).id
        );

        // The same flow read via the CLI's JSON must get the same id, so switching
        // backends does not duplicate rows in the history store.
        let json = serde_json::json!({"flow": {
            "time": "2023-11-14T22:13:20.123456Z",
            "verdict": "DROPPED",
            "IP": {"source": "10.0.0.1", "destination": "10.0.0.2"},
            "l4": {"TCP": {"source_port": 40000, "destination_port": 5432}},
            "source": {"namespace": "shop", "pod_name": "web-1"},
            "destination": {"namespace": "db", "pod_name": "pg-0"}
        }});
        let via_cli = crate::services::hubble::flow_from_hubble_line(0, &json).unwrap();
        assert_eq!(via_cli.id, a.id);
    }

    #[test]
    fn non_flow_messages_are_skipped_and_envelope_time_is_the_fallback() {
        let lost = GetFlowsResponse {
            response_types: Some(ResponseTypes::LostEvents(Default::default())),
            ..Default::default()
        };
        assert!(flow_from_response(&lost).is_none());
        assert!(flow_from_response(&GetFlowsResponse::default()).is_none());

        let mut inner = tcp_flow("u", Verdict::Forwarded, 80);
        inner.time = None;
        let msg = GetFlowsResponse {
            time: ts(1_700_000_000, 0),
            response_types: Some(ResponseTypes::Flow(inner)),
            ..Default::default()
        };
        assert_eq!(
            flow_from_response(&msg).unwrap().timestamp,
            "2023-11-14T22:13:20Z"
        );
    }

    #[test]
    fn namespace_filter_matches_either_end() {
        assert!(namespace_filters(None).is_empty());
        let f = namespace_filters(Some("shop"));
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].source_pod, vec!["shop/"]);
        assert!(f[0].destination_pod.is_empty());
        assert_eq!(f[1].destination_pod, vec!["shop/"]);
        assert!(f[1].source_pod.is_empty());
    }

    #[tokio::test]
    async fn last_flows_requests_n_without_follow_and_skips_non_flows() {
        let flows = vec![
            tcp_flow("a", Verdict::Forwarded, 80),
            tcp_flow("b", Verdict::Dropped, 81),
            tcp_flow("c", Verdict::Forwarded, 82),
        ];
        let (addr, seen) = serve(flows).await;
        let got = last_flows(&addr, 2, None, Some("east")).await.unwrap();
        assert_eq!(
            got.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert!(got.iter().all(|f| f.cluster.as_deref() == Some("east")));
        let req = seen.lock().unwrap()[0].clone();
        assert_eq!(req.number, 2);
        assert!(!req.follow);
        assert!(req.whitelist.is_empty());
    }

    #[tokio::test]
    async fn last_flows_sends_the_namespace_filter() {
        let (addr, seen) = serve(vec![tcp_flow("a", Verdict::Forwarded, 80)]).await;
        last_flows(&addr, 10, Some("shop"), None).await.unwrap();
        let req = seen.lock().unwrap()[0].clone();
        assert_eq!(req.whitelist.len(), 2);
        assert_eq!(req.whitelist[0].source_pod, vec!["shop/"]);
        assert_eq!(req.whitelist[1].destination_pod, vec!["shop/"]);
    }

    #[tokio::test]
    async fn follow_requests_a_live_stream() {
        let (addr, seen) = serve(vec![tcp_flow("a", Verdict::Forwarded, 80)]).await;
        let mut stream = follow_flows(&addr, Some("shop")).await.unwrap();
        let mut n = 0;
        while let Some(msg) = stream.message().await.unwrap() {
            if flow_from_response(&msg).is_some() {
                n += 1;
            }
        }
        assert_eq!(n, 1);
        let req = seen.lock().unwrap()[0].clone();
        assert!(req.follow);
        assert_eq!(req.number, 0);
    }

    #[tokio::test]
    async fn health_reflects_server_status() {
        let (addr, _) = serve(vec![]).await;
        assert!(is_healthy(&addr).await);

        // Nothing listening: bind then drop to get a free, closed port.
        let closed = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().to_string()
        };
        assert!(!is_healthy(&closed).await);
        assert!(last_flows(&closed, 5, None, None).await.is_err());
    }

    #[tokio::test]
    async fn a_plain_tcp_listener_is_not_a_healthy_hubble() {
        // The old TCP-connect check would call this healthy.
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            loop {
                if let Ok((mut s, _)) = l.accept().await {
                    use tokio::io::AsyncWriteExt;
                    let _ = s.write_all(b"not grpc").await;
                }
            }
        });
        assert!(!is_healthy(&addr).await);
    }

    /// Serves a fixed set of flows on `FAKE_HUBBLE_ADDR` until killed, for running the
    /// real API binary against a Hubble without a cluster:
    /// `FAKE_HUBBLE_ADDR=127.0.0.1:4245 cargo test fake_observer_server -- --ignored`
    #[tokio::test]
    #[ignore = "long-running helper, not a test"]
    async fn fake_observer_server() {
        let addr = std::env::var("FAKE_HUBBLE_ADDR").expect("set FAKE_HUBBLE_ADDR");
        let mut drop = tcp_flow("e2e-drop", Verdict::Dropped, 5432);
        drop.uuid = String::new(); // exercises the content-derived id
        let flows = vec![
            tcp_flow("e2e-1", Verdict::Forwarded, 80),
            drop,
            tcp_flow("e2e-3", Verdict::Forwarded, 443),
        ];
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        let svc = pb::observer::observer_server::ObserverServer::new(testing::Fake {
            flows,
            seen: Default::default(),
        });
        tonic::transport::Server::builder()
            .add_service(svc)
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .unwrap();
    }
}
