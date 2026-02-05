use anyhow::Result;
use crate::kubernetes::K8sClient;

#[cfg(test)]
mod tests;

pub struct PolicyManager {
    k8s_client: K8sClient,
}

impl PolicyManager {
    pub fn new(k8s_client: K8sClient) -> Self {
        Self { k8s_client }
    }

    pub async fn apply_intra_namespace_policy(&self, namespace: &str) -> Result<()> {
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

        self.k8s_client.apply_custom_resource(Some(namespace), &policy).await?;
        Ok(())
    }

    pub async fn apply_dns_policy(&self, namespace: &str) -> Result<()> {
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
    - toEndpoints:
        - matchLabels:
            k8s-app: kube-dns
      toPorts:
        - ports:
            - port: "53"
              protocol: TCP
"#
        );

        self.k8s_client.apply_custom_resource(Some(namespace), &policy).await?;
        Ok(())
    }

    pub async fn apply_hubble_policy(&self) -> Result<()> {
        let policy = r#"
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-hubble
  namespace: kube-system
spec:
  endpointSelector:
    matchLabels:
      k8s-app: cilium
  ingress:
    - fromEndpoints:
        - matchLabels:
            app: cilium-tui
    - fromEndpoints:
        - {}
"#;

        self.k8s_client.apply_custom_resource(Some("kube-system"), policy).await?;
        Ok(())
    }

    pub async fn apply_best_practice_policy(&self, namespace: &str, from_app: &str, to_app: &str, port: u16) -> Result<()> {
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

        self.k8s_client.apply_custom_resource(Some(namespace), &policy).await?;
        Ok(())
    }
}
