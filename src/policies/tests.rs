#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_intra_namespace_policy_generation() {
        let namespace = "test-ns";
        let policy = format!(
            r#"
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-intra-namespace
  namespace: {namespace}
spec:
  endpointSelector: {{}}
  ingress:
    - fromEndpoints:
        - {{}}
"#
        );

        assert!(policy.contains("test-ns"));
        assert!(policy.contains("allow-intra-namespace"));
        assert!(policy.contains("CiliumNetworkPolicy"));
    }

    #[test]
    fn test_dns_policy_generation() {
        let namespace = "test-ns";
        let policy = format!(
            r#"
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-dns
  namespace: {namespace}
spec:
  endpointSelector: {{}}
  egress:
    - toEndpoints:
        - matchLabels:
            k8s-app: kube-dns
      toPorts:
        - ports:
            - port: "53"
              protocol: UDP
"#
        );

        assert!(policy.contains("test-ns"));
        assert!(policy.contains("allow-dns"));
        assert!(policy.contains("kube-dns"));
        assert!(policy.contains("53"));
    }

    #[test]
    fn test_best_practice_policy_generation() {
        let namespace = "prod";
        let from_app = "web";
        let to_app = "db";
        let port = 5432;

        let policy = format!(
            r#"
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-{from_app}-to-{to_app}
  namespace: {namespace}
spec:
  endpointSelector:
    matchLabels:
      app: {to_app}
  ingress:
    - fromEndpoints:
        - matchLabels:
            app: {from_app}
      toPorts:
        - ports:
            - port: "{port}"
              protocol: TCP
"#
        );

        assert!(policy.contains("prod"));
        assert!(policy.contains("allow-web-to-db"));
        assert!(policy.contains("app: web"));
        assert!(policy.contains("app: db"));
        assert!(policy.contains("5432"));
    }
}
