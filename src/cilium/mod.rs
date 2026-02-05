use anyhow::{Result, Context};
use k8s_openapi::api::core::v1::{ConfigMap, ServiceAccount};
use k8s_openapi::api::rbac::v1::{ClusterRoleBinding, RoleRef, Subject};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::collections::BTreeMap;
use std::process::Command;

use crate::kubernetes::K8sClient;

pub struct CiliumManager {
    k8s_client: K8sClient,
}

#[derive(Debug, Clone)]
pub struct CiliumStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub hubble_enabled: bool,
}

impl CiliumManager {
    pub fn new(k8s_client: K8sClient) -> Self {
        Self { k8s_client }
    }

    pub async fn is_installed(&self) -> Result<bool> {
        let pods = self.k8s_client
            .get_pods_by_label("kube-system", "k8s-app=cilium")
            .await?;

        Ok(!pods.is_empty())
    }

    pub async fn get_status(&self) -> Result<CiliumStatus> {
        let installed = self.is_installed().await?;

        if !installed {
            return Ok(CiliumStatus {
                installed: false,
                version: None,
                hubble_enabled: false,
            });
        }

        // Get version from cilium CLI if available
        let version = self.get_cilium_version().ok();

        // Check if Hubble is enabled by looking for hubble-relay
        let hubble_pods = self.k8s_client
            .get_pods_by_label("kube-system", "k8s-app=hubble-relay")
            .await
            .unwrap_or_default();

        Ok(CiliumStatus {
            installed: true,
            version,
            hubble_enabled: !hubble_pods.is_empty(),
        })
    }

    fn get_cilium_version(&self) -> Result<String> {
        let output = Command::new("cilium")
            .args(["version", "--client"])
            .output()
            .context("Failed to get Cilium version")?;

        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            // Extract version from output
            for line in version_str.lines() {
                if line.contains("cilium-cli") {
                    if let Some(version) = line.split_whitespace().last() {
                        return Ok(version.to_string());
                    }
                }
            }
        }

        Ok("unknown".to_string())
    }

    pub async fn install(&self, auto_approve: bool) -> Result<()> {
        println!("🔍 Cilium not detected in the cluster.");

        // Check if cilium CLI is available
        if !self.is_cilium_cli_available() {
            anyhow::bail!(
                "❌ Cilium CLI not found. Please install it first:\n\
                 \n\
                 Linux:   curl -L --remote-name-all https://github.com/cilium/cilium-cli/releases/latest/download/cilium-linux-amd64.tar.gz\n\
                 macOS:   brew install cilium-cli\n\
                 \n\
                 Or visit: https://docs.cilium.io/en/stable/gettingstarted/k8s-install-default/"
            );
        }

        if !auto_approve {
            println!("\n❓ Would you like to install Cilium now? (y/N)");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                anyhow::bail!("Cilium installation cancelled. Please install Cilium manually:\n  cilium install");
            }
        }

        println!("📦 Installing Cilium...");

        let output = Command::new("cilium")
            .args(["install"])
            .status()
            .context("Failed to execute cilium install")?;

        if !output.success() {
            anyhow::bail!("Failed to install Cilium. Please check the error messages above.");
        }

        println!("⏳ Waiting for Cilium to be ready...");

        let status_output = Command::new("cilium")
            .args(["status", "--wait"])
            .status()
            .context("Failed to wait for Cilium status")?;

        if !status_output.success() {
            anyhow::bail!("Cilium installation completed but status check failed.");
        }

        println!("✅ Cilium installed successfully!");
        Ok(())
    }

    pub async fn upgrade(&self) -> Result<()> {
        println!("🔄 Upgrading Cilium...");

        let output = Command::new("cilium")
            .args(["upgrade"])
            .status()
            .context("Failed to execute cilium upgrade")?;

        if !output.success() {
            anyhow::bail!("Failed to upgrade Cilium");
        }

        println!("✅ Cilium upgraded successfully!");
        Ok(())
    }

    fn is_cilium_cli_available(&self) -> bool {
        Command::new("cilium")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub async fn enable_features(&self) -> Result<()> {
        let mut data = BTreeMap::new();
        data.insert("enable-hubble".to_string(), "true".to_string());
        data.insert("hubble-metrics-enabled".to_string(), "true".to_string());
        data.insert("hubble-listen-address".to_string(), ":4244".to_string());
        data.insert("hubble-relay-enabled".to_string(), "true".to_string());
        data.insert("monitor-aggregation".to_string(), "medium".to_string());
        data.insert("enable-l7-proxy".to_string(), "true".to_string());

        let configmap = ConfigMap {
            metadata: ObjectMeta {
                name: Some("cilium-config".to_string()),
                namespace: Some("kube-system".to_string()),
                ..Default::default()
            },
            data: Some(data),
            ..Default::default()
        };

        self.k8s_client
            .create_or_update_configmap("kube-system", configmap)
            .await?;

        Ok(())
    }

    pub async fn create_tui_service_account(&self) -> Result<()> {
        // Create ServiceAccount
        let sa = ServiceAccount {
            metadata: ObjectMeta {
                name: Some("cilium-tui".to_string()),
                namespace: Some("kube-system".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        self.k8s_client
            .create_or_update_service_account("kube-system", sa)
            .await?;

        // Create ClusterRoleBinding
        let crb = ClusterRoleBinding {
            metadata: ObjectMeta {
                name: Some("cilium-tui-binding".to_string()),
                ..Default::default()
            },
            subjects: Some(vec![Subject {
                kind: "ServiceAccount".to_string(),
                name: "cilium-tui".to_string(),
                namespace: Some("kube-system".to_string()),
                ..Default::default()
            }]),
            role_ref: RoleRef {
                api_group: "rbac.authorization.k8s.io".to_string(),
                kind: "ClusterRole".to_string(),
                name: "cluster-admin".to_string(),
            },
        };

        self.k8s_client
            .create_or_update_cluster_role_binding(crb)
            .await?;

        Ok(())
    }
}
