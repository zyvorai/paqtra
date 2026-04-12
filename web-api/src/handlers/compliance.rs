// Compliance endpoints
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use super::{track_request, to_json};

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

/// Build sample audit findings for a given framework.
fn sample_findings(framework: &str) -> Vec<AuditFinding> {
    match framework {
        "pci-dss-4.0" => vec![
            AuditFinding {
                control_id: "PCI-1.3.1".to_string(),
                title: "Restrict inbound traffic to system components in the CDE".to_string(),
                status: "passed".to_string(),
                severity: "high".to_string(),
                description: "CiliumNetworkPolicy default-deny ingress is enforced on all \
                              namespaces in the cardholder data environment."
                    .to_string(),
            },
            AuditFinding {
                control_id: "PCI-1.3.2".to_string(),
                title: "Restrict outbound traffic from the CDE".to_string(),
                status: "failed".to_string(),
                severity: "high".to_string(),
                description: "Namespace 'payment-processing' allows unrestricted egress to \
                              the internet. A default-deny egress policy with explicit \
                              allowlisting is required."
                    .to_string(),
            },
            AuditFinding {
                control_id: "PCI-2.2.7".to_string(),
                title: "Encrypt all non-console administrative access".to_string(),
                status: "passed".to_string(),
                severity: "critical".to_string(),
                description: "All inter-pod communication uses WireGuard transparent encryption \
                              via Cilium. No unencrypted admin channels detected."
                    .to_string(),
            },
            AuditFinding {
                control_id: "PCI-6.5.4".to_string(),
                title: "Insecure direct object references".to_string(),
                status: "passed".to_string(),
                severity: "medium".to_string(),
                description: "L7 HTTP policies enforce path-based access control on all \
                              API gateway endpoints."
                    .to_string(),
            },
            AuditFinding {
                control_id: "PCI-10.2.1".to_string(),
                title: "Audit trails for all access to cardholder data".to_string(),
                status: "failed".to_string(),
                severity: "high".to_string(),
                description: "Hubble flow logs are enabled but retention is set to 1 hour. \
                              PCI-DSS requires a minimum of 90 days of audit trail retention."
                    .to_string(),
            },
        ],
        "soc2-type2" => vec![
            AuditFinding {
                control_id: "CC6.1".to_string(),
                title: "Logical and physical access controls".to_string(),
                status: "passed".to_string(),
                severity: "high".to_string(),
                description: "Network segmentation enforced via CiliumNetworkPolicy across \
                              all production namespaces."
                    .to_string(),
            },
            AuditFinding {
                control_id: "CC6.6".to_string(),
                title: "System boundary protections".to_string(),
                status: "failed".to_string(),
                severity: "medium".to_string(),
                description: "Three namespaces (dev, staging, sandbox) lack default-deny \
                              ingress policies."
                    .to_string(),
            },
            AuditFinding {
                control_id: "CC7.2".to_string(),
                title: "System monitoring for anomalies".to_string(),
                status: "passed".to_string(),
                severity: "high".to_string(),
                description: "Anomaly detection pipeline is active with 5-minute detection \
                              windows and automated alerting."
                    .to_string(),
            },
        ],
        _ => vec![
            AuditFinding {
                control_id: "GEN-1.1".to_string(),
                title: "Network segmentation".to_string(),
                status: "passed".to_string(),
                severity: "high".to_string(),
                description: "Default-deny network policies are applied to production namespaces."
                    .to_string(),
            },
            AuditFinding {
                control_id: "GEN-2.1".to_string(),
                title: "Encryption in transit".to_string(),
                status: "passed".to_string(),
                severity: "high".to_string(),
                description: "WireGuard transparent encryption is enabled cluster-wide."
                    .to_string(),
            },
            AuditFinding {
                control_id: "GEN-3.1".to_string(),
                title: "Audit logging".to_string(),
                status: "failed".to_string(),
                severity: "medium".to_string(),
                description: "Flow log retention does not meet the framework's minimum \
                              retention period."
                    .to_string(),
            },
        ],
    }
}

pub async fn run_audit(
    State(state): State<Arc<AppState>>,
    claims: Option<axum::Extension<crate::middleware::auth::Claims>>,
    Json(req): Json<RunAuditRequest>,
) -> Result<Json<Value>, StatusCode> {
    if super::check_admin(&state, &claims).is_err() {
        return Err(StatusCode::FORBIDDEN);
    }
    track_request(&state, |_| {}).await;

    let framework = &req.framework;

    let findings = sample_findings(framework);
    let total_controls = findings.len() as u32;
    let passed = findings.iter().filter(|f| f.status == "passed").count() as u32;
    let failed = findings.iter().filter(|f| f.status == "failed").count() as u32;
    let skipped = total_controls - passed - failed;
    let score = if total_controls > 0 {
        (passed as f64 / total_controls as f64) * 100.0
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

    Ok(Json(to_json(&audit)))
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
    let ns_json = state.k8s.kubectl_json(&["get", "namespaces", "-o", "json"]).await;
    let namespaces_total = ns_json.get("items").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    // Find namespaces with policies
    let ns_with_policies: std::collections::HashSet<String> = ns_data.iter().map(|p| p.namespace.clone()).collect();
    let all_ns: Vec<String> = ns_json.get("items").and_then(|v| v.as_array())
        .map(|items| items.iter()
            .filter_map(|i| i.get("metadata").and_then(|m| m.get("name")).and_then(|v| v.as_str()).map(String::from))
            .collect())
        .unwrap_or_default();
    let ns_without: Vec<&str> = all_ns.iter()
        .filter(|ns| !ns_with_policies.contains(ns.as_str()) && !ns.starts_with("kube-"))
        .map(|s| s.as_str())
        .collect();

    let policy_coverage = if namespaces_total > 0 {
        ((namespaces_total - ns_without.len()) as f64 / namespaces_total as f64 * 100.0 * 10.0).round() / 10.0
    } else { 0.0 };

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
        recommendations.push(format!("Apply network policies to namespaces: {}", ns_without.join(", ")));
    }
    if policies_count == 0 {
        recommendations.push("No CiliumNetworkPolicies found -- create least-privilege policies".to_string());
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
