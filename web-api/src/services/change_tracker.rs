// Background GitOps change tracker
//
// Runs every 120 seconds, queries K8s events for recent changes to
// network-related resources, and stores them in Redis under the
// `cv:changes:` prefix. Detects ArgoCD/Flux annotations to mark
// GitOps-managed resources.

use crate::AppState;
use std::sync::Arc;

const CHANGES_PREFIX: &str = "cv:changes:";
const CHANGES_TTL: u64 = 604800; // 7 days

/// Spawn the background change tracker as a detached tokio task.
pub fn spawn_change_tracker(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
        loop {
            interval.tick().await;
            if let Err(e) = track_changes(&state).await {
                tracing::debug!("Change tracking cycle failed: {}", e);
            }
        }
    });
}

/// Resource kinds we care about for change tracking.
const RELEVANT_KINDS: &[&str] = &[
    "CiliumNetworkPolicy",
    "CiliumClusterwideNetworkPolicy",
    "ConfigMap",
    "Service",
    "Deployment",
];

/// Run a single change-tracking cycle: fetch K8s events, filter to
/// relevant network resources, check for GitOps annotations, and
/// persist new changes to Redis.
async fn track_changes(state: &AppState) -> anyhow::Result<()> {
    let data = state
        .k8s
        .kubectl_json(&[
            "get",
            "events",
            "--all-namespaces",
            "--field-selector",
            "reason!=Pulled,reason!=Scheduled,reason!=Started",
            "-o",
            "json",
            "--sort-by=.lastTimestamp",
        ])
        .await;

    let items = match data.get("items").and_then(|v| v.as_array()) {
        Some(items) => items,
        None => {
            tracing::debug!("Change tracker: no events found");
            return Ok(());
        }
    };

    let mut new_count: u64 = 0;

    for item in items {
        let involved = match item.get("involvedObject") {
            Some(v) => v,
            None => continue,
        };

        let kind = involved.get("kind").and_then(|v| v.as_str()).unwrap_or("");

        // Filter: only relevant resource kinds
        if !is_relevant_resource(kind, involved) {
            continue;
        }

        let meta = item.get("metadata").unwrap_or(item);
        let uid = meta.get("uid").and_then(|v| v.as_str()).unwrap_or("");
        if uid.is_empty() {
            continue;
        }

        // Short UID for the change ID
        let uid_short = &uid[..uid.len().min(8)];
        let redis_key = format!("{}{}", CHANGES_PREFIX, uid);

        // Check if we already stored this event
        if let Ok(Some(_)) = state.cache.get::<serde_json::Value>(&redis_key).await {
            continue;
        }

        let resource_name = involved.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let namespace = meta
            .get("namespace")
            .and_then(|v| v.as_str())
            .or_else(|| involved.get("namespace").and_then(|v| v.as_str()))
            .unwrap_or("");
        let reason = item.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        let message = item.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let timestamp = item
            .get("lastTimestamp")
            .or_else(|| item.get("firstTimestamp"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Check for ArgoCD / Flux GitOps annotations on the involved resource
        let gitops_managed = check_gitops_annotations(state, kind, resource_name, namespace).await;

        let change = serde_json::json!({
            "id": format!("chg-{}", uid_short),
            "timestamp": timestamp,
            "type": kind,
            "resource": resource_name,
            "namespace": namespace,
            "reason": reason,
            "message": message,
            "gitops_managed": gitops_managed,
            "rollback_available": false,
        });

        if let Err(e) = state.cache.set(&redis_key, &change, CHANGES_TTL).await {
            tracing::debug!("Failed to store change {}: {}", uid_short, e);
            continue;
        }

        new_count += 1;
    }

    if new_count > 0 {
        tracing::debug!("Change tracker: stored {} new change(s)", new_count);
    }

    Ok(())
}

/// Check whether the event kind is relevant. For ConfigMaps, we only care
/// about the `cilium-config` ConfigMap.
fn is_relevant_resource(kind: &str, involved: &serde_json::Value) -> bool {
    if !RELEVANT_KINDS.contains(&kind) {
        return false;
    }
    if kind == "ConfigMap" {
        let name = involved.get("name").and_then(|v| v.as_str()).unwrap_or("");
        return name == "cilium-config";
    }
    true
}

/// Query the actual K8s resource to check for ArgoCD or Flux annotations/labels.
async fn check_gitops_annotations(
    state: &AppState,
    kind: &str,
    name: &str,
    namespace: &str,
) -> bool {
    if name.is_empty() {
        return false;
    }

    // Map resource kind to kubectl resource type
    let resource_type = match kind {
        "CiliumNetworkPolicy" => "ciliumnetworkpolicy",
        "CiliumClusterwideNetworkPolicy" => "ciliumclusterwidenetworkpolicy",
        "ConfigMap" => "configmap",
        "Service" => "service",
        "Deployment" => "deployment",
        _ => return false,
    };

    let mut args = vec!["get", resource_type, name, "-o", "json"];
    if !namespace.is_empty() {
        args.push("-n");
        args.push(namespace);
    }

    let resource = state.k8s.kubectl_json(&args).await;

    let meta = match resource.get("metadata") {
        Some(m) => m,
        None => return false,
    };

    // Check ArgoCD annotations
    if let Some(annotations) = meta.get("annotations").and_then(|v| v.as_object()) {
        if annotations
            .keys()
            .any(|k| k.starts_with("argocd.argoproj.io/"))
        {
            return true;
        }
    }

    // Check Flux labels
    if let Some(labels) = meta.get("labels").and_then(|v| v.as_object()) {
        if labels.keys().any(|k| k.starts_with("toolkit.fluxcd.io/")) {
            return true;
        }
    }

    false
}
