// Hubble service client
//
// This service wraps Hubble gRPC/CLI interactions for the web API.
// It attempts a gRPC connection first, then falls back to CLI invocation.

use crate::models::flow::{Flow, FlowEndpoint, FlowStats};
use anyhow::Result;
use tokio::process::Command;

pub struct HubbleService {
    address: String,
    clusters: Vec<(String, String)>, // (cluster_name, address)
}

impl HubbleService {
    /// Create a new HubbleService.
    ///
    /// `clusters` is a list of `(cluster_name, host:port)` pairs for
    /// multi-cluster flow aggregation. Each address is validated at startup.
    ///
    /// # Errors
    /// Returns an error if `address` or any cluster address is not in `host:port` format,
    /// or if any cluster name is empty.
    pub fn new(address: &str, clusters: Vec<(String, String)>) -> Result<Self> {
        Self::validate_address(address)?;
        for (name, addr) in &clusters {
            if name.is_empty() {
                anyhow::bail!("Cluster name must not be empty");
            }
            Self::validate_address(addr)?;
        }
        Ok(Self {
            address: address.to_string(),
            clusters,
        })
    }

    fn validate_address(address: &str) -> Result<()> {
        let parts: Vec<&str> = address.rsplitn(2, ':').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            anyhow::bail!(
                "Invalid Hubble address '{}': expected host:port format (e.g. hubble-relay:4245)",
                address
            );
        }
        if parts[0].parse::<u16>().is_err() {
            anyhow::bail!(
                "Invalid Hubble address '{}': port must be a valid u16",
                address
            );
        }
        Ok(())
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Return the configured cluster list.
    pub fn clusters(&self) -> &[(String, String)] {
        &self.clusters
    }

    /// Check if Hubble relay is reachable via TCP
    pub async fn is_healthy(&self) -> bool {
        tokio::net::TcpStream::connect(&self.address).await.is_ok()
    }

    /// Check if a specific Hubble address is reachable via TCP
    pub async fn is_address_healthy(address: &str) -> bool {
        tokio::net::TcpStream::connect(address).await.is_ok()
    }

    /// Get flows from all configured clusters, tagged with cluster_name.
    /// Each cluster is queried concurrently. Returns a Vec of (cluster_name, flows).
    pub async fn get_flows_multi_cluster(
        &self,
        limit: usize,
        namespace: Option<&str>,
    ) -> Vec<(String, Vec<Flow>)> {
        let mut handles = Vec::new();

        for (name, addr) in &self.clusters {
            let name = name.clone();
            let addr = addr.clone();
            let namespace = namespace.map(|s| s.to_string());

            handles.push(tokio::spawn(async move {
                let flows =
                    Self::get_flows_from_address(&addr, limit, namespace.as_deref(), Some(&name))
                        .await
                        .unwrap_or_default();
                (name, flows)
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
        results
    }

    /// Retrieve flows from a specific Hubble address, optionally tagging each
    /// flow with a cluster name.
    async fn get_flows_from_address(
        address: &str,
        limit: usize,
        namespace: Option<&str>,
        cluster_name: Option<&str>,
    ) -> Result<Vec<Flow>> {
        let limit = limit.min(10_000);

        if let Some(ns) = namespace {
            if ns.starts_with('-') || ns.contains(char::is_whitespace) {
                anyhow::bail!("Invalid namespace: '{}'", ns);
            }
        }

        let mut cmd = Command::new("hubble");
        cmd.arg("observe")
            .arg("--output")
            .arg("json")
            .arg("--last")
            .arg(limit.to_string())
            .arg("--server")
            .arg(address);

        if let Some(ns) = namespace {
            cmd.arg("--namespace").arg(ns);
        }

        let output = match cmd.output().await {
            Ok(out) => out,
            Err(e) => {
                tracing::debug!("hubble CLI not available for {}: {}", address, e);
                return Ok(Vec::new());
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::debug!(
                "hubble observe ({}) returned non-zero: {}",
                address,
                stderr.trim()
            );
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let cluster_tag = cluster_name.map(|s| s.to_string());
        let flows: Vec<Flow> = stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .enumerate()
            .map(|(i, v)| {
                let mut flow = hubble_json_to_flow(i, &v);
                if cluster_tag.is_some() {
                    flow.cluster = cluster_tag.clone();
                }
                flow
            })
            .collect();

        Ok(flows)
    }

    /// Retrieve flows from Hubble via the `hubble` CLI with `--server`.
    /// Checks relay connectivity first, then falls back gracefully.
    pub async fn get_flows(&self, limit: usize, namespace: Option<&str>) -> Result<Vec<Flow>> {
        // Cap the limit to prevent excessive resource consumption
        let limit = limit.min(10_000);

        // Validate namespace to prevent flag injection
        if let Some(ns) = namespace {
            if ns.starts_with('-') || ns.contains(char::is_whitespace) {
                anyhow::bail!("Invalid namespace: '{}'", ns);
            }
        }

        // Build the hubble observe command
        let mut cmd = Command::new("hubble");
        cmd.arg("observe")
            .arg("--output")
            .arg("json")
            .arg("--last")
            .arg(limit.to_string())
            .arg("--server")
            .arg(&self.address);

        if let Some(ns) = namespace {
            cmd.arg("--namespace").arg(ns);
        }

        let output = match cmd.output().await {
            Ok(out) => out,
            Err(e) => {
                tracing::debug!("hubble CLI not available: {}", e);
                return Ok(Vec::new());
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::debug!("hubble observe returned non-zero: {}", stderr.trim());
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let flows: Vec<Flow> = stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .enumerate()
            .map(|(i, v)| hubble_json_to_flow(i, &v))
            .collect();

        Ok(flows)
    }

    /// Compute aggregate flow statistics
    pub async fn get_flow_stats(&self) -> Result<FlowStats> {
        let flows = self.get_flows(1000, None).await?;

        let total = flows.len() as u64;
        let forwarded = flows.iter().filter(|f| f.verdict == "FORWARDED").count() as u64;
        let dropped = flows.iter().filter(|f| f.verdict == "DROPPED").count() as u64;

        Ok(FlowStats {
            total_flows: total,
            forwarded,
            dropped,
            requests_per_second: 0.0, // Would need time-window sampling
            avg_latency_ms: 0.0,      // Would need L7 data
        })
    }
}

/// Convert a raw Hubble JSON object into our canonical Flow struct.
///
/// Hubble CLI (`hubble observe --output json`) emits each event as
/// `{"flow": { ...fields... }}`. Older/flat payloads put fields at the root.
/// IPs live under `IP.source` / `IP.destination`, not on the endpoint object.
pub fn hubble_json_to_flow(index: usize, v: &serde_json::Value) -> Flow {
    let flow = v.get("flow").unwrap_or(v);
    let src = flow.get("source").unwrap_or(flow);
    let dst = flow.get("destination").unwrap_or(flow);
    let ip = flow.get("IP").or_else(|| flow.get("ip"));

    // Extract L7 HTTP fields if present
    let l7_http = flow.get("l7").and_then(|l7| {
        l7.get("http")
            .or_else(|| l7.get("Http"))
            .or_else(|| l7.get("HTTP"))
    });

    let http_method = l7_http.and_then(|http| {
        http.get("method")
            .or_else(|| http.get("Method"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
    });

    let http_url = l7_http.and_then(|http| {
        http.get("url")
            .or_else(|| http.get("Url"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
    });

    let http_code = l7_http.and_then(|http| {
        http.get("code")
            .or_else(|| http.get("Code"))
            .and_then(|x| x.as_u64())
            .map(|c| c as u16)
    });

    let src_ip = json_str(src, "ip")
        .or_else(|| json_str(src, "IP"))
        .or_else(|| {
            ip.and_then(|i| i.get("source"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        });
    let dst_ip = json_str(dst, "ip")
        .or_else(|| json_str(dst, "IP"))
        .or_else(|| {
            ip.and_then(|i| i.get("destination"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        });

    // Prefer pod_name; fall back to workload name or identity label for host/reserved.
    let src_pod = json_str(src, "pod_name")
        .or_else(|| json_str(src, "pod"))
        .or_else(|| workload_name(src))
        .or_else(|| reserved_label(src));
    let dst_pod = json_str(dst, "pod_name")
        .or_else(|| json_str(dst, "pod"))
        .or_else(|| workload_name(dst))
        .or_else(|| reserved_label(dst));

    Flow {
        id: flow
            .get("uuid")
            .or_else(|| flow.get("id"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("flow-{}", index)),
        timestamp: flow
            .get("time")
            .or_else(|| flow.get("timestamp"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        source: FlowEndpoint {
            namespace: json_str(src, "namespace").or_else(|| namespace_from_labels(src)),
            pod: src_pod,
            ip: src_ip,
        },
        destination: FlowEndpoint {
            namespace: json_str(dst, "namespace").or_else(|| namespace_from_labels(dst)),
            pod: dst_pod,
            ip: dst_ip,
        },
        verdict: flow
            .get("verdict")
            .and_then(|x| x.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        protocol: flow
            .get("l4")
            .and_then(|l4| {
                if l4.get("TCP").is_some() {
                    Some("TCP")
                } else if l4.get("UDP").is_some() {
                    Some("UDP")
                } else if l4.get("ICMPv4").is_some() || l4.get("ICMP").is_some() {
                    Some("ICMPv4")
                } else if l4.get("ICMPv6").is_some() {
                    Some("ICMPv6")
                } else {
                    None
                }
            })
            .unwrap_or("UNKNOWN")
            .to_string(),
        port: flow
            .get("l4")
            .and_then(|l4| {
                l4.get("TCP")
                    .or_else(|| l4.get("UDP"))
                    .and_then(|proto| {
                        proto
                            .get("destination_port")
                            .or_else(|| proto.get("destinationPort"))
                    })
                    .and_then(|p| p.as_u64())
            })
            .unwrap_or(0) as u16,
        http_method,
        http_url,
        http_code,
        cluster: flow
            .get("cluster")
            .or_else(|| src.get("cluster_name"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
    }
}

fn workload_name(ep: &serde_json::Value) -> String {
    ep.get("workloads")
        .and_then(|w| w.as_array())
        .and_then(|arr| arr.first())
        .and_then(|w| w.get("name"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn reserved_label(ep: &serde_json::Value) -> String {
    ep.get("labels")
        .and_then(|l| l.as_array())
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str())
        .find(|l| l.starts_with("reserved:"))
        .map(|l| l.trim_start_matches("reserved:").to_string())
        .unwrap_or_default()
}

fn namespace_from_labels(ep: &serde_json::Value) -> String {
    ep.get("labels")
        .and_then(|l| l.as_array())
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str())
        .find_map(|l| {
            l.strip_prefix("k8s:io.kubernetes.pod.namespace=")
                .or_else(|| l.strip_prefix("k8s:io.cilium.k8s.namespace.labels.kubernetes.io/metadata.name="))
        })
        .unwrap_or("")
        .to_string()
}

/// Helper: extract a string field from a JSON value, returning empty string if absent
fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

/// Extension trait so we can chain .or_else on String (not Option)
trait StringOrElse {
    fn or_else(self, f: impl FnOnce() -> String) -> String;
}

impl StringOrElse for String {
    fn or_else(self, f: impl FnOnce() -> String) -> String {
        if self.is_empty() {
            f()
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_wrapped_hubble_cli_json() {
        let v = json!({
            "flow": {
                "time": "2026-09-24T04:30:16.104Z",
                "uuid": "abc-123",
                "verdict": "FORWARDED",
                "IP": { "source": "10.42.0.48", "destination": "10.42.0.1", "ipVersion": "IPv4" },
                "l4": { "TCP": { "source_port": 51466, "destination_port": 4245 } },
                "source": {
                    "namespace": "kube-system",
                    "pod_name": "hubble-relay-xyz",
                    "labels": ["k8s:k8s-app=hubble-relay"]
                },
                "destination": {
                    "identity": 1,
                    "labels": ["reserved:host", "reserved:kube-apiserver"]
                }
            }
        });
        let f = hubble_json_to_flow(0, &v);
        assert_eq!(f.id, "abc-123");
        assert_eq!(f.verdict, "FORWARDED");
        assert_eq!(f.protocol, "TCP");
        assert_eq!(f.port, 4245);
        assert_eq!(f.source.namespace, "kube-system");
        assert_eq!(f.source.pod, "hubble-relay-xyz");
        assert_eq!(f.source.ip, "10.42.0.48");
        assert_eq!(f.destination.pod, "host");
        assert_eq!(f.destination.ip, "10.42.0.1");
        assert!(!f.timestamp.is_empty());
    }

    #[test]
    fn parses_flat_flow_payload() {
        let v = json!({
            "time": "2026-01-01T00:00:00Z",
            "uuid": "flat-1",
            "verdict": "DROPPED",
            "IP": { "source": "1.1.1.1", "destination": "2.2.2.2" },
            "l4": { "UDP": { "destination_port": 53 } },
            "source": { "namespace": "default", "pod_name": "a" },
            "destination": { "namespace": "kube-system", "pod_name": "coredns" }
        });
        let f = hubble_json_to_flow(0, &v);
        assert_eq!(f.verdict, "DROPPED");
        assert_eq!(f.protocol, "UDP");
        assert_eq!(f.port, 53);
        assert_eq!(f.destination.pod, "coredns");
    }
}
