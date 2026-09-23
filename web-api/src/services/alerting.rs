// Background alerting engine
//
// Evaluates alert rules stored in the in-memory cache every 60 seconds and fires alerts
// when conditions are met. Alert history is persisted back to the cache.

use crate::AppState;
use std::sync::Arc;

const ALERT_RULES_PREFIX: &str = "cv:alert_rules:";
const ALERT_HISTORY_PREFIX: &str = "cv:alert_history:";

/// Start the background alerting engine.
/// Evaluates alert rules from the cache every 60 seconds.
pub fn spawn_alerting_engine(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Err(e) = evaluate_rules(&state).await {
                tracing::warn!("Alert evaluation failed: {}", e);
            }
        }
    });
}

/// Load all alert rules from the cache, evaluate each enabled rule against
/// live Hubble flows, and fire alerts when thresholds are breached.
async fn evaluate_rules(state: &AppState) -> anyhow::Result<()> {
    let rules = state.cache.list_values(ALERT_RULES_PREFIX).await?;
    if rules.is_empty() {
        tracing::debug!("No alert rules found, skipping evaluation cycle");
        return Ok(());
    }

    // Fetch flows once for all rules that need them
    let flows = state.hubble.get_flows(500, None).await.unwrap_or_default();

    for rule in &rules {
        let enabled = rule
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !enabled {
            continue;
        }

        let rule_id = rule.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let rule_name = rule
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unnamed");
        let severity = rule
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("warning");
        let condition = rule.get("condition").and_then(|v| v.as_str()).unwrap_or("");

        let result = evaluate_condition(condition, &flows, state).await;

        match result {
            ConditionResult::Fired(message) => {
                tracing::info!(
                    rule_id = rule_id,
                    rule_name = rule_name,
                    "Alert fired: {}",
                    message
                );
                fire_alert(state, rule_id, rule_name, severity, &message).await;
                update_rule_trigger(state, rule_id).await;
            }
            ConditionResult::Ok => {}
            ConditionResult::Skipped(reason) => {
                tracing::debug!(rule_id = rule_id, "Rule skipped: {}", reason);
            }
        }
    }

    Ok(())
}

enum ConditionResult {
    /// Condition threshold was breached; contains a human-readable message.
    Fired(String),
    /// Condition evaluated normally and is within threshold.
    Ok,
    /// Condition could not be evaluated (unsupported, missing data, etc.).
    Skipped(String),
}

/// Parse and evaluate a single rule condition string against the flow data.
async fn evaluate_condition(
    condition: &str,
    flows: &[crate::models::flow::Flow],
    state: &AppState,
) -> ConditionResult {
    let cond = condition.trim();

    // ── drop_rate > X% for Ym ───────────────────────────────
    if cond.starts_with("drop_rate >") {
        if let Some(threshold) = parse_percent_threshold(cond) {
            let total = flows.len();
            if total == 0 {
                return ConditionResult::Ok;
            }
            let dropped = flows.iter().filter(|f| f.verdict == "DROPPED").count();
            let rate = (dropped as f64 / total as f64) * 100.0;
            if rate > threshold {
                return ConditionResult::Fired(format!(
                    "Drop rate {:.1}% exceeds threshold {:.1}% ({} dropped out of {} flows)",
                    rate, threshold, dropped, total
                ));
            }
            return ConditionResult::Ok;
        }
        return ConditionResult::Skipped(format!("Could not parse drop_rate condition: {}", cond));
    }

    // ── dns_servfail > N/min ────────────────────────────────
    if cond.starts_with("dns_servfail >") {
        if let Some(threshold) = parse_rate_threshold(cond) {
            // Count DNS (port 53) flows that were DROPPED as a proxy for SERVFAIL
            let dns_drops = flows
                .iter()
                .filter(|f| f.port == 53 && f.verdict == "DROPPED")
                .count();
            if dns_drops as f64 > threshold {
                return ConditionResult::Fired(format!(
                    "DNS failures {} exceed threshold {}/min",
                    dns_drops, threshold as u64
                ));
            }
            return ConditionResult::Ok;
        }
        return ConditionResult::Skipped(format!(
            "Could not parse dns_servfail condition: {}",
            cond
        ));
    }

    // ── policy_denied > N/min ───────────────────────────────
    if cond.starts_with("policy_denied >") {
        if let Some(threshold) = parse_rate_threshold(cond) {
            let denied = flows.iter().filter(|f| f.verdict == "DROPPED").count();
            if denied as f64 > threshold {
                return ConditionResult::Fired(format!(
                    "Policy denies {} exceed threshold {}/min",
                    denied, threshold as u64
                ));
            }
            return ConditionResult::Ok;
        }
        return ConditionResult::Skipped(format!(
            "Could not parse policy_denied condition: {}",
            cond
        ));
    }

    // ── endpoint_status != ready ────────────────────────────
    if cond.starts_with("endpoint_status") && cond.contains("!= ready") {
        let data = state
            .k8s
            .kubectl_json(&["get", "ciliumendpoints", "--all-namespaces", "-o", "json"])
            .await;

        let not_ready: Vec<String> = data
            .get("items")
            .and_then(|v| v.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let ep_state = item
                            .get("status")
                            .and_then(|s| s.get("state"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("ready");
                        if ep_state != "ready" {
                            let name = item
                                .get("metadata")
                                .and_then(|m| m.get("name"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            Some(format!("{} ({})", name, ep_state))
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        if !not_ready.is_empty() {
            let summary = if not_ready.len() <= 5 {
                not_ready.join(", ")
            } else {
                format!(
                    "{} and {} more",
                    not_ready[..5].join(", "),
                    not_ready.len() - 5
                )
            };
            return ConditionResult::Fired(format!(
                "{} endpoint(s) not ready: {}",
                not_ready.len(),
                summary
            ));
        }
        return ConditionResult::Ok;
    }

    // ── ct_entries > X% max ─────────────────────────────────
    if cond.starts_with("ct_entries >") {
        return ConditionResult::Skipped(
            "ct_entries evaluation requires eBPF reader (not available in web-api)".to_string(),
        );
    }

    // ── Unrecognized condition ──────────────────────────────
    ConditionResult::Skipped(format!("Unrecognized alert condition: {}", cond))
}

/// Parse a threshold percentage from conditions like "drop_rate > 5% for 5m".
/// Returns the numeric threshold (e.g. 5.0).
fn parse_percent_threshold(condition: &str) -> Option<f64> {
    // Look for a pattern like "> 5%" or ">5%"
    let after_gt = condition.split('>').nth(1)?;
    let pct_pos = after_gt.find('%')?;
    let num_str = after_gt[..pct_pos].trim();
    num_str.parse::<f64>().ok()
}

/// Parse a rate threshold from conditions like "dns_servfail > 10/min".
/// Returns the numeric threshold (e.g. 10.0).
fn parse_rate_threshold(condition: &str) -> Option<f64> {
    let after_gt = condition.split('>').nth(1)?;
    let num_str = after_gt.split('/').next()?.trim();
    num_str.parse::<f64>().ok()
}

/// Write a fired alert to cache alert history.
async fn fire_alert(
    state: &AppState,
    rule_id: &str,
    rule_name: &str,
    severity: &str,
    message: &str,
) {
    let alert_id = format!("alert-{}", uuid::Uuid::new_v4());
    let now = chrono::Utc::now().to_rfc3339();

    let alert = serde_json::json!({
        "id": alert_id,
        "rule_id": rule_id,
        "rule_name": rule_name,
        "severity": severity,
        "message": message,
        "fired_at": now,
        "status": "firing",
    });

    let key = format!("{}{}", ALERT_HISTORY_PREFIX, alert_id);
    if let Err(e) = state.cache.set_persistent(&key, &alert).await {
        tracing::warn!("Failed to persist alert {}: {}", alert_id, e);
    }
}

/// Update the rule's last_triggered timestamp and increment trigger_count.
async fn update_rule_trigger(state: &AppState, rule_id: &str) {
    let key = format!("{}{}", ALERT_RULES_PREFIX, rule_id);
    if let Ok(Some(mut rule)) = state.cache.get::<serde_json::Value>(&key).await {
        rule["last_triggered"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        let count = rule
            .get("trigger_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        rule["trigger_count"] = serde_json::json!(count + 1);
        if let Err(e) = state.cache.set_persistent(&key, &rule).await {
            tracing::warn!("Failed to update rule {}: {}", rule_id, e);
        }
    }
}
