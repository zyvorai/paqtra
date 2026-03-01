// Kubernetes service client
//
// This service wraps Kubernetes API interactions for the web API.
// Uses kubectl as a subprocess for policy management.

use crate::models::policy::{CreatePolicyRequest, Policy};
use anyhow::{Context, Result};
use tokio::process::Command;

pub struct K8sService {
    context: Option<String>,
}

impl K8sService {
    pub fn new(context: Option<String>) -> Self {
        Self { context }
    }

    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }

    /// Check if Kubernetes API is reachable
    pub async fn is_healthy(&self) -> bool {
        let mut cmd = Command::new("kubectl");
        cmd.arg("cluster-info").arg("--request-timeout=2s");
        if let Some(ctx) = &self.context {
            cmd.arg("--context").arg(ctx);
        }
        match cmd.output().await {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// List CiliumNetworkPolicy resources across all namespaces
    pub async fn list_policies(&self) -> Result<Vec<Policy>> {
        let mut cmd = Command::new("kubectl");
        cmd.arg("get")
            .arg("ciliumnetworkpolicies")
            .arg("--all-namespaces")
            .arg("-o")
            .arg("json");
        if let Some(ctx) = &self.context {
            cmd.arg("--context").arg(ctx);
        }

        let output = cmd.output().await;

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let list: serde_json::Value = serde_json::from_str(&stdout)
                    .context("Failed to parse kubectl JSON output")?;

                let policies = list
                    .get("items")
                    .and_then(|items| items.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| k8s_resource_to_policy(item))
                            .collect()
                    })
                    .unwrap_or_default();

                Ok(policies)
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::debug!("kubectl list policies failed: {}", stderr);
                Ok(Vec::new())
            }
            Err(e) => {
                tracing::debug!("kubectl not available: {}", e);
                Ok(Vec::new())
            }
        }
    }

    /// Create a CiliumNetworkPolicy from a request
    pub async fn create_policy(&self, req: &CreatePolicyRequest) -> Result<Policy> {
        let policy_manifest = serde_json::json!({
            "apiVersion": "cilium.io/v2",
            "kind": "CiliumNetworkPolicy",
            "metadata": {
                "name": req.name,
                "namespace": req.namespace,
            },
            "spec": req.spec,
        });

        let manifest_str = serde_json::to_string(&policy_manifest)?;

        let mut cmd = Command::new("kubectl");
        cmd.arg("apply").arg("-f").arg("-");
        if let Some(ctx) = &self.context {
            cmd.arg("--context").arg(ctx);
        }

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn kubectl")?;

        // Write manifest to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(manifest_str.as_bytes()).await?;
            // drop stdin to close it
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("kubectl apply failed: {}", stderr);
        }

        Ok(Policy {
            id: uuid::Uuid::new_v4().to_string(),
            name: req.name.clone(),
            namespace: req.namespace.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            status: "created".to_string(),
        })
    }

    /// Delete a CiliumNetworkPolicy by name (id is treated as "namespace/name")
    pub async fn delete_policy(&self, id: &str) -> Result<()> {
        // id format: "namespace/name" or just "name" (default namespace)
        let (namespace, name) = if let Some(pos) = id.find('/') {
            (&id[..pos], &id[pos + 1..])
        } else {
            ("default", id)
        };

        let mut cmd = Command::new("kubectl");
        cmd.arg("delete")
            .arg("ciliumnetworkpolicy")
            .arg(name)
            .arg("-n")
            .arg(namespace);
        if let Some(ctx) = &self.context {
            cmd.arg("--context").arg(ctx);
        }

        let output = cmd.output().await;

        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                anyhow::bail!("kubectl delete failed: {}", stderr)
            }
            Err(e) => anyhow::bail!("kubectl not available: {}", e),
        }
    }
}

/// Convert a raw Kubernetes resource JSON into our Policy model
fn k8s_resource_to_policy(item: &serde_json::Value) -> Policy {
    let metadata = item.get("metadata").unwrap_or(item);

    let name = metadata
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let namespace = metadata
        .get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();

    let created_at = metadata
        .get("creationTimestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let uid = metadata
        .get("uid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let status = item
        .get("status")
        .and_then(|s| s.get("phase"))
        .and_then(|v| v.as_str())
        .unwrap_or("active")
        .to_string();

    Policy {
        id: uid,
        name,
        namespace,
        created_at,
        status,
    }
}
