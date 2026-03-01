#![allow(dead_code)]
// Traffic Shadowing - Mirror production traffic for testing
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::{ShadowConfig, ShadowStats};

/// Shadows production traffic to test environments
pub struct TrafficShadowing {
    active_shadows: RwLock<HashMap<String, Shadow>>,
}

struct Shadow {
    config: ShadowConfig,
    stats: ShadowStats,
}

impl TrafficShadowing {
    pub fn new() -> Result<Self> {
        Ok(Self {
            active_shadows: RwLock::new(HashMap::new()),
        })
    }

    pub async fn start(&mut self, config: ShadowConfig) -> Result<String> {
        // Validate config
        if config.source_service.is_empty() {
            anyhow::bail!("source_service cannot be empty");
        }
        if config.target_service.is_empty() {
            anyhow::bail!("target_service cannot be empty");
        }
        if config.sampling_rate <= 0.0 || config.sampling_rate > 1.0 {
            anyhow::bail!(
                "sampling_rate must be in range (0.0, 1.0], got {}",
                config.sampling_rate
            );
        }

        let shadow_id = uuid::Uuid::new_v4().to_string();

        tracing::warn!(
            shadow_id = %shadow_id,
            source = %config.source_service,
            target = %config.target_service,
            "Traffic shadowing is not yet implemented. Shadow session created \
             but no actual traffic mirroring will occur. In production: \
             create Envoy/Cilium L7 policy to mirror traffic, set up response \
             comparison, and configure sampling rate."
        );

        let shadow = Shadow {
            config: config.clone(),
            stats: ShadowStats {
                total_requests: 0,
                shadowed_requests: 0,
                success_rate: 0.0,
                error_rate: 0.0,
                latency_diff_ms: 0.0,
                discrepancies: vec![],
            },
        };

        self.active_shadows.write().await.insert(shadow_id.clone(), shadow);

        Ok(shadow_id)
    }

    pub async fn stop(&mut self, shadow_id: &str) -> Result<()> {
        let removed = self.active_shadows.write().await.remove(shadow_id);
        if removed.is_none() {
            anyhow::bail!("Shadow session not found: {}", shadow_id);
        }
        tracing::info!("Shadow stopped: {}", shadow_id);
        Ok(())
    }

    pub async fn get_stats(&self, shadow_id: &str) -> Result<ShadowStats> {
        let shadows = self.active_shadows.read().await;
        shadows
            .get(shadow_id)
            .map(|s| s.stats.clone())
            .ok_or_else(|| anyhow::anyhow!("Shadow session not found: {}", shadow_id))
    }
}

impl Default for TrafficShadowing {
    fn default() -> Self {
        Self {
            active_shadows: RwLock::new(HashMap::new()),
        }
    }
}
