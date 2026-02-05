/// Simulation Engine
///
/// Core policy simulation engine that evaluates flows against modified policies

use super::*;
use anyhow::Result;
use std::collections::HashMap;

pub struct SimulationEngine {
    /// Current policy decisions
    policies: Vec<PolicyDecision>,

    /// Simulated policy changes
    simulated_policies: Vec<PolicyDecision>,

    /// Execution trace for debugging
    trace: Vec<String>,
}

impl SimulationEngine {
    pub fn new(current_policies: Vec<PolicyDecision>) -> Self {
        Self {
            policies: current_policies.clone(),
            simulated_policies: current_policies,
            trace: Vec::new(),
        }
    }

    /// Apply a simulation scenario to policies
    pub fn apply_scenario(&mut self, scenario: &SimulationScenario) -> Result<()> {
        self.trace.push(format!("Applying scenario: {:?}", scenario));

        match scenario {
            SimulationScenario::AddPolicy { policy_yaml, namespace } => {
                self.add_policy(policy_yaml, namespace)?;
            }

            SimulationScenario::RemovePolicy { policy_name, namespace } => {
                self.remove_policy(policy_name, namespace)?;
            }

            SimulationScenario::ModifyPolicy { policy_name, namespace, new_yaml } => {
                self.remove_policy(policy_name, namespace)?;
                self.add_policy(new_yaml, namespace)?;
            }

            SimulationScenario::BlockTraffic {
                from_labels,
                to_labels,
                port,
                protocol,
            } => {
                self.block_traffic(from_labels, to_labels, port.as_ref(), protocol.as_ref())?;
            }

            SimulationScenario::AllowTraffic {
                from_labels,
                to_labels,
                port,
                protocol,
            } => {
                self.allow_traffic(from_labels, to_labels, *port, protocol)?;
            }

            SimulationScenario::BlockExternalIP { ip } => {
                self.block_external_ip(ip)?;
            }

            SimulationScenario::DefaultDeny { namespace } => {
                self.apply_default_deny(namespace)?;
            }
        }

        self.trace.push(format!("Scenario applied. Total policies: {}", self.simulated_policies.len()));

        Ok(())
    }

    /// Add a new policy (parsed from YAML)
    fn add_policy(&mut self, _policy_yaml: &str, namespace: &str) -> Result<()> {
        self.trace.push(format!("Adding policy in namespace: {}", namespace));

        // TODO: Parse YAML and extract policy rules
        // For now, this is a placeholder
        // In real implementation, parse CiliumNetworkPolicy YAML
        // and convert to PolicyDecision entries

        Ok(())
    }

    /// Remove a policy
    fn remove_policy(&mut self, policy_name: &str, namespace: &str) -> Result<()> {
        self.trace.push(format!("Removing policy: {}/{}", namespace, policy_name));

        // TODO: Remove policies associated with this policy name
        // For now, placeholder

        Ok(())
    }

    /// Block specific traffic
    fn block_traffic(
        &mut self,
        from_labels: &HashMap<String, String>,
        to_labels: &HashMap<String, String>,
        port: Option<&u16>,
        protocol: Option<&String>,
    ) -> Result<()> {
        self.trace.push(format!(
            "Blocking traffic: {:?} → {:?} port {:?} proto {:?}",
            from_labels, to_labels, port, protocol
        ));

        // Find matching policies and set to Deny
        // TODO: Resolve labels to identities
        let src_identity = Self::labels_to_identity(from_labels);
        let dst_identity = Self::labels_to_identity(to_labels);

        // Add deny rules
        if let Some(p) = port {
            let proto_opt = protocol.map(|s| s.clone());
            let proto = Self::protocol_to_number(&proto_opt);

            self.simulated_policies.push(PolicyDecision {
                src_identity,
                dst_identity,
                port: *p,
                protocol: proto,
                verdict: PolicyVerdict::Deny,
            });

            self.trace.push(format!(
                "Added DENY rule: {} → {} port {} proto {}",
                src_identity, dst_identity, p, proto
            ));
        } else {
            // Block all ports
            for p in [80, 443, 8080, 3000, 5432, 6379, 9200] {
                self.simulated_policies.push(PolicyDecision {
                    src_identity,
                    dst_identity,
                    port: p,
                    protocol: 6, // TCP
                    verdict: PolicyVerdict::Deny,
                });
            }
        }

        Ok(())
    }

    /// Allow specific traffic
    fn allow_traffic(
        &mut self,
        from_labels: &HashMap<String, String>,
        to_labels: &HashMap<String, String>,
        port: u16,
        protocol: &str,
    ) -> Result<()> {
        self.trace.push(format!(
            "Allowing traffic: {:?} → {:?} port {} proto {}",
            from_labels, to_labels, port, protocol
        ));

        let src_identity = Self::labels_to_identity(from_labels);
        let dst_identity = Self::labels_to_identity(to_labels);
        let proto = Self::protocol_to_number(&Some(protocol.to_string()));

        self.simulated_policies.push(PolicyDecision {
            src_identity,
            dst_identity,
            port,
            protocol: proto,
            verdict: PolicyVerdict::Allow,
        });

        Ok(())
    }

    /// Block external IP
    fn block_external_ip(&mut self, ip: &IpAddr) -> Result<()> {
        self.trace.push(format!("Blocking external IP: {}", ip));

        // TODO: Find all flows to this IP and add deny rules
        // For now, placeholder

        Ok(())
    }

    /// Apply default-deny to namespace
    fn apply_default_deny(&mut self, namespace: &str) -> Result<()> {
        self.trace.push(format!("Applying default-deny to namespace: {}", namespace));

        // Remove all wildcard allow rules
        self.simulated_policies.retain(|p| {
            !(p.dst_identity == 0 && p.verdict == PolicyVerdict::Allow)
        });

        self.trace.push(format!("Removed wildcard allow rules"));

        Ok(())
    }

    /// Evaluate a flow against simulated policies
    pub fn evaluate_flow(&self, flow: &HistoricalFlow) -> PolicyVerdict {
        // Find matching policy decision
        for decision in &self.simulated_policies {
            if self.matches_flow(decision, flow) {
                return decision.verdict.clone();
            }
        }

        // Default deny
        PolicyVerdict::Deny
    }

    /// Check if policy decision matches flow
    fn matches_flow(&self, decision: &PolicyDecision, flow: &HistoricalFlow) -> bool {
        // Exact match
        if decision.src_identity == flow.src_identity
            && decision.dst_identity == flow.dst_identity
            && decision.port == flow.port
            && decision.protocol == flow.protocol
        {
            return true;
        }

        // Wildcard matches
        if decision.src_identity == flow.src_identity
            && decision.dst_identity == 0 // wildcard destination
        {
            return true;
        }

        false
    }

    /// Get execution trace
    pub fn get_trace(&self) -> Vec<String> {
        self.trace.clone()
    }

    /// Convert labels to identity (placeholder)
    fn labels_to_identity(labels: &HashMap<String, String>) -> u32 {
        // TODO: Real implementation should query IPCache
        // For now, hash the labels
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Sort for consistent hashing
        let mut sorted_labels: Vec<_> = labels.iter().collect();
        sorted_labels.sort_by_key(|k| k.0);

        for (k, v) in sorted_labels {
            k.hash(&mut hasher);
            v.hash(&mut hasher);
        }

        (hasher.finish() % 10000) as u32
    }

    /// Convert protocol string to number
    fn protocol_to_number(protocol: &Option<String>) -> u8 {
        match protocol.as_ref().map(|s| s.as_str()) {
            Some("TCP") | Some("tcp") => 6,
            Some("UDP") | Some("udp") => 17,
            Some("ICMP") | Some("icmp") => 1,
            _ => 6, // Default to TCP
        }
    }

    /// Get policy count
    pub fn policy_count(&self) -> usize {
        self.simulated_policies.len()
    }

    /// Compare with original policies
    pub fn get_changes(&self) -> PolicyChanges {
        let added = self.simulated_policies.len().saturating_sub(self.policies.len());
        let removed = self.policies.len().saturating_sub(self.simulated_policies.len());

        PolicyChanges {
            added,
            removed,
            modified: 0, // TODO: Calculate actual modifications
            total_before: self.policies.len(),
            total_after: self.simulated_policies.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyChanges {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
    pub total_before: usize,
    pub total_after: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_engine_creation() {
        let policies = vec![
            PolicyDecision {
                src_identity: 100,
                dst_identity: 200,
                port: 80,
                protocol: 6,
                verdict: PolicyVerdict::Allow,
            },
        ];

        let engine = SimulationEngine::new(policies);
        assert_eq!(engine.policy_count(), 1);
    }

    #[test]
    fn test_block_traffic() {
        let policies = vec![];
        let mut engine = SimulationEngine::new(policies);

        let from_labels = HashMap::from([("app".to_string(), "web".to_string())]);
        let to_labels = HashMap::from([("app".to_string(), "db".to_string())]);

        engine.block_traffic(&from_labels, &to_labels, Some(&5432), Some(&"TCP".to_string())).unwrap();

        assert!(engine.policy_count() > 0);
    }

    #[test]
    fn test_evaluate_flow() {
        let policies = vec![
            PolicyDecision {
                src_identity: 100,
                dst_identity: 200,
                port: 80,
                protocol: 6,
                verdict: PolicyVerdict::Allow,
            },
        ];

        let engine = SimulationEngine::new(policies);

        let flow = HistoricalFlow {
            src_identity: 100,
            dst_identity: 200,
            src_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            dst_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
            port: 80,
            protocol: 6,
            timestamp: 123456,
            verdict: PolicyVerdict::Allow,
        };

        let verdict = engine.evaluate_flow(&flow);
        assert_eq!(verdict, PolicyVerdict::Allow);
    }

    #[test]
    fn test_protocol_conversion() {
        assert_eq!(SimulationEngine::protocol_to_number(&Some("TCP".to_string())), 6);
        assert_eq!(SimulationEngine::protocol_to_number(&Some("UDP".to_string())), 17);
        assert_eq!(SimulationEngine::protocol_to_number(&Some("ICMP".to_string())), 1);
    }
}
