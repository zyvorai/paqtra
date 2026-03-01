// Alert Management - Intelligent alerting with noise reduction
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::{Anomaly, Severity};

/// Manages alerts with intelligent deduplication and noise reduction
pub struct AlertManager {
    confidence_threshold: f64,
    alert_history: RwLock<Vec<Anomaly>>,
    suppression_rules: HashMap<String, SuppressionRule>,
}

#[derive(Debug, Clone)]
struct SuppressionRule {
    window_minutes: u64,
    max_alerts: usize,
}

impl AlertManager {
    pub fn new(confidence_threshold: f64) -> Self {
        let mut suppression_rules = HashMap::new();

        // Default suppression rules to prevent alert fatigue
        suppression_rules.insert(
            "traffic_spike".to_string(),
            SuppressionRule {
                window_minutes: 15,
                max_alerts: 3,
            },
        );

        suppression_rules.insert(
            "default".to_string(),
            SuppressionRule {
                window_minutes: 30,
                max_alerts: 5,
            },
        );

        Self {
            confidence_threshold,
            alert_history: RwLock::new(Vec::new()),
            suppression_rules,
        }
    }

    /// Send alerts with intelligent deduplication
    pub async fn send_alerts(&self, anomalies: &[Anomaly]) -> Result<()> {
        let mut history = self.alert_history.write().await;

        for anomaly in anomalies {
            // Check if alert should be suppressed
            if self.should_suppress(anomaly, &history) {
                tracing::debug!(
                    "Suppressing alert for {:?} (deduplication)",
                    anomaly.anomaly_type
                );
                continue;
            }

            // Send alert based on severity
            self.send_alert(anomaly).await?;

            // Add to history
            history.push(anomaly.clone());
        }

        // Clean old alerts (keep last 24 hours)
        let cutoff = Utc::now() - Duration::hours(24);
        history.retain(|a| a.timestamp > cutoff);

        Ok(())
    }

    async fn send_alert(&self, anomaly: &Anomaly) -> Result<()> {
        match anomaly.severity {
            Severity::Critical => {
                tracing::error!(
                    "[CRITICAL] ANOMALY: {:?} in {}/{}",
                    anomaly.anomaly_type,
                    anomaly.context.namespace,
                    anomaly.context.service
                );
                // In production: PagerDuty, Slack, email
            }
            Severity::High => {
                tracing::warn!(
                    "[WARNING] HIGH SEVERITY: {:?} in {}/{}",
                    anomaly.anomaly_type,
                    anomaly.context.namespace,
                    anomaly.context.service
                );
                // In production: Slack, email
            }
            Severity::Medium => {
                tracing::warn!(
                    "[MEDIUM] {:?} detected",
                    anomaly.anomaly_type
                );
                // In production: Slack notification
            }
            Severity::Low | Severity::Info => {
                tracing::info!(
                    "[INFO] {:?} observed",
                    anomaly.anomaly_type
                );
                // In production: Log aggregation only
            }
        }

        Ok(())
    }

    fn should_suppress(&self, anomaly: &Anomaly, history: &[Anomaly]) -> bool {
        let rule = self
            .suppression_rules
            .get(&format!("{:?}", anomaly.anomaly_type))
            .or_else(|| self.suppression_rules.get("default"))
            .unwrap();

        let window_start = Utc::now() - Duration::minutes(rule.window_minutes as i64);

        let similar_alerts = history
            .iter()
            .filter(|a| {
                a.timestamp > window_start
                    && a.anomaly_type == anomaly.anomaly_type
                    && a.context.namespace == anomaly.context.namespace
                    && a.context.service == anomaly.context.service
            })
            .count();

        similar_alerts >= rule.max_alerts
    }

    /// Get anomalies in a time range
    pub fn get_anomalies_in_range(
        &self,
        _start: DateTime<Utc>,
        _end: DateTime<Utc>,
    ) -> Result<Vec<Anomaly>> {
        // In real implementation, this would query a persistent store
        Ok(Vec::new())
    }
}
