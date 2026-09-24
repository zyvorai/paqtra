// Compliance endpoints
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use super::{actor_from_claims, audit_log, to_json, track_request};
use crate::AppState;

/// Typed request for the compliance audit endpoint.
#[derive(Debug, Deserialize)]
pub struct RunAuditRequest {
    #[serde(default = "default_framework")]
    pub framework: String,
}

fn default_framework() -> String {
    "pci-dss-4.0".to_string()
}

/// A compliance framework definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceFramework {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub control_count: u32,
}

/// Result of a compliance audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub audit_id: String,
    pub framework: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub total_controls: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub score: f64,
    pub findings: Vec<AuditFinding>,
}

/// Individual finding within an audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    pub control_id: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub description: String,
}

/// Security posture summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPosture {
    pub score: f64,
    pub trend: String,
    pub policy_coverage: f64,
    pub encryption_coverage: f64,
    pub namespace_isolation: f64,
    pub last_audit: Option<String>,
}

/// Supported compliance frameworks
fn supported_frameworks() -> Vec<ComplianceFramework> {
    vec![
        ComplianceFramework {
            id: "pci-dss-4.0".to_string(),
            name: "PCI-DSS".to_string(),
            version: "4.0".to_string(),
            description: "Payment Card Industry Data Security Standard".to_string(),
            control_count: 64,
        },
        ComplianceFramework {
            id: "soc2-type2".to_string(),
            name: "SOC2".to_string(),
            version: "Type II".to_string(),
            description: "Service Organization Control 2".to_string(),
            control_count: 48,
        },
        ComplianceFramework {
            id: "hipaa".to_string(),
            name: "HIPAA".to_string(),
            version: "2013".to_string(),
            description: "Health Insurance Portability and Accountability Act".to_string(),
            control_count: 42,
        },
        ComplianceFramework {
            id: "gdpr".to_string(),
            name: "GDPR".to_string(),
            version: "2018".to_string(),
            description: "General Data Protection Regulation".to_string(),
            control_count: 35,
        },
        ComplianceFramework {
            id: "iso27001-2022".to_string(),
            name: "ISO27001".to_string(),
            version: "2022".to_string(),
            description: "Information Security Management System".to_string(),
            control_count: 93,
        },
        ComplianceFramework {
            id: "nist-csf-2.0".to_string(),
            name: "NIST".to_string(),
            version: "CSF 2.0".to_string(),
            description: "NIST Cybersecurity Framework".to_string(),
            control_count: 108,
        },
    ]
}

pub async fn list_frameworks(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, StatusCode> {
    track_request(&state, |_| {}).await;

    let frameworks = supported_frameworks();
    Ok(Json(json!({
        "frameworks": frameworks,
        "total": frameworks.len(),
    })))
}

pub async fn run_audit(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<RunAuditRequest>,
) -> Result<Json<Value>, StatusCode> {
    if super::check_editor(&state, &claims).is_err() {
        return Err(StatusCode::FORBIDDEN);
    }
    track_request(&state, |_| {}).await;

    let framework = &req.framework;
    let mut findings = Vec::new();

    // --- Check 1: Default-deny network policies per namespace ---
    let policies = state.k8s.list_policies().await.unwrap_or_default();
    let ns_json = state
        .k8s
        .kubectl_json(&["get", "namespaces", "-o", "json"])
        .await;
    let all_ns: Vec<String> = ns_json
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    i.get("metadata")
                        .and_then(|m| m.get("name"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();

    let ns_with_policies: std::collections::HashSet<String> =
        policies.iter().map(|p| p.namespace.clone()).collect();

    let ns_without: Vec<&str> = all_ns
        .iter()
        .filter(|ns| {
            !ns_with_policies.contains(ns.as_str())
                && !ns.starts_with("kube-")
                && *ns != "kube-system"
        })
        .map(|s| s.as_str())
        .collect();

    if ns_without.is_empty() {
        findings.push(AuditFinding {
            control_id: format!("{}-NET-1", framework_prefix(framework)),
            title: "Default-deny network policies".to_string(),
            status: "passed".to_string(),
            severity: "high".to_string(),
            description: "All non-system namespaces have CiliumNetworkPolicy coverage.".to_string(),
        });
    } else {
        findings.push(AuditFinding {
            control_id: format!("{}-NET-1", framework_prefix(framework)),
            title: "Default-deny network policies".to_string(),
            status: "failed".to_string(),
            severity: "high".to_string(),
            description: format!(
                "The following namespaces lack CiliumNetworkPolicy coverage: {}",
                ns_without.join(", ")
            ),
        });
    }

    // --- Check 2: Hubble monitoring enabled ---
    let hubble_healthy = state.hubble.is_healthy().await;
    if hubble_healthy {
        findings.push(AuditFinding {
            control_id: format!("{}-MON-1", framework_prefix(framework)),
            title: "Network flow monitoring".to_string(),
            status: "passed".to_string(),
            severity: "high".to_string(),
            description: "Hubble relay is reachable and actively monitoring network flows."
                .to_string(),
        });
    } else {
        findings.push(AuditFinding {
            control_id: format!("{}-MON-1", framework_prefix(framework)),
            title: "Network flow monitoring".to_string(),
            status: "failed".to_string(),
            severity: "critical".to_string(),
            description: "Hubble relay is unreachable. Network flow monitoring is not operational."
                .to_string(),
        });
    }

    // --- Check 3: Encryption status via cilium-config ConfigMap ---
    let cilium_cfg = state
        .k8s
        .kubectl_json(&[
            "get",
            "configmap",
            "cilium-config",
            "-n",
            "kube-system",
            "-o",
            "json",
        ])
        .await;
    let encryption_enabled = cilium_cfg
        .get("data")
        .and_then(|d| d.get("enable-wireguard"))
        .and_then(|v| v.as_str())
        .map(|v| v == "true")
        .unwrap_or(false)
        || cilium_cfg
            .get("data")
            .and_then(|d| d.get("encrypt-node"))
            .and_then(|v| v.as_str())
            .map(|v| v == "true")
            .unwrap_or(false)
        || cilium_cfg
            .get("data")
            .and_then(|d| d.get("enable-ipsec"))
            .and_then(|v| v.as_str())
            .map(|v| v == "true")
            .unwrap_or(false);

    if encryption_enabled {
        findings.push(AuditFinding {
            control_id: format!("{}-ENC-1", framework_prefix(framework)),
            title: "Encryption in transit".to_string(),
            status: "passed".to_string(),
            severity: "critical".to_string(),
            description: "Transparent encryption (WireGuard or IPsec) is enabled in the Cilium \
                          configuration."
                .to_string(),
        });
    } else {
        findings.push(AuditFinding {
            control_id: format!("{}-ENC-1", framework_prefix(framework)),
            title: "Encryption in transit".to_string(),
            status: "failed".to_string(),
            severity: "critical".to_string(),
            description: "No transparent encryption (WireGuard or IPsec) is enabled in the \
                          cilium-config ConfigMap. Inter-pod traffic may be unencrypted."
                .to_string(),
        });
    }

    // --- Check 4: Dropped flows ---
    let flows = state.hubble.get_flows(500, None).await.unwrap_or_default();
    let dropped_count = flows.iter().filter(|f| f.verdict == "DROPPED").count();
    let total_flows = flows.len();

    if dropped_count == 0 {
        findings.push(AuditFinding {
            control_id: format!("{}-DRP-1", framework_prefix(framework)),
            title: "Dropped network flows".to_string(),
            status: "passed".to_string(),
            severity: "medium".to_string(),
            description: format!(
                "No dropped flows detected in the last {} observed flows.",
                total_flows
            ),
        });
    } else {
        let drop_pct = if total_flows > 0 {
            (dropped_count as f64 / total_flows as f64 * 100.0 * 10.0).round() / 10.0
        } else {
            0.0
        };
        findings.push(AuditFinding {
            control_id: format!("{}-DRP-1", framework_prefix(framework)),
            title: "Dropped network flows".to_string(),
            status: "failed".to_string(),
            severity: "medium".to_string(),
            description: format!(
                "{} of {} observed flows were dropped ({:.1}%). Investigate policy \
                 denials or misconfigured endpoints.",
                dropped_count, total_flows, drop_pct
            ),
        });
    }

    let total_controls = findings.len() as u32;
    let passed = findings.iter().filter(|f| f.status == "passed").count() as u32;
    let failed = findings.iter().filter(|f| f.status == "failed").count() as u32;
    let skipped = total_controls - passed - failed;
    let score = if total_controls > 0 {
        (passed as f64 / total_controls as f64 * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };

    let audit = AuditResult {
        audit_id: uuid::Uuid::new_v4().to_string(),
        framework: framework.to_string(),
        status: "completed".to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
        total_controls,
        passed,
        failed,
        skipped,
        score,
        findings,
    };

    audit_log(
        &state,
        "compliance.audit",
        framework,
        "",
        &format!("Compliance audit completed: score={}", score),
        &actor_from_claims(&claims),
        "success",
    )
    .await;

    Ok(Json(to_json(&audit)))
}

/// Map a framework ID to a short prefix for control IDs.
fn framework_prefix(framework: &str) -> &str {
    match framework {
        "pci-dss-4.0" => "PCI",
        "soc2-type2" => "SOC2",
        "hipaa" => "HIPAA",
        "gdpr" => "GDPR",
        "iso27001-2022" => "ISO",
        "nist-csf-2.0" => "NIST",
        _ => "GEN",
    }
}

pub async fn security_posture(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
) -> Result<Json<Value>, StatusCode> {
    if super::check_admin(&state, &claims).is_err() {
        return Err(StatusCode::FORBIDDEN);
    }
    track_request(&state, |_| {}).await;

    // Query real cluster state for posture calculation
    let ns_data = state.k8s.list_policies().await.unwrap_or_default();
    let policies_count = ns_data.len();

    // Get namespace count
    let ns_json = state
        .k8s
        .kubectl_json(&["get", "namespaces", "-o", "json"])
        .await;
    let namespaces_total = ns_json
        .get("items")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    // Find namespaces with policies
    let ns_with_policies: std::collections::HashSet<String> =
        ns_data.iter().map(|p| p.namespace.clone()).collect();
    let all_ns: Vec<String> = ns_json
        .get("items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    i.get("metadata")
                        .and_then(|m| m.get("name"))
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();
    let ns_without: Vec<&str> = all_ns
        .iter()
        .filter(|ns| !ns_with_policies.contains(ns.as_str()) && !ns.starts_with("kube-"))
        .map(|s| s.as_str())
        .collect();

    let policy_coverage = if namespaces_total > 0 {
        ((namespaces_total - ns_without.len()) as f64 / namespaces_total as f64 * 100.0 * 10.0)
            .round()
            / 10.0
    } else {
        0.0
    };

    let score = policy_coverage; // Score reflects actual policy coverage percentage

    let posture = SecurityPosture {
        score,
        trend: "current".to_string(),
        policy_coverage,
        encryption_coverage: 0.0,
        namespace_isolation: policy_coverage,
        last_audit: Some(chrono::Utc::now().to_rfc3339()),
    };

    let mut recommendations = Vec::new();
    if !ns_without.is_empty() {
        recommendations.push(format!(
            "Apply network policies to namespaces: {}",
            ns_without.join(", ")
        ));
    }
    if policies_count == 0 {
        recommendations
            .push("No CiliumNetworkPolicies found -- create least-privilege policies".to_string());
    }

    Ok(Json(json!({
        "posture": to_json(&posture),
        "breakdown": {
            "namespaces_total": namespaces_total,
            "namespaces_with_policies": ns_with_policies.len(),
            "namespaces_without_policies": ns_without,
            "total_policies": policies_count,
        },
        "recommendations": recommendations,
    })))
}
