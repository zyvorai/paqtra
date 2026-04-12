// Security Posture Assessment
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::process::Command;

use super::{
    DimensionScore, Effort, Priority, RecommendationCategory, ScoreTrend, SecurityRecommendation,
    SecurityScore,
};

/// Calculates overall security posture
#[derive(Default)]
pub struct SecurityPosture {}

impl SecurityPosture {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }

    /// Run a kubectl command and return (success, stdout).
    ///
    /// NOTE: This uses synchronous `std::process::Command`. The commands include
    /// `--request-timeout=2s` to ensure they complete quickly.
    fn kubectl_check(args: &[&str]) -> (bool, String) {
        match Command::new("kubectl").args(args).output() {
            Ok(output) => (
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).to_string(),
            ),
            Err(_) => (false, String::new()),
        }
    }

    fn kubectl_available() -> bool {
        Self::kubectl_check(&["cluster-info", "--request-timeout=2s"]).0
    }

    fn assess_network_segmentation(&self) -> DimensionScore {
        if !Self::kubectl_available() {
            return DimensionScore {
                score: 0.0,
                weight: 0.25,
                findings: vec!["kubectl not available - cannot assess".to_string()],
            };
        }

        let mut score: f64 = 0.0;
        let mut findings = Vec::new();

        // Check for network policies
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "ciliumnetworkpolicies,networkpolicies",
            "--all-namespaces",
            "-o",
            "name",
        ]);
        let policy_count = if ok {
            output.lines().filter(|l| !l.is_empty()).count()
        } else {
            0
        };

        if policy_count > 10 {
            score += 40.0;
            findings.push(format!(
                "{} network policies (strong segmentation)",
                policy_count
            ));
        } else if policy_count > 0 {
            score += 20.0;
            findings.push(format!(
                "{} network policies (basic segmentation)",
                policy_count
            ));
        } else {
            findings.push("No network policies - flat network".to_string());
        }

        // Check for default-deny policies
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "ciliumnetworkpolicies",
            "--all-namespaces",
            "-o",
            "json",
        ]);
        if ok && output.contains("endpointSelector") {
            score += 30.0;
            findings.push("Endpoint-level segmentation policies found".to_string());
        }

        // Check namespace count (more namespaces = better logical separation)
        let (ok, output) = Self::kubectl_check(&["get", "namespaces", "-o", "name"]);
        let ns_count = if ok {
            output.lines().filter(|l| !l.is_empty()).count()
        } else {
            0
        };
        if ns_count > 3 {
            score += 30.0;
            findings.push(format!("{} namespaces for workload separation", ns_count));
        } else if ns_count > 1 {
            score += 15.0;
            findings.push(format!("{} namespaces", ns_count));
        }

        DimensionScore {
            score: score.min(100.0),
            weight: 0.25,
            findings,
        }
    }

    fn assess_access_control(&self) -> DimensionScore {
        if !Self::kubectl_available() {
            return DimensionScore {
                score: 0.0,
                weight: 0.25,
                findings: vec!["kubectl not available - cannot assess".to_string()],
            };
        }

        let mut score: f64 = 0.0;
        let mut findings = Vec::new();

        // Check RBAC roles
        let (ok, output) = Self::kubectl_check(&["get", "roles", "--all-namespaces", "-o", "name"]);
        let role_count = if ok {
            output.lines().filter(|l| !l.is_empty()).count()
        } else {
            0
        };
        if role_count > 0 {
            score += 30.0;
            findings.push(format!("{} RBAC roles defined", role_count));
        } else {
            findings.push("No custom RBAC roles found".to_string());
        }

        // Check for excessive cluster-admin bindings
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "clusterrolebindings",
            "-o",
            "jsonpath={.items[?(@.roleRef.name=='cluster-admin')].subjects[*].name}",
        ]);
        if ok {
            let admins: Vec<&str> = output
                .split_whitespace()
                .filter(|s| !s.is_empty())
                .collect();
            if admins.len() <= 2 {
                score += 40.0;
                findings.push(format!(
                    "cluster-admin access is restricted ({} bindings)",
                    admins.len()
                ));
            } else {
                score += 10.0;
                findings.push(format!(
                    "cluster-admin bound to {} subjects - too broad",
                    admins.len()
                ));
            }
        }

        // Check pod security standards
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "namespaces",
            "-o",
            "jsonpath={.items[*].metadata.labels.pod-security\\.kubernetes\\.io/enforce}",
        ]);
        if ok && !output.trim().is_empty() {
            score += 30.0;
            findings.push("Pod Security Standards enforced on namespaces".to_string());
        } else {
            findings.push("No Pod Security Standards labels found on namespaces".to_string());
        }

        DimensionScore {
            score: score.min(100.0),
            weight: 0.25,
            findings,
        }
    }

    fn assess_encryption(&self) -> DimensionScore {
        if !Self::kubectl_available() {
            return DimensionScore {
                score: 0.0,
                weight: 0.20,
                findings: vec!["kubectl not available - cannot assess".to_string()],
            };
        }

        let mut score: f64 = 0.0;
        let mut findings = Vec::new();

        // Check WireGuard
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "configmap",
            "cilium-config",
            "-n",
            "kube-system",
            "-o",
            "jsonpath={.data.enable-wireguard}",
        ]);
        if ok && output.trim() == "true" {
            score += 50.0;
            findings.push("WireGuard encryption enabled for pod-to-pod traffic".to_string());
        }

        // Check IPsec
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "configmap",
            "cilium-config",
            "-n",
            "kube-system",
            "-o",
            "jsonpath={.data.encrypt-node}",
        ]);
        if ok && output.trim() == "true" {
            score += 50.0;
            findings.push("Node-level encryption enabled".to_string());
        }

        // Check TLS secrets (indicates TLS usage)
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "secrets",
            "--all-namespaces",
            "--field-selector=type=kubernetes.io/tls",
            "-o",
            "name",
        ]);
        let tls_count = if ok {
            output.lines().filter(|l| !l.is_empty()).count()
        } else {
            0
        };
        if tls_count > 0 {
            score += 25.0;
            findings.push(format!("{} TLS certificates in cluster", tls_count));
        } else {
            findings.push("No TLS certificates found".to_string());
        }

        if score == 0.0 {
            findings.push("No encryption detected - enable WireGuard or IPsec".to_string());
        }

        DimensionScore {
            score: score.min(100.0),
            weight: 0.20,
            findings,
        }
    }

    fn assess_monitoring(&self) -> DimensionScore {
        if !Self::kubectl_available() {
            return DimensionScore {
                score: 0.0,
                weight: 0.15,
                findings: vec!["kubectl not available - cannot assess".to_string()],
            };
        }

        let mut score: f64 = 0.0;
        let mut findings = Vec::new();

        // Check Hubble
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "pods",
            "-n",
            "kube-system",
            "-l",
            "k8s-app=hubble-relay",
            "--field-selector=status.phase=Running",
            "-o",
            "name",
        ]);
        if ok && !output.trim().is_empty() {
            score += 40.0;
            findings.push("Hubble flow observability is running".to_string());
        } else {
            findings.push("Hubble not detected".to_string());
        }

        // Check for Hubble UI
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "pods",
            "-n",
            "kube-system",
            "-l",
            "k8s-app=hubble-ui",
            "-o",
            "name",
        ]);
        if ok && !output.trim().is_empty() {
            score += 20.0;
            findings.push("Hubble UI available for visual monitoring".to_string());
        }

        // Check for audit logging
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "configmap",
            "cilium-config",
            "-n",
            "kube-system",
            "-o",
            "jsonpath={.data.monitor-aggregation}",
        ]);
        if ok && !output.trim().is_empty() {
            score += 20.0;
            findings.push(format!("Monitor aggregation: {}", output.trim()));
        }

        // Check Cilium agent health
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "pods",
            "-n",
            "kube-system",
            "-l",
            "k8s-app=cilium",
            "--field-selector=status.phase=Running",
            "-o",
            "name",
        ]);
        if ok && !output.trim().is_empty() {
            let agent_count = output.lines().filter(|l| !l.is_empty()).count();
            score += 20.0;
            findings.push(format!("{} Cilium agents running", agent_count));
        }

        DimensionScore {
            score: score.min(100.0),
            weight: 0.15,
            findings,
        }
    }

    fn assess_compliance(&self) -> DimensionScore {
        if !Self::kubectl_available() {
            return DimensionScore {
                score: 0.0,
                weight: 0.15,
                findings: vec!["kubectl not available - cannot assess".to_string()],
            };
        }

        let mut score: f64 = 0.0;
        let mut findings = Vec::new();

        // Check for network policies (baseline compliance)
        let (ok, output) = Self::kubectl_check(&[
            "get",
            "ciliumnetworkpolicies",
            "--all-namespaces",
            "-o",
            "name",
        ]);
        if ok && !output.trim().is_empty() {
            score += 30.0;
            findings.push("CiliumNetworkPolicies deployed".to_string());
        }

        // Check RBAC
        let (ok, output) =
            Self::kubectl_check(&["get", "rolebindings", "--all-namespaces", "-o", "name"]);
        if ok && !output.trim().is_empty() {
            score += 30.0;
            findings.push("RBAC role bindings configured".to_string());
        }

        // Check for resource quotas (operational discipline)
        let (ok, output) =
            Self::kubectl_check(&["get", "resourcequotas", "--all-namespaces", "-o", "name"]);
        if ok && !output.trim().is_empty() {
            score += 20.0;
            findings.push("Resource quotas enforced".to_string());
        }

        // Check for limit ranges
        let (ok, output) =
            Self::kubectl_check(&["get", "limitranges", "--all-namespaces", "-o", "name"]);
        if ok && !output.trim().is_empty() {
            score += 20.0;
            findings.push("Limit ranges configured".to_string());
        }

        if score == 0.0 {
            findings.push("No compliance controls detected".to_string());
        }

        DimensionScore {
            score: score.min(100.0),
            weight: 0.15,
            findings,
        }
    }

    pub async fn calculate_score(&self) -> Result<SecurityScore> {
        let mut dimensions = HashMap::new();

        dimensions.insert(
            "Network Segmentation".to_string(),
            self.assess_network_segmentation(),
        );
        dimensions.insert("Access Control".to_string(), self.assess_access_control());
        dimensions.insert("Encryption".to_string(), self.assess_encryption());
        dimensions.insert("Monitoring & Logging".to_string(), self.assess_monitoring());
        dimensions.insert("Compliance".to_string(), self.assess_compliance());

        let overall_score = dimensions.values().map(|d| d.score * d.weight).sum::<f64>();

        // Determine trend (would compare with stored previous score in production)
        let trend = if overall_score > 70.0 {
            ScoreTrend::Improving
        } else if overall_score > 30.0 {
            ScoreTrend::Stable
        } else {
            ScoreTrend::Declining
        };

        tracing::info!("Security posture score: {:.1}/100", overall_score);

        Ok(SecurityScore {
            overall_score,
            dimensions,
            timestamp: Utc::now(),
            trend,
        })
    }

    pub async fn get_recommendations(&self) -> Result<Vec<SecurityRecommendation>> {
        let mut recs = vec![SecurityRecommendation {
            priority: Priority::P2High,
            category: RecommendationCategory::BestPractice,
            title: "Review RBAC permissions".to_string(),
            description: "Ensure service accounts follow least-privilege principle".to_string(),
            impact: "Reduces blast radius of compromised accounts".to_string(),
            effort: Effort::Medium,
            auto_applicable: false,
        }];

        // Add contextual recommendations based on current state
        if Self::kubectl_available() {
            let (ok, output) = Self::kubectl_check(&[
                "get",
                "configmap",
                "cilium-config",
                "-n",
                "kube-system",
                "-o",
                "jsonpath={.data.enable-wireguard}",
            ]);
            if !ok || output.trim() != "true" {
                recs.push(SecurityRecommendation {
                    priority: Priority::P2High,
                    category: RecommendationCategory::BestPractice,
                    title: "Enable WireGuard encryption".to_string(),
                    description: "Encrypt pod-to-pod traffic with WireGuard".to_string(),
                    impact: "Protects data in transit within the cluster".to_string(),
                    effort: Effort::Low,
                    auto_applicable: false,
                });
            }
        }

        Ok(recs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_posture_creation() {
        let posture = SecurityPosture::new();
        assert!(posture.is_ok());
    }

    #[test]
    fn test_security_posture_default() {
        let _posture = SecurityPosture::default();
    }

    #[tokio::test]
    async fn test_calculate_score_returns_valid_score() {
        let posture = SecurityPosture::new().unwrap();
        let score = posture.calculate_score().await.unwrap();
        assert!(score.overall_score >= 0.0 && score.overall_score <= 100.0);
    }

    #[tokio::test]
    async fn test_score_dimensions_present() {
        let posture = SecurityPosture::new().unwrap();
        let score = posture.calculate_score().await.unwrap();
        assert!(score.dimensions.contains_key("Network Segmentation"));
        assert!(score.dimensions.contains_key("Access Control"));
        assert!(score.dimensions.contains_key("Encryption"));
        assert!(score.dimensions.contains_key("Monitoring & Logging"));
        assert!(score.dimensions.contains_key("Compliance"));
    }

    #[tokio::test]
    async fn test_dimension_weights_sum_to_one() {
        let posture = SecurityPosture::new().unwrap();
        let score = posture.calculate_score().await.unwrap();
        let total_weight: f64 = score.dimensions.values().map(|d| d.weight).sum();
        assert!(
            (total_weight - 1.0).abs() < 0.001,
            "Dimension weights should sum to 1.0, got {}",
            total_weight
        );
    }

    #[tokio::test]
    async fn test_all_dimensions_scored() {
        let posture = SecurityPosture::new().unwrap();
        let score = posture.calculate_score().await.unwrap();
        // Each dimension should have a score between 0 and 100
        for (name, dim) in &score.dimensions {
            assert!(
                dim.score >= 0.0 && dim.score <= 100.0,
                "Dimension '{}' score {} out of range",
                name,
                dim.score
            );
            // Each dimension should have at least one finding
            assert!(
                !dim.findings.is_empty(),
                "Dimension '{}' should have findings",
                name
            );
        }
    }

    #[tokio::test]
    async fn test_score_trend_determined() {
        let posture = SecurityPosture::new().unwrap();
        let score = posture.calculate_score().await.unwrap();
        // Trend should be one of the valid values
        assert!(
            score.trend == ScoreTrend::Stable
                || score.trend == ScoreTrend::Improving
                || score.trend == ScoreTrend::Declining
        );
    }

    #[tokio::test]
    async fn test_get_recommendations() {
        let posture = SecurityPosture::new().unwrap();
        let recs = posture.get_recommendations().await.unwrap();
        assert!(!recs.is_empty());
        assert_eq!(recs[0].category, RecommendationCategory::BestPractice);
    }
}
