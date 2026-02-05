/// Policy-specific healing logic

use super::*;

pub struct PolicyHealer;

impl PolicyHealer {
    pub fn detect_policy_gaps(drops: &[DropReason]) -> Vec<Problem> {
        let mut problems = Vec::new();

        for drop in drops {
            if drop.reason == DropReasonType::PolicyDenied {
                problems.push(Problem::PolicyGap {
                    src_namespace: "unknown".to_string(),
                    src_pod: drop.src_ip.clone(),
                    dst_namespace: "unknown".to_string(),
                    dst_pod: drop.dst_ip.clone(),
                    port: drop.port,
                    protocol: protocol_name(drop.protocol),
                });
            }
        }

        problems
    }

    pub fn suggest_policy(
        src: &str,
        dst: &str,
        port: u16,
        protocol: &str,
    ) -> String {
        format!(
            r#"
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: auto-allow-{}-to-{}
spec:
  endpointSelector:
    matchLabels:
      app: {}
  egress:
    - toEndpoints:
        - matchLabels:
            app: {}
      toPorts:
        - ports:
            - port: "{}"
              protocol: {}
"#,
            src, dst, src, dst, port, protocol
        )
    }
}

fn protocol_name(proto_num: u8) -> String {
    match proto_num {
        6 => "TCP".to_string(),
        17 => "UDP".to_string(),
        _ => format!("{}", proto_num),
    }
}
