/// DNS-specific healing logic

use super::*;

pub struct DNSHealer;

impl DNSHealer {
    pub fn detect_dns_issues(drops: &[DropReason]) -> Vec<Problem> {
        let mut problems = Vec::new();

        // Group DNS drops by source
        let mut dns_drops_by_source: HashMap<String, u64> = HashMap::new();

        for drop in drops {
            if drop.port == 53 {
                *dns_drops_by_source
                    .entry(drop.src_ip.clone())
                    .or_insert(0) += 1;
            }
        }

        // Create problems for sources with many DNS drops
        for (src_ip, count) in dns_drops_by_source {
            if count >= 3 {
                // Threshold
                problems.push(Problem::DNSDrops {
                    namespace: "default".to_string(), // TODO: lookup actual namespace
                    pod: src_ip,
                    count,
                });
            }
        }

        problems
    }

    pub fn generate_dns_fix(namespace: &str) -> Fix {
        Fix {
            problem: Problem::DNSDrops {
                namespace: namespace.to_string(),
                pod: "unknown".to_string(),
                count: 0,
            },
            action: FixAction::CreateDNSPolicy {
                namespace: namespace.to_string(),
            },
            applied: false,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}
