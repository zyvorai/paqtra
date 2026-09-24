//! Path investigation and evidence-backed policy preview.
//!
//! Every claim carries confidence: observed | inferred | unavailable.
//! Unsupported policy constructs are reported as unknown — never a confident pass.

use crate::services::change_tracker;
use crate::services::flow_store::{FlowQuery, StoredFlow};
use crate::AppState;
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Observed,
    Inferred,
    Unavailable,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkloadRef {
    pub namespace: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pod: String,
    #[serde(default)]
    pub labels: Option<Value>,
}

impl WorkloadRef {
    pub fn pod_hint(&self) -> &str {
        if !self.pod.is_empty() {
            &self.pod
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InvestigatePathRequest {
    pub source: WorkloadRef,
    pub destination: WorkloadRef,
    pub port: u16,
    #[serde(default = "default_proto")]
    pub protocol: String,
    /// Minutes to look back (default 60).
    #[serde(default = "default_window")]
    pub time_window_minutes: i64,
}

fn default_proto() -> String {
    "TCP".into()
}
fn default_window() -> i64 {
    60
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceRef {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvestigateStep {
    pub id: String,
    pub title: String,
    pub detail: String,
    pub confidence: Confidence,
    pub evidence: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvestigateResult {
    pub id: String,
    pub request: Value,
    pub steps: Vec<InvestigateStep>,
    pub likely_owner: String,
    pub next_actions: Vec<String>,
    pub flow_ingest: Value,
    pub created_at: String,
}

pub async fn investigate_path(state: &AppState, req: &InvestigatePathRequest) -> InvestigateResult {
    let id = format!("inv-{}", Uuid::new_v4());
    let since = (Utc::now() - ChronoDuration::minutes(req.time_window_minutes.max(1))).to_rfc3339();
    let mut steps = Vec::new();
    let stats = state.flow_store.stats();

    // 1. Resolve workloads
    let src_hint = req.source.pod_hint().to_string();
    let dst_hint = req.destination.pod_hint().to_string();
    let k8s_ok = state.k8s.is_healthy().await;

    let (src_pods, dst_eps) = if k8s_ok {
        let src = state
            .k8s
            .kubectl_json(&["get", "pods", "-n", &req.source.namespace, "-o", "json"])
            .await;
        let svc = state
            .k8s
            .kubectl_json(&["get", "svc", "-n", &req.destination.namespace, "-o", "json"])
            .await;
        let eps = state
            .k8s
            .kubectl_json(&[
                "get",
                "endpoints",
                "-n",
                &req.destination.namespace,
                "-o",
                "json",
            ])
            .await;
        (src, (svc, eps))
    } else {
        (json!({}), (json!({}), json!({})))
    };

    let matching_src = count_name_matches(&src_pods, &src_hint);
    let (svc_json, eps_json) = dst_eps;
    let matching_svc = count_name_matches(&svc_json, &dst_hint);
    let backend_count = endpoint_address_count(&eps_json, &dst_hint);

    steps.push(InvestigateStep {
        id: "resolve".into(),
        title: "Resolve workloads".into(),
        detail: if k8s_ok {
            format!(
                "Source '{}': {} pod match(es). Destination '{}': {} Service match(es), {} endpoint address(es).",
                src_hint, matching_src, dst_hint, matching_svc, backend_count
            )
        } else {
            "Kubernetes API unavailable — workload resolution skipped.".into()
        },
        confidence: if k8s_ok {
            Confidence::Observed
        } else {
            Confidence::Unavailable
        },
        evidence: if k8s_ok {
            vec![EvidenceRef {
                kind: "k8s".into(),
                id: format!("{}/{}", req.source.namespace, src_hint),
            }]
        } else {
            vec![]
        },
    });

    // 2. Identities (best-effort from flows / unavailable)
    let path_flows = state
        .flow_store
        .query(&FlowQuery {
            src_namespace: Some(req.source.namespace.clone()),
            src_pod: Some(src_hint.clone()),
            dst_namespace: Some(req.destination.namespace.clone()),
            dst_pod: Some(dst_hint.clone()),
            port: Some(req.port),
            since_rfc3339: Some(since.clone()),
            limit: 100,
            ..Default::default()
        })
        .unwrap_or_default();

    let any_identity = path_flows
        .iter()
        .any(|f| f.src_identity > 0 || f.dst_identity > 0);
    steps.push(InvestigateStep {
        id: "identity".into(),
        title: "Cilium identities".into(),
        detail: if any_identity {
            format!(
                "Saw identity fields on {} stored flow(s) for this path.",
                path_flows.len()
            )
        } else if !path_flows.is_empty() {
            format!(
                "{} flow(s) indexed for this path; identity IDs not present in ingested metadata.",
                path_flows.len()
            )
        } else {
            "No indexed flows with identities for this path in the window.".into()
        },
        confidence: if any_identity {
            Confidence::Observed
        } else if !path_flows.is_empty() {
            Confidence::Inferred
        } else {
            Confidence::Unavailable
        },
        evidence: path_flows
            .iter()
            .take(5)
            .map(|f| EvidenceRef {
                kind: "flow".into(),
                id: f.id.clone(),
            })
            .collect(),
    });

    // 3. Policy / verdict from flows
    let dropped: Vec<_> = path_flows
        .iter()
        .filter(|f| f.verdict.eq_ignore_ascii_case("DROPPED"))
        .cloned()
        .collect();
    let forwarded: Vec<_> = path_flows
        .iter()
        .filter(|f| f.verdict.eq_ignore_ascii_case("FORWARDED"))
        .cloned()
        .collect();

    let policies = if k8s_ok {
        state.k8s.list_policies().await.unwrap_or_default()
    } else {
        Vec::new()
    };
    let ns_policies: Vec<_> = policies
        .iter()
        .filter(|p| p.namespace == req.source.namespace || p.namespace == req.destination.namespace)
        .collect();

    let denying_guess = ns_policies
        .iter()
        .find(|p| {
            let n = p.name.to_lowercase();
            n.contains("deny") || n.contains("default-deny")
        })
        .map(|p| p.name.clone());

    steps.push(InvestigateStep {
        id: "policy".into(),
        title: "Policy verdict".into(),
        detail: if !dropped.is_empty() {
            format!(
                "{} dropped / {} forwarded flow(s) in window. {} CNP(s) in related namespaces.{}",
                dropped.len(),
                forwarded.len(),
                ns_policies.len(),
                denying_guess
                    .as_ref()
                    .map(|n| format!(" Candidate deny policy name: {n}."))
                    .unwrap_or_default()
            )
        } else if !forwarded.is_empty() {
            format!(
                "{} forwarded flow(s); no drops for this path in the window.",
                forwarded.len()
            )
        } else {
            format!(
                "No path flows in store. {} CNP(s) visible in related namespaces.",
                ns_policies.len()
            )
        },
        confidence: if !path_flows.is_empty() {
            Confidence::Observed
        } else if k8s_ok {
            Confidence::Inferred
        } else {
            Confidence::Unavailable
        },
        evidence: {
            let mut e: Vec<_> = dropped
                .iter()
                .chain(forwarded.iter())
                .take(8)
                .map(|f| EvidenceRef {
                    kind: "flow".into(),
                    id: f.id.clone(),
                })
                .collect();
            if let Some(n) = &denying_guess {
                e.push(EvidenceRef {
                    kind: "policy".into(),
                    id: n.clone(),
                });
            }
            e
        },
    });

    // 4. DNS
    let dns_flows = state
        .flow_store
        .query(&FlowQuery {
            src_namespace: Some(req.source.namespace.clone()),
            port: Some(53),
            since_rfc3339: Some(since.clone()),
            limit: 50,
            ..Default::default()
        })
        .unwrap_or_default();
    let dns_drops = dns_flows
        .iter()
        .filter(|f| f.verdict.eq_ignore_ascii_case("DROPPED"))
        .count();
    steps.push(InvestigateStep {
        id: "dns".into(),
        title: "DNS signals".into(),
        detail: if req.port == 53 || dns_drops > 0 {
            format!(
                "{} DNS (port 53) flow(s) from source namespace; {} dropped.",
                dns_flows.len(),
                dns_drops
            )
        } else if dns_flows.is_empty() {
            "No DNS flows indexed for source namespace in window.".into()
        } else {
            format!(
                "{} DNS flow(s) observed from source namespace (port 53).",
                dns_flows.len()
            )
        },
        confidence: if !dns_flows.is_empty() {
            Confidence::Observed
        } else {
            Confidence::Unavailable
        },
        evidence: dns_flows
            .iter()
            .take(3)
            .map(|f| EvidenceRef {
                kind: "flow".into(),
                id: f.id.clone(),
            })
            .collect(),
    });

    // 5. Node path
    let cross_node = path_flows.iter().any(|f| {
        !f.src_ip.is_empty()
            && !f.dst_ip.is_empty()
            && f.src_ip.split('.').take(3).collect::<Vec<_>>()
                != f.dst_ip.split('.').take(3).collect::<Vec<_>>()
    });
    steps.push(InvestigateStep {
        id: "path".into(),
        title: "Node path".into(),
        detail: if path_flows.is_empty() {
            "Insufficient flow evidence to infer same-node vs cross-node.".into()
        } else if cross_node {
            "IP prefixes differ across observed flows — likely cross-node (inferred).".into()
        } else {
            "Observed flows share address prefixes — possibly same-node (inferred).".into()
        },
        confidence: if path_flows.is_empty() {
            Confidence::Unavailable
        } else {
            Confidence::Inferred
        },
        evidence: path_flows
            .iter()
            .take(2)
            .map(|f| EvidenceRef {
                kind: "flow".into(),
                id: f.id.clone(),
            })
            .collect(),
    });

    // 6. Correlated changes (thin timeline join)
    let changes = change_tracker::recent_changes(state, 20).await;
    steps.push(InvestigateStep {
        id: "changes".into(),
        title: "Correlated changes".into(),
        detail: if changes.is_empty() {
            "No recent network-related change events in tracker cache.".into()
        } else {
            format!(
                "{} recent change event(s); top: {}",
                changes.len(),
                changes
                    .iter()
                    .take(3)
                    .filter_map(|c| c.get("resource").and_then(|n| n.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        confidence: if changes.is_empty() {
            Confidence::Unavailable
        } else {
            Confidence::Observed
        },
        evidence: changes
            .iter()
            .take(5)
            .filter_map(|c| {
                Some(EvidenceRef {
                    kind: "change".into(),
                    id: c.get("id")?.as_str()?.to_string(),
                })
            })
            .collect(),
    });

    // Owner
    let likely_owner = if dns_drops > 0 && (req.port == 53 || dropped.is_empty()) {
        "dns"
    } else if k8s_ok && matching_svc > 0 && backend_count == 0 {
        "no_backend"
    } else if !dropped.is_empty() {
        "policy"
    } else {
        // Forwarded flows, no k8s data, or nothing observed: no root cause to name.
        "unknown"
    };

    let mut next_actions = Vec::new();
    match likely_owner {
        "policy" => {
            next_actions.push(
                "Preview a narrowly scoped CiliumNetworkPolicy allow for this port (POST /api/v1/policies/simulate)."
                    .into(),
            );
            if let Some(n) = denying_guess {
                next_actions.push(format!(
                    "Review CNP '{n}' selectors against source/destination identities."
                ));
            }
        }
        "dns" => {
            next_actions
                .push("Allow DNS egress (UDP/TCP 53) to kube-dns / node-local-dns via CNP.".into());
        }
        "no_backend" => {
            next_actions.push(
                "Check Deployment/EndpointSlice readiness for the destination Service.".into(),
            );
        }
        _ => {
            next_actions.push(
                "Widen the time window or confirm Hubble ingest is healthy, then re-run.".into(),
            );
        }
    }
    next_actions.push(
        "Enforcement stays in Cilium CRDs — Paqtra will not attach or rewrite BPF programs.".into(),
    );

    // Persist bundle snapshot in cache
    let result = InvestigateResult {
        id: id.clone(),
        request: json!({
            "source": { "namespace": req.source.namespace, "name": src_hint },
            "destination": { "namespace": req.destination.namespace, "name": dst_hint },
            "port": req.port,
            "protocol": req.protocol,
            "time_window_minutes": req.time_window_minutes,
        }),
        steps: steps.clone(),
        likely_owner: likely_owner.into(),
        next_actions: next_actions.clone(),
        flow_ingest: json!({
            "total": stats.total,
            "last_ok": stats.last_ingest_ok,
            "last_at": stats.last_ingest_at,
            "source": stats.ingest_source,
            "confidence": if stats.last_ingest_ok { "observed" } else { "unavailable" },
        }),
        created_at: Utc::now().to_rfc3339(),
    };

    let bundle = build_bundle(&result, &path_flows, &dropped, &ns_policies);
    let _ = state
        .cache
        .set_persistent(&format!("cv:investigate_bundle:{}", id), &bundle)
        .await;

    result
}

fn count_name_matches(list: &Value, hint: &str) -> usize {
    let hint = hint.to_lowercase();
    list.get("items")
        .and_then(|i| i.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|it| {
                    let name = it
                        .pointer("/metadata/name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    !hint.is_empty() && (name.contains(&hint) || hint.contains(&name))
                })
                .count()
        })
        .unwrap_or(0)
}

fn endpoint_address_count(eps: &Value, hint: &str) -> usize {
    let hint = hint.to_lowercase();
    eps.get("items")
        .and_then(|i| i.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|it| {
                    let name = it
                        .pointer("/metadata/name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_lowercase();
                    hint.is_empty() || name.contains(&hint) || hint.contains(&name)
                })
                .map(|it| {
                    it.pointer("/subsets")
                        .and_then(|s| s.as_array())
                        .map(|subs| {
                            subs.iter()
                                .map(|s| {
                                    s.get("addresses")
                                        .and_then(|a| a.as_array())
                                        .map(|a| a.len())
                                        .unwrap_or(0)
                                })
                                .sum::<usize>()
                        })
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0)
}

fn build_bundle(
    result: &InvestigateResult,
    path_flows: &[StoredFlow],
    dropped: &[StoredFlow],
    policies: &[&crate::models::policy::Policy],
) -> Value {
    json!({
        "id": result.id,
        "created_at": result.created_at,
        "likely_owner": result.likely_owner,
        "request": result.request,
        "steps": result.steps,
        "next_actions": result.next_actions,
        "flows": path_flows.iter().take(50).map(|f| json!({
            "id": f.id,
            "ts": f.ts,
            "verdict": f.verdict,
            "port": f.port,
            "src": format!("{}/{}", f.src_namespace, f.src_pod),
            "dst": format!("{}/{}", f.dst_namespace, f.dst_pod),
            "source": f.source.as_str(),
        })).collect::<Vec<_>>(),
        "dropped_sample": dropped.iter().take(20).map(|f| f.id.clone()).collect::<Vec<_>>(),
        "policies": policies.iter().take(20).map(|p| json!({
            "name": p.name,
            "namespace": p.namespace,
        })).collect::<Vec<_>>(),
        "redaction": "No payloads, argv, or Secret contents included.",
    })
}

// ---------------------------------------------------------------------------
// Policy preview (replaces heuristic simulate)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PolicyPreview {
    pub policy: String,
    pub namespace: String,
    pub analysis: Value,
    pub impact: Value,
    pub pairs: Vec<Value>,
    pub uncertainty: Vec<String>,
    pub rollback: Value,
    pub confidence: Confidence,
}

pub async fn preview_policy(
    state: &AppState,
    name: &str,
    namespace: &str,
    spec: &Value,
) -> PolicyPreview {
    let mut uncertainty = Vec::new();
    let unsupported = detect_unsupported(spec);
    uncertainty.extend(unsupported.clone());

    let ingress_rules = spec
        .get("ingress")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let egress_rules = spec
        .get("egress")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let selector_labels = endpoint_selector_labels(spec);
    let ports = collect_ports(spec);

    let k8s_ok = state.k8s.is_healthy().await;
    let mut matched_endpoints = 0usize;
    if k8s_ok && !selector_labels.is_empty() {
        let pods = state
            .k8s
            .kubectl_json(&["get", "pods", "-n", namespace, "-o", "json"])
            .await;
        matched_endpoints = pods
            .get("items")
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|p| {
                        let labels = p.pointer("/metadata/labels").cloned().unwrap_or(json!({}));
                        selector_labels.iter().all(|(k, v)| {
                            labels.get(k).and_then(|x| x.as_str()) == Some(v.as_str())
                        })
                    })
                    .count()
            })
            .unwrap_or(0);
    } else if selector_labels.is_empty() {
        uncertainty.push(
            "Empty or missing endpointSelector — namespace-wide scope; endpoint match unknown."
                .into(),
        );
    } else if !k8s_ok {
        uncertainty.push("Kubernetes unavailable — cannot resolve selectors to endpoints.".into());
    }

    let since = (Utc::now() - ChronoDuration::hours(24)).to_rfc3339();
    let recent = state
        .flow_store
        .query(&FlowQuery {
            src_namespace: Some(namespace.to_string()),
            since_rfc3339: Some(since),
            limit: 500,
            ..Default::default()
        })
        .unwrap_or_default();

    let mut pairs = Vec::new();
    let mut would_allow = 0u64;
    let mut would_deny = 0u64;
    let mut unknown_n = 0u64;

    if !unsupported.is_empty() {
        unknown_n = recent.len() as u64;
        uncertainty.push(
            "Unsupported constructs present — matching flows marked unknown, not a confident pass."
                .into(),
        );
        for f in recent.iter().take(30) {
            pairs.push(json!({
                "src": format!("{}/{}", f.src_namespace, f.src_pod),
                "dst": format!("{}/{}", f.dst_namespace, f.dst_pod),
                "port": f.port,
                "outcome": "unknown",
                "confidence": "unavailable",
                "flow_id": f.id,
            }));
        }
    } else {
        for f in &recent {
            let port_ok = ports.is_empty() || ports.contains(&f.port);
            let in_ns = f.src_namespace == namespace || f.dst_namespace == namespace;
            if !in_ns {
                continue;
            }
            // L3/L4 allow-list style inference only.
            let outcome = if port_ok && (ingress_rules + egress_rules) > 0 {
                would_allow += 1;
                "would_allow"
            } else if (ingress_rules + egress_rules) == 0 {
                would_deny += 1;
                "would_deny"
            } else {
                unknown_n += 1;
                "unknown"
            };
            if pairs.len() < 40 {
                pairs.push(json!({
                    "src": format!("{}/{}", f.src_namespace, f.src_pod),
                    "dst": format!("{}/{}", f.dst_namespace, f.dst_pod),
                    "port": f.port,
                    "prior_verdict": f.verdict,
                    "outcome": outcome,
                    "confidence": if outcome == "unknown" { "unavailable" } else { "inferred" },
                    "flow_id": f.id,
                }));
            }
        }
        if recent.is_empty() {
            uncertainty.push(
                "No indexed flows in namespace for the last 24h — impact pairs unavailable.".into(),
            );
        }
    }

    let confidence = if !unsupported.is_empty() {
        Confidence::Unavailable
    } else if k8s_ok {
        Confidence::Inferred
    } else {
        Confidence::Unavailable
    };

    PolicyPreview {
        policy: name.to_string(),
        namespace: namespace.to_string(),
        analysis: json!({
            "ingress_rules": ingress_rules,
            "egress_rules": egress_rules,
            "total_rules": ingress_rules + egress_rules,
            "endpoint_selector_labels": selector_labels,
            "ports": ports,
            "matched_endpoints": matched_endpoints,
            "unsupported_constructs": unsupported,
            "flows_considered": recent.len(),
        }),
        impact: json!({
            "would_allow": would_allow,
            "would_deny": would_deny,
            "unknown": unknown_n,
            "matched_endpoints": matched_endpoints,
            "risk_level": if !unsupported.is_empty() {
                "unknown"
            } else if selector_labels.is_empty() && ports.is_empty() {
                "high"
            } else if ports.is_empty() {
                "medium"
            } else {
                "low"
            },
            // Back-compat fields (explicitly inferred, not observed counts)
            "estimated_flows_affected": would_allow + would_deny + unknown_n,
            "services_impacted": matched_endpoints.max(1),
            "confidence": confidence,
        }),
        pairs,
        uncertainty,
        rollback: json!({
            "method": "cilium_crd",
            "steps": [
                format!("kubectl -n {namespace} delete ciliumnetworkpolicy {name}"),
                "Re-apply previous CNP revision from GitOps or backup if required",
                "Re-run POST /api/v1/investigate/path to verify connectivity contracts",
            ],
            "note": "Paqtra never writes Cilium BPF maps or attaches programs.",
        }),
        confidence,
    }
}

fn detect_unsupported(spec: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let blob = spec.to_string();
    if blob.contains("toFQDNs") || blob.contains("toFQDN") {
        out.push("toFQDNs not evaluated in v1 preview".into());
    }
    if blob.contains("\"http\"") || blob.contains("l7") || blob.contains("L7") {
        out.push("L7 rules not evaluated in v1 preview".into());
    }
    if blob.contains("toEntities") {
        out.push("toEntities matching is partial — results may be unknown".into());
    }
    if blob.contains("deny") && (blob.contains("ingress") || blob.contains("egress")) {
        // Cilium deny rules coexist with allows — we don't model precedence fully.
        if blob.contains("ingressDeny") || blob.contains("egressDeny") {
            out.push("ingressDeny/egressDeny precedence not fully modeled".into());
        }
    }
    out
}

fn endpoint_selector_labels(spec: &Value) -> Vec<(String, String)> {
    spec.pointer("/endpointSelector/matchLabels")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn collect_ports(spec: &Value) -> Vec<u16> {
    let mut ports = Vec::new();
    for key in ["ingress", "egress"] {
        if let Some(rules) = spec.get(key).and_then(|v| v.as_array()) {
            for rule in rules {
                if let Some(to_ports) = rule.get("toPorts").and_then(|v| v.as_array()) {
                    for tp in to_ports {
                        if let Some(plist) = tp.get("ports").and_then(|v| v.as_array()) {
                            for p in plist {
                                if let Some(n) = p
                                    .get("port")
                                    .and_then(|x| x.as_str())
                                    .and_then(|s| s.parse().ok())
                                    .or_else(|| {
                                        p.get("port").and_then(|x| x.as_u64()).map(|n| n as u16)
                                    })
                                {
                                    ports.push(n);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ports.sort_unstable();
    ports.dedup();
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_fqdn() {
        let spec = json!({"egress": [{"toFQDNs": [{"matchName": "example.com"}]}]});
        let u = detect_unsupported(&spec);
        assert!(u.iter().any(|s| s.contains("toFQDNs")));
    }

    #[test]
    fn selector_and_ports() {
        let spec = json!({
            "endpointSelector": {"matchLabels": {"app": "web"}},
            "ingress": [{"toPorts": [{"ports": [{"port": "443", "protocol": "TCP"}]}]}]
        });
        assert_eq!(
            endpoint_selector_labels(&spec),
            vec![("app".into(), "web".into())]
        );
        assert_eq!(collect_ports(&spec), vec![443]);
    }
}
