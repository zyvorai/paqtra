// Hubble service client
//
// This service wraps Hubble gRPC/CLI interactions for the web API.
// It attempts a gRPC connection first, then falls back to CLI invocation.

use crate::models::flow::{Flow, FlowEndpoint, FlowStats};
use anyhow::{Context, Result};
use tokio::process::Command;

pub struct HubbleService {
    address: String,
}

impl HubbleService {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    /// Check if Hubble relay is reachable via TCP
    pub async fn is_healthy(&self) -> bool {
        tokio::net::TcpStream::connect(&self.address).await.is_ok()
    }

    /// Retrieve flows from Hubble. Attempts gRPC first, then falls back to CLI.
    pub async fn get_flows(
        &self,
        limit: usize,
        namespace: Option<&str>,
    ) -> Result<Vec<Flow>> {
        // Try gRPC connection first
        match self.get_flows_grpc(limit, namespace).await {
            Ok(flows) => {
                tracing::debug!("Retrieved {} flows via gRPC", flows.len());
                return Ok(flows);
            }
            Err(e) => {
                tracing::warn!(
                    "gRPC flow retrieval failed, falling back to CLI: {}",
                    e
                );
            }
        }

        // Fall back to CLI
        self.get_flows_cli(limit, namespace).await
    }

    /// Attempt to get flows via gRPC (stub -- real impl would use tonic)
    async fn get_flows_grpc(
        &self,
        _limit: usize,
        _namespace: Option<&str>,
    ) -> Result<Vec<Flow>> {
        // Verify connectivity first
        tokio::net::TcpStream::connect(&self.address)
            .await
            .context("Cannot connect to Hubble relay via gRPC")?;

        // In a real implementation this would use a tonic-generated Hubble
        // Observer client. For now we return an error so the CLI fallback
        // is exercised, unless Hubble is genuinely reachable.
        anyhow::bail!("gRPC client not yet implemented -- falling back to CLI")
    }

    /// Fall back to the `hubble` CLI binary
    async fn get_flows_cli(
        &self,
        limit: usize,
        namespace: Option<&str>,
    ) -> Result<Vec<Flow>> {
        let mut cmd = Command::new("hubble");
        cmd.arg("observe")
            .arg("--output")
            .arg("json")
            .arg("--last")
            .arg(limit.to_string())
            .arg("--server")
            .arg(&self.address);

        if let Some(ns) = namespace {
            cmd.arg("--namespace").arg(ns);
        }

        let output = cmd.output().await;

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let flows: Vec<Flow> = stdout
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
                    .enumerate()
                    .map(|(i, v)| hubble_json_to_flow(i, &v))
                    .collect();
                Ok(flows)
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                tracing::debug!("hubble CLI returned non-zero: {}", stderr);
                // Return empty list rather than error -- Hubble may not be installed
                Ok(Vec::new())
            }
            Err(e) => {
                tracing::debug!("hubble CLI not available: {}", e);
                // Return empty list -- Hubble CLI not installed
                Ok(Vec::new())
            }
        }
    }

    /// Compute aggregate flow statistics
    pub async fn get_flow_stats(&self) -> Result<FlowStats> {
        let flows = self.get_flows(1000, None).await?;

        let total = flows.len() as u64;
        let forwarded = flows.iter().filter(|f| f.verdict == "FORWARDED").count() as u64;
        let dropped = flows.iter().filter(|f| f.verdict == "DROPPED").count() as u64;

        Ok(FlowStats {
            total_flows: total,
            forwarded,
            dropped,
            requests_per_second: 0.0, // Would need time-window sampling
            avg_latency_ms: 0.0,      // Would need L7 data
        })
    }
}

/// Convert a raw Hubble JSON object into our canonical Flow struct
fn hubble_json_to_flow(index: usize, v: &serde_json::Value) -> Flow {
    let src = v.get("source").unwrap_or(v);
    let dst = v.get("destination").unwrap_or(v);

    Flow {
        id: v
            .get("uuid")
            .or_else(|| v.get("id"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("flow-{}", index)),
        timestamp: v
            .get("time")
            .or_else(|| v.get("timestamp"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string(),
        source: FlowEndpoint {
            namespace: json_str(src, "namespace"),
            pod: json_str(src, "pod_name").or_else(|| json_str(src, "pod")),
            ip: json_str(src, "ip").or_else(|| json_str(src, "IP")),
        },
        destination: FlowEndpoint {
            namespace: json_str(dst, "namespace"),
            pod: json_str(dst, "pod_name").or_else(|| json_str(dst, "pod")),
            ip: json_str(dst, "ip").or_else(|| json_str(dst, "IP")),
        },
        verdict: v
            .get("verdict")
            .and_then(|x| x.as_str())
            .unwrap_or("UNKNOWN")
            .to_string(),
        protocol: v
            .get("l4")
            .and_then(|l4| {
                if l4.get("TCP").is_some() {
                    Some("TCP")
                } else if l4.get("UDP").is_some() {
                    Some("UDP")
                } else if l4.get("ICMPv4").is_some() {
                    Some("ICMPv4")
                } else {
                    None
                }
            })
            .unwrap_or("UNKNOWN")
            .to_string(),
        port: v
            .get("l4")
            .and_then(|l4| {
                l4.get("TCP")
                    .or_else(|| l4.get("UDP"))
                    .and_then(|proto| proto.get("destination_port"))
                    .and_then(|p| p.as_u64())
            })
            .unwrap_or(0) as u16,
    }
}

/// Helper: extract a string field from a JSON value, returning empty string if absent
fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

/// Extension trait so we can chain .or_else on String (not Option)
trait StringOrElse {
    fn or_else(self, f: impl FnOnce() -> String) -> String;
}

impl StringOrElse for String {
    fn or_else(self, f: impl FnOnce() -> String) -> String {
        if self.is_empty() {
            f()
        } else {
            self
        }
    }
}
