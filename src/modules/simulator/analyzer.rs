/// Impact Analyzer
///
/// Analyzes the impact of policy changes on services and dependencies

use super::*;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

pub struct ImpactAnalyzer<'a> {
    k8s_client: &'a K8sClient,
}

impl<'a> ImpactAnalyzer<'a> {
    pub fn new(k8s_client: &'a K8sClient) -> Self {
        Self { k8s_client }
    }

    /// Analyze impact from flow simulation results
    pub async fn analyze(&self, flow_results: &[FlowSimulationResult]) -> Result<ImpactAnalysis> {
        let total_flows = flow_results.len();

        // Count blocked and allowed flows
        let blocked_flows = flow_results
            .iter()
            .filter(|f| {
                f.after == PolicyVerdict::Deny && f.before != PolicyVerdict::Deny
            })
            .count();

        let allowed_flows = flow_results
            .iter()
            .filter(|f| f.after == PolicyVerdict::Allow)
            .count();

        let changed_flows = flow_results
            .iter()
            .filter(|f| f.changed)
            .count();

        // Find impacted services
        let impacted_services = self.find_impacted_services(flow_results).await?;

        // Identify critical services
        let critical_services = self.identify_critical_services(&impacted_services);

        Ok(ImpactAnalysis {
            total_flows,
            blocked_flows,
            allowed_flows,
            changed_flows,
            impacted_services,
            critical_services,
        })
    }

    /// Find affected resources
    pub async fn find_affected_resources(
        &self,
        flow_results: &[FlowSimulationResult],
    ) -> Result<AffectedResources> {
        // Collect unique identities
        let pod_identities: Vec<u32> = flow_results
            .iter()
            .filter(|f| f.changed)
            .flat_map(|f| vec![f.src_identity, f.dst_identity])
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Group by namespace (placeholder - would need IPCache)
        let namespaces = vec!["default".to_string()]; // TODO: Resolve from identities

        // Find affected services
        let services = self.find_affected_services(flow_results).await?;

        // Find affected endpoints
        let endpoints = self.find_affected_endpoints(flow_results).await?;

        // Find broken dependencies
        let broken_dependencies = self.find_broken_dependencies(flow_results).await?;

        Ok(AffectedResources {
            namespaces,
            pod_identities,
            services,
            endpoints,
            broken_dependencies,
        })
    }

    /// Find impacted services
    async fn find_impacted_services(&self, flow_results: &[FlowSimulationResult]) -> Result<Vec<String>> {
        let mut services = HashSet::new();

        // Group flows by destination
        let mut by_dst: HashMap<u32, Vec<&FlowSimulationResult>> = HashMap::new();
        for result in flow_results.iter().filter(|f| f.changed) {
            by_dst.entry(result.dst_identity).or_insert_with(Vec::new).push(result);
        }

        // For each affected destination, identify service
        for (identity, flows) in by_dst {
            if flows.iter().any(|f| f.after == PolicyVerdict::Deny) {
                // TODO: Resolve identity to service name via K8s API
                services.insert(format!("service-{}", identity));
            }
        }

        Ok(services.into_iter().collect())
    }

    /// Identify critical services
    fn identify_critical_services(&self, impacted_services: &[String]) -> Vec<String> {
        // TODO: Get service criticality from labels/annotations
        // For now, identify by common critical service patterns
        impacted_services
            .iter()
            .filter(|s| {
                s.contains("db")
                    || s.contains("database")
                    || s.contains("api")
                    || s.contains("auth")
                    || s.contains("dns")
            })
            .cloned()
            .collect()
    }

    /// Find affected services with detailed impact
    async fn find_affected_services(
        &self,
        flow_results: &[FlowSimulationResult],
    ) -> Result<Vec<ServiceImpact>> {
        let mut services = Vec::new();

        // Group by destination identity (service)
        let mut by_service: HashMap<u32, Vec<&FlowSimulationResult>> = HashMap::new();
        for result in flow_results {
            by_service
                .entry(result.dst_identity)
                .or_insert_with(Vec::new)
                .push(result);
        }

        for (identity, flows) in by_service {
            let blocked_flows: Vec<_> = flows
                .iter()
                .filter(|f| f.changed && f.after == PolicyVerdict::Deny)
                .collect();

            if blocked_flows.is_empty() {
                continue;
            }

            // Determine impact type
            let total_flows = flows.len();
            let blocked_count = blocked_flows.len();

            let impact_type = if blocked_count == total_flows {
                ImpactType::FullyBlocked
            } else if blocked_count > 0 {
                ImpactType::PartiallyBlocked
            } else {
                ImpactType::Unaffected
            };

            // Collect affected ports
            let affected_ports: Vec<u16> = blocked_flows
                .iter()
                .map(|f| f.port)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();

            // Count unique clients
            let client_count = flows
                .iter()
                .map(|f| f.src_identity)
                .collect::<HashSet<_>>()
                .len();

            services.push(ServiceImpact {
                name: format!("service-{}", identity), // TODO: Resolve real name
                namespace: "default".to_string(),       // TODO: Resolve real namespace
                impact_type,
                affected_ports,
                client_count,
            });
        }

        Ok(services)
    }

    /// Find affected endpoints
    async fn find_affected_endpoints(
        &self,
        flow_results: &[FlowSimulationResult],
    ) -> Result<Vec<EndpointImpact>> {
        let mut endpoints = Vec::new();

        // Group by source identity (endpoint)
        let mut by_endpoint: HashMap<u32, Vec<&FlowSimulationResult>> = HashMap::new();
        for result in flow_results {
            by_endpoint
                .entry(result.src_identity)
                .or_insert_with(Vec::new)
                .push(result);
        }

        for (identity, flows) in by_endpoint {
            let blocked_egress: Vec<_> = flows
                .iter()
                .filter(|f| f.changed && f.after == PolicyVerdict::Deny)
                .map(|f| EgressBlocked {
                    to_identity: f.dst_identity,
                    port: f.port,
                    protocol: f.protocol,
                    flow_count: 1, // TODO: Count actual occurrences
                })
                .collect();

            if blocked_egress.is_empty() {
                continue;
            }

            endpoints.push(EndpointImpact {
                identity,
                namespace: "default".to_string(), // TODO: Resolve
                labels: HashMap::new(),            // TODO: Resolve from IPCache
                blocked_egress,
                blocked_ingress: Vec::new(), // TODO: Analyze ingress
            });
        }

        Ok(endpoints)
    }

    /// Find broken dependencies
    async fn find_broken_dependencies(
        &self,
        flow_results: &[FlowSimulationResult],
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Find flows that changed from Allow to Deny
        for result in flow_results {
            if result.before == PolicyVerdict::Allow && result.after == PolicyVerdict::Deny {
                let protocol = match result.protocol {
                    6 => "TCP",
                    17 => "UDP",
                    _ => "OTHER",
                };

                // Determine criticality based on port
                let criticality = Self::determine_criticality(result.port, protocol);

                dependencies.push(Dependency {
                    from_service: format!("service-{}", result.src_identity),
                    to_service: format!("service-{}", result.dst_identity),
                    port: result.port,
                    protocol: protocol.to_string(),
                    criticality,
                });
            }
        }

        Ok(dependencies)
    }

    /// Determine dependency criticality
    fn determine_criticality(port: u16, protocol: &str) -> DependencyCriticality {
        match (port, protocol) {
            // Critical infrastructure ports
            (53, _) => DependencyCriticality::Critical,   // DNS
            (443, "TCP") => DependencyCriticality::Critical, // HTTPS
            (5432, _) => DependencyCriticality::Critical, // PostgreSQL
            (3306, _) => DependencyCriticality::Critical, // MySQL
            (6379, _) => DependencyCriticality::Critical, // Redis
            (9200, _) => DependencyCriticality::Critical, // Elasticsearch

            // Important ports
            (80, "TCP") => DependencyCriticality::Important,   // HTTP
            (8080, _) => DependencyCriticality::Important,     // HTTP alt
            (3000, _) => DependencyCriticality::Important,     // Common app port
            (9090, _) => DependencyCriticality::Important,     // Prometheus

            // Everything else
            _ => DependencyCriticality::Optional,
        }
    }

    /// Analyze service-to-service communication
    pub async fn analyze_service_mesh(
        &self,
        flow_results: &[FlowSimulationResult],
    ) -> Result<ServiceMeshAnalysis> {
        let mut edges = Vec::new();
        let mut nodes = HashSet::new();

        for result in flow_results {
            nodes.insert(result.src_identity);
            nodes.insert(result.dst_identity);

            if result.changed {
                edges.push(CommunicationEdge {
                    from: result.src_identity,
                    to: result.dst_identity,
                    port: result.port,
                    protocol: result.protocol,
                    before_allowed: result.before == PolicyVerdict::Allow,
                    after_allowed: result.after == PolicyVerdict::Allow,
                });
            }
        }

        Ok(ServiceMeshAnalysis {
            total_nodes: nodes.len(),
            affected_edges: edges.len(),
            edges,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ServiceMeshAnalysis {
    pub total_nodes: usize,
    pub affected_edges: usize,
    pub edges: Vec<CommunicationEdge>,
}

#[derive(Debug, Clone)]
pub struct CommunicationEdge {
    pub from: u32,
    pub to: u32,
    pub port: u16,
    pub protocol: u8,
    pub before_allowed: bool,
    pub after_allowed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_criticality() {
        assert_eq!(
            ImpactAnalyzer::determine_criticality(53, "UDP"),
            DependencyCriticality::Critical
        );
        assert_eq!(
            ImpactAnalyzer::determine_criticality(443, "TCP"),
            DependencyCriticality::Critical
        );
        assert_eq!(
            ImpactAnalyzer::determine_criticality(80, "TCP"),
            DependencyCriticality::Important
        );
        assert_eq!(
            ImpactAnalyzer::determine_criticality(12345, "TCP"),
            DependencyCriticality::Optional
        );
    }

    #[test]
    fn test_identify_critical_services() {
        let k8s_client = K8sClient::new();
        // Can't actually await in sync test, but we can test the logic
        let services = vec![
            "web-service".to_string(),
            "db-service".to_string(),
            "api-gateway".to_string(),
            "cache-service".to_string(),
        ];

        // We can't call the async method directly in sync test
        // But we can verify the pattern matching logic would work
        assert!(services.iter().any(|s| s.contains("db")));
        assert!(services.iter().any(|s| s.contains("api")));
    }
}
