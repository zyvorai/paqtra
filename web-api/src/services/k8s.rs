// Kubernetes service client
//
// This service wraps Kubernetes API interactions for the web API.
// Uses kubectl as a subprocess for policy management.

use crate::models::policy::{CreatePolicyRequest, Policy};
use anyhow::{Context, Result};
use regex::Regex;
use std::sync::LazyLock;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

/// Kubernetes resource name validation: RFC 1123 DNS subdomain
static K8S_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9][a-z0-9.\-]{0,252}$").unwrap());

/// Validate a Kubernetes resource name
fn validate_k8s_name(name: &str, field: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("{} must not be empty", field);
    }
    if !K8S_NAME_RE.is_match(name) {
        anyhow::bail!(
            "{} '{}' is not a valid Kubernetes name (must match RFC 1123 DNS subdomain)",
            field,
            name
        );
    }
    // Reject names that look like kubectl flags
    if name.starts_with('-') {
        anyhow::bail!("{} must not start with '-'", field);
    }
    Ok(())
}

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

    /// Build a kubectl command with the configured context.
    fn kubectl(&self, args: &[&str]) -> Command {
        let mut cmd = Command::new("kubectl");
        for arg in args {
            cmd.arg(arg);
        }
        if let Some(ctx) = &self.context {
            cmd.arg("--context").arg(ctx);
        }
        cmd
    }

    /// Check if Kubernetes API is reachable
    pub async fn is_healthy(&self) -> bool {
        let mut cmd = self.kubectl(&["cluster-info", "--request-timeout=2s"]);
        match timeout(Duration::from_secs(30), cmd.output()).await {
            Ok(Ok(out)) => out.status.success(),
            _ => false,
        }
    }

    /// Run a kubectl command and return the parsed JSON output.
    /// Returns an empty JSON object `{}` on failure.
    pub async fn kubectl_json(&self, args: &[&str]) -> serde_json::Value {
        let mut cmd = self.kubectl(args);
        match timeout(Duration::from_secs(15), cmd.output()).await {
            Ok(Ok(out)) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}))
            }
            Ok(Ok(out)) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::debug!("kubectl {:?} failed: {}", args.first(), stderr.trim());
                serde_json::json!({})
            }
            _ => serde_json::json!({}),
        }
    }

    /// Run an arbitrary CLI command and return stdout as a string.
    pub async fn run_cmd(program: &str, args: &[&str]) -> String {
        let mut cmd = Command::new(program);
        for a in args {
            cmd.arg(a);
        }
        match timeout(Duration::from_secs(10), cmd.output()).await {
            Ok(Ok(out)) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            }
            _ => String::new(),
        }
    }

    /// Run a CLI command with data piped to stdin. Returns (success, stdout, stderr).
    pub async fn run_cmd_stdin(
        program: &str,
        args: &[&str],
        stdin_data: &str,
    ) -> (bool, String, String) {
        use tokio::io::AsyncWriteExt;
        let mut cmd = Command::new(program);
        for a in args {
            cmd.arg(a);
        }
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(_) => return (false, String::new(), "Failed to spawn process".to_string()),
        };

        // We need to write to stdin before waiting, so split into parts
        let mut child = child;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(stdin_data.as_bytes()).await;
            drop(stdin);
        }

        match timeout(Duration::from_secs(10), child.wait_with_output()).await {
            Ok(Ok(out)) => {
                let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                (out.status.success(), stdout, stderr)
            }
            _ => (false, String::new(), "Command timed out".to_string()),
        }
    }

    /// List CiliumNetworkPolicy resources across all namespaces
    pub async fn list_policies(&self) -> Result<Vec<Policy>> {
        let mut cmd = self.kubectl(&[
            "get",
            "ciliumnetworkpolicies",
            "--all-namespaces",
            "-o",
            "json",
        ]);

        let output = timeout(Duration::from_secs(30), cmd.output())
            .await
            .context("kubectl list policies timed out after 30 seconds")?;

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let list: serde_json::Value =
                    serde_json::from_str(&stdout).context("Failed to parse kubectl JSON output")?;

                let policies = list
                    .get("items")
                    .and_then(|items| items.as_array())
                    .map(|items| items.iter().map(k8s_resource_to_policy).collect())
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
        validate_k8s_name(&req.name, "policy name")?;
        validate_k8s_name(&req.namespace, "namespace")?;

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

        let mut cmd = self.kubectl(&["apply", "-f", "-"]);

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

        let output = timeout(Duration::from_secs(30), child.wait_with_output())
            .await
            .context("kubectl apply timed out after 30 seconds")??;

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

    /// Revert a Deployment to its previous revision (`kubectl rollout undo`).
    /// With `dry_run` the API server validates the request without changing
    /// anything. Returns kubectl's output.
    pub async fn rollout_undo(&self, namespace: &str, name: &str, dry_run: bool) -> Result<String> {
        validate_k8s_name(name, "deployment name")?;
        validate_k8s_name(namespace, "namespace")?;

        let target = format!("deployment/{}", name);
        let mut args = vec!["rollout", "undo", target.as_str(), "-n", namespace];
        if dry_run {
            args.push("--dry-run=server");
        }
        let mut cmd = self.kubectl(&args);

        let output = timeout(Duration::from_secs(30), cmd.output())
            .await
            .context("kubectl rollout undo timed out after 30 seconds")?
            .context("kubectl not available")?;
        if !output.status.success() {
            anyhow::bail!(
                "kubectl rollout undo failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Delete a CiliumNetworkPolicy by name (id is treated as "namespace/name")
    pub async fn delete_policy(&self, id: &str) -> Result<()> {
        // id format: "namespace/name" or just "name" (default namespace)
        let (namespace, name) = if let Some(pos) = id.find('/') {
            (&id[..pos], &id[pos + 1..])
        } else {
            ("default", id)
        };

        validate_k8s_name(name, "policy name")?;
        validate_k8s_name(namespace, "namespace")?;

        let mut cmd = self.kubectl(&["delete", "ciliumnetworkpolicy", name, "-n", namespace]);

        let output = timeout(Duration::from_secs(30), cmd.output())
            .await
            .context("kubectl delete timed out after 30 seconds")?;

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

/// Extract a string field from a JSON value, returning `fallback` if absent.
fn json_str(v: &serde_json::Value, key: &str, fallback: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or(fallback)
        .to_string()
}

/// Convert a raw Kubernetes resource JSON into our Policy model
fn k8s_resource_to_policy(item: &serde_json::Value) -> Policy {
    let metadata = item.get("metadata").unwrap_or(item);

    let name = json_str(metadata, "name", "unknown");
    let namespace = json_str(metadata, "namespace", "default");
    let created_at = json_str(metadata, "creationTimestamp", "");
    let uid = json_str(metadata, "uid", "");

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
