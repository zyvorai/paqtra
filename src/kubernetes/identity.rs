#![allow(dead_code)]
/// Kubernetes Identity Resolution
///
/// Maps Cilium security identities to Kubernetes pod information

use anyhow::Result;
use k8s_openapi::api::core::v1::Pod;
use kube::{api::ListParams, Api, Client};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{Duration, interval};

use crate::ebpf::IdentityInfo;

/// Identity resolver with Kubernetes integration
pub struct K8sIdentityResolver {
    client: Client,
    cache: Arc<RwLock<IdentityCache>>,
}

/// Identity cache
struct IdentityCache {
    /// Identity → IdentityInfo mapping
    identities: HashMap<u32, IdentityInfo>,

    /// IP → Identity mapping
    ip_to_identity: HashMap<String, u32>,

    /// Pod name → Identity mapping
    pod_to_identity: HashMap<String, u32>,

    /// Last update timestamp
    last_update: std::time::Instant,
}

impl K8sIdentityResolver {
    /// Create a new identity resolver
    pub async fn new(client: Client) -> Result<Self> {
        let cache = Arc::new(RwLock::new(IdentityCache {
            identities: HashMap::new(),
            ip_to_identity: HashMap::new(),
            pod_to_identity: HashMap::new(),
            last_update: std::time::Instant::now(),
        }));

        Ok(Self { client, cache })
    }

    /// Refresh identity cache from Kubernetes
    pub async fn refresh(&self) -> Result<usize> {
        let pods: Api<Pod> = Api::all(self.client.clone());
        let lp = ListParams::default();

        let pod_list = pods.list(&lp).await?;

        let mut cache = self.cache.write().unwrap();
        let mut count = 0;

        for pod in pod_list.items {
            if let Some(identity) = Self::extract_identity(&pod) {
                let info = Self::pod_to_identity_info(&pod, identity);

                // Update identity cache
                cache.identities.insert(identity, info.clone());

                // Update IP mapping
                if let Some(pod_ip) = pod.status.as_ref()
                    .and_then(|s| s.pod_ip.as_ref()) {
                    cache.ip_to_identity.insert(pod_ip.clone(), identity);
                }

                // Update pod name mapping
                let pod_key = format!("{}/{}",
                    pod.metadata.namespace.as_deref().unwrap_or("default"),
                    pod.metadata.name.as_deref().unwrap_or("unknown")
                );
                cache.pod_to_identity.insert(pod_key, identity);

                count += 1;
            }
        }

        cache.last_update = std::time::Instant::now();

        Ok(count)
    }

    /// Extract Cilium identity from pod
    fn extract_identity(pod: &Pod) -> Option<u32> {
        // Try to get identity from pod labels
        // Cilium adds labels like: security.cilium.io/identity=100
        pod.metadata.labels.as_ref()
            .and_then(|labels| labels.get("security.cilium.io/identity"))
            .and_then(|s| s.parse::<u32>().ok())
            .or_else(|| {
                // Fallback: try annotations
                pod.metadata.annotations.as_ref()
                    .and_then(|annot| annot.get("cilium.io/identity"))
                    .and_then(|s| s.parse::<u32>().ok())
            })
    }

    /// Convert Pod to IdentityInfo
    fn pod_to_identity_info(pod: &Pod, identity: u32) -> IdentityInfo {
        let namespace = pod.metadata.namespace.clone()
            .unwrap_or_else(|| "default".to_string());

        let pod_name = pod.metadata.name.clone()
            .unwrap_or_else(|| "unknown".to_string());

        // Extract labels
        let labels = pod.metadata.labels.as_ref()
            .map(|labels_map| {
                labels_map.iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        IdentityInfo {
            identity,
            labels,
            namespace,
            pod_name,
        }
    }

    /// Resolve identity to pod information
    pub fn resolve_identity(&self, identity: u32) -> Option<IdentityInfo> {
        self.cache.read().unwrap()
            .identities.get(&identity)
            .cloned()
    }

    /// Resolve IP to identity
    pub fn resolve_ip(&self, ip: &str) -> Option<u32> {
        self.cache.read().unwrap()
            .ip_to_identity.get(ip)
            .copied()
    }

    /// Resolve pod name to identity
    pub fn resolve_pod(&self, namespace: &str, pod_name: &str) -> Option<u32> {
        let key = format!("{}/{}", namespace, pod_name);
        self.cache.read().unwrap()
            .pod_to_identity.get(&key)
            .copied()
    }

    /// Get all identities
    pub fn get_all_identities(&self) -> Vec<IdentityInfo> {
        self.cache.read().unwrap()
            .identities.values()
            .cloned()
            .collect()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read().unwrap();
        CacheStats {
            total_identities: cache.identities.len(),
            total_ips: cache.ip_to_identity.len(),
            total_pods: cache.pod_to_identity.len(),
            age_seconds: cache.last_update.elapsed().as_secs(),
        }
    }

    /// Start background refresh task
    pub fn start_refresh_task(self: Arc<Self>, interval_secs: u64) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            loop {
                ticker.tick().await;

                match self.refresh().await {
                    Ok(count) => {
                        tracing::debug!("Identity cache refreshed: {} identities", count);
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh identity cache: {}", e);
                    }
                }
            }
        });
    }

    /// Enrich IP cache entry with pod information
    pub fn enrich_ipcache_entry(&self, entry: &mut crate::ebpf::IPCacheEntry) {
        if let Some(info) = self.resolve_identity(entry.identity) {
            entry.namespace = info.namespace;
            entry.labels = info.labels;
        }
    }

    /// Enrich connection tracking entry with pod information
    pub fn enrich_ct_entry(&self, entry: &mut crate::ebpf::ConntrackEntry) {
        // For CT entries, we need to resolve IPs first, then identities
        // This is simplified - in production would need proper IP → identity → pod chain

        // Try to resolve source IP
        if let Some(src_identity) = self.resolve_ip(&entry.src_ip) {
            if let Some(info) = self.resolve_identity(src_identity) {
                // Could add fields to ConntrackEntry to store this info
                tracing::trace!("Source: {}/{}", info.namespace, info.pod_name);
            }
        }

        // Try to resolve destination IP
        if let Some(dst_identity) = self.resolve_ip(&entry.dst_ip) {
            if let Some(info) = self.resolve_identity(dst_identity) {
                tracing::trace!("Destination: {}/{}", info.namespace, info.pod_name);
            }
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_identities: usize,
    pub total_ips: usize,
    pub total_pods: usize,
    pub age_seconds: u64,
}

/// Helper: Extract common labels from Kubernetes pod
pub fn extract_common_labels(pod: &Pod) -> HashMap<String, String> {
    let mut labels = HashMap::new();

    if let Some(pod_labels) = &pod.metadata.labels {
        // Extract common label keys
        for key in &["app", "app.kubernetes.io/name", "component", "tier", "version"] {
            if let Some(value) = pod_labels.get(*key) {
                labels.insert(key.to_string(), value.clone());
            }
        }
    }

    labels
}

/// Helper: Derive service name from pod
pub fn derive_service_name(pod: &Pod) -> String {
    pod.metadata.labels.as_ref()
        .and_then(|labels| {
            labels.get("app")
                .or_else(|| labels.get("app.kubernetes.io/name"))
        })
        .cloned()
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_cache_stats() {
        let stats = CacheStats {
            total_identities: 100,
            total_ips: 150,
            total_pods: 100,
            age_seconds: 60,
        };

        assert_eq!(stats.total_identities, 100);
        assert_eq!(stats.total_ips, 150);
        assert_eq!(stats.age_seconds, 60);
    }

    #[test]
    fn test_extract_common_labels() {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "web".to_string());
        labels.insert("tier".to_string(), "frontend".to_string());
        labels.insert("other".to_string(), "value".to_string());

        let pod = Pod {
            metadata: ObjectMeta {
                labels: Some(labels),
                ..Default::default()
            },
            ..Default::default()
        };

        let common = extract_common_labels(&pod);

        assert_eq!(common.get("app"), Some(&"web".to_string()));
        assert_eq!(common.get("tier"), Some(&"frontend".to_string()));
        assert_eq!(common.get("other"), None); // Not a common label
    }

    #[test]
    fn test_derive_service_name() {
        use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "nginx".to_string());

        let pod = Pod {
            metadata: ObjectMeta {
                labels: Some(labels),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(derive_service_name(&pod), "nginx");
    }
}
