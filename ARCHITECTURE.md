# Architecture Documentation

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         CILIUM TUI                              │
│                     (Zero-Touch Bootstrap)                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
        ┌────────────────────────────────────────┐
        │         Bootstrap Manager              │
        │  (Orchestrates entire setup flow)      │
        └────────────────────────────────────────┘
                              │
                ┌─────────────┼─────────────┐
                │             │             │
                ▼             ▼             ▼
        ┌───────────┐  ┌───────────┐  ┌──────────┐
        │ Kubernetes│  │  Cilium   │  │ Policies │
        │  Client   │  │  Manager  │  │ Manager  │
        └───────────┘  └───────────┘  └──────────┘
                │             │             │
                │             │             │
                ▼             ▼             ▼
        ┌─────────────────────────────────────────┐
        │       Kubernetes API Server             │
        └─────────────────────────────────────────┘
                              │
                ┌─────────────┼─────────────┐
                │             │             │
                ▼             ▼             ▼
        ┌───────────┐  ┌───────────┐  ┌──────────┐
        │ ConfigMap │  │  Policies │  │   RBAC   │
        │  (Hubble) │  │  (CNP)    │  │  (SA)    │
        └───────────┘  └───────────┘  └──────────┘
```

## Module Breakdown

### 1. Main Entry Point

```rust
main.rs
  │
  ├─→ Parse CLI args (clap)
  ├─→ Setup logging (tracing)
  ├─→ Create BootstrapManager
  ├─→ Run bootstrap (unless --skip-bootstrap)
  └─→ Launch TUI
```

### 2. Bootstrap Flow

```
BootstrapManager::run_bootstrap()
  │
  ├─→ detect_cluster()
  │   └─→ K8sClient::get_current_context()
  │
  ├─→ detect_cilium()
  │   └─→ CiliumManager::is_installed()
  │
  ├─→ enable_cilium_features()
  │   └─→ CiliumManager::enable_features()
  │       └─→ Create/update ConfigMap
  │
  ├─→ apply_default_policies()
  │   ├─→ PolicyManager::apply_intra_namespace_policy()
  │   ├─→ PolicyManager::apply_dns_policy()
  │   └─→ PolicyManager::apply_hubble_policy()
  │
  ├─→ setup_service_account()
  │   └─→ CiliumManager::create_tui_service_account()
  │       ├─→ Create ServiceAccount
  │       └─→ Create ClusterRoleBinding
  │
  └─→ setup_hubble_port_forward()
      └─→ Spawn background process
```

### 3. Kubernetes Client

```
K8sClient
  │
  ├─→ new() - Infer config from ~/.kube/config
  │
  ├─→ list_namespaces() - Get all namespaces
  ├─→ list_pods() - Get pods in namespace
  ├─→ get_pods_by_label() - Filter pods by label
  │
  ├─→ create_or_update_configmap() - Apply ConfigMap
  ├─→ create_or_update_service_account() - Apply SA
  ├─→ create_or_update_cluster_role_binding() - Apply CRB
  │
  └─→ apply_custom_resource() - Apply CRD (via kubectl)
```

### 4. Cilium Manager

```
CiliumManager
  │
  ├─→ is_installed()
  │   └─→ Check for pods with label k8s-app=cilium
  │
  ├─→ enable_features()
  │   └─→ Update cilium-config ConfigMap
  │       ├─→ enable-hubble: true
  │       ├─→ hubble-metrics-enabled: true
  │       └─→ enable-l7-proxy: true
  │
  └─→ create_tui_service_account()
      ├─→ Create ServiceAccount in kube-system
      └─→ Bind to cluster-admin role
```

### 5. Policy Manager

```
PolicyManager
  │
  ├─→ apply_intra_namespace_policy(ns)
  │   └─→ Allow all pods in same namespace to communicate
  │
  ├─→ apply_dns_policy(ns)
  │   └─→ Allow all pods to reach kube-dns
  │
  ├─→ apply_hubble_policy()
  │   └─→ Allow Hubble observability traffic
  │
  └─→ apply_best_practice_policy()
      └─→ Custom app-to-app policies
```

### 6. Hubble Client

```
HubbleClient
  │
  ├─→ new(port) - Create client for Hubble
  │
  ├─→ start_port_forward()
  │   └─→ Background: cilium hubble port-forward
  │
  └─→ get_flows()
      └─→ Fetch flows via CLI (future: gRPC)
```

### 7. TUI Application

```
TuiApp
  │
  ├─→ new(context, port) - Initialize TUI
  │
  ├─→ run()
  │   ├─→ Setup terminal (crossterm)
  │   ├─→ Create backend (ratatui)
  │   ├─→ Event loop
  │   └─→ Cleanup on exit
  │
  ├─→ ui() - Main rendering
  │   ├─→ Title bar
  │   ├─→ Tab selector
  │   ├─→ Content area (based on selected tab)
  │   └─→ Footer (keyboard shortcuts)
  │
  └─→ Render functions
      ├─→ render_flows() - Live network traffic
      ├─→ render_endpoints() - Discovered endpoints
      ├─→ render_policies() - Active policies
      └─→ render_metrics() - Cluster metrics
```

## Data Flow

### Flow Monitoring

```
Cilium Agent
     │
     ▼
Hubble Observer
     │
     ▼
Port Forward (localhost:4245)
     │
     ▼
HubbleClient::get_flows()
     │
     ▼
Parse JSON flows
     │
     ▼
TuiApp.flows (Vec<Flow>)
     │
     ▼
render_flows()
     │
     ▼
Terminal Display
```

### Policy Creation

```
User runs cilium-tui
     │
     ▼
Bootstrap detects namespaces
     │
     ▼
For each namespace:
     │
     ├─→ Generate YAML for allow-intra-namespace
     ├─→ Generate YAML for allow-dns
     │
     ▼
K8sClient::apply_custom_resource()
     │
     ▼
kubectl apply -f -
     │
     ▼
Kubernetes API creates CiliumNetworkPolicy
     │
     ▼
Cilium Agent enforces policy
```

## State Management

### Application State

```rust
TuiApp {
    hubble_client: HubbleClient,      // Connection to Hubble
    flows: Vec<Flow>,                 // Latest flows
    selected_tab: usize,              // Current tab index
    context: String,                  // Cluster context
}
```

### Flow Data

```rust
Flow {
    time: String,                     // Timestamp
    verdict: String,                  // FORWARDED/DROPPED
    source: FlowEndpoint {            // Source pod
        namespace: String,
        pod_name: String,
        ip: String,
    },
    destination: FlowEndpoint {       // Dest pod
        namespace: String,
        pod_name: String,
        ip: String,
    },
    type: String,                     // L3/L4/L7
}
```

## Async Architecture

```
Tokio Runtime
  │
  ├─→ Main Thread
  │   └─→ TUI Event Loop (blocking)
  │
  ├─→ Background Task: Port Forward
  │   └─→ cilium hubble port-forward
  │
  └─→ Background Task: Flow Fetching
      └─→ Periodic HubbleClient::get_flows()
```

## Error Handling

```
Result<T, anyhow::Error>
  │
  ├─→ Bootstrap errors → Early exit with message
  ├─→ K8s API errors → Propagate with context
  ├─→ TUI errors → Cleanup terminal, then exit
  └─→ Flow fetch errors → Log, continue (non-fatal)
```

## Security Model

### Permissions Required

```
ClusterRole: cluster-admin
  │
  ├─→ Read all pods
  ├─→ Read all namespaces
  ├─→ Create/update ConfigMaps
  ├─→ Create/update CiliumNetworkPolicies
  ├─→ Create ServiceAccounts
  └─→ Create ClusterRoleBindings
```

### Resource Creation

All resources created are:
- Idempotent (can run multiple times)
- Non-destructive (don't delete existing resources)
- Namespace-scoped (except RBAC)
- Labeled for identification

## Performance Considerations

### Optimization Strategies

1. **Lazy Loading**
   - Only fetch flows when TUI is active
   - Limit flow history (last 100)

2. **Caching**
   - Cache namespace list
   - Cache pod labels

3. **Async Operations**
   - Port forward in background
   - Non-blocking K8s API calls

4. **Efficient Rendering**
   - Only redraw on data change or user input
   - Use ratatui's incremental rendering

## Extension Points

### Adding New Policy Types

```rust
impl PolicyManager {
    pub async fn apply_custom_policy(
        &self,
        namespace: &str,
        template: PolicyTemplate
    ) -> Result<()> {
        // Generate YAML from template
        // Apply via K8s client
    }
}
```

### Adding New TUI Tabs

```rust
impl TuiApp {
    fn render_my_new_tab(&self, f: &mut Frame, area: Rect) {
        // Custom rendering logic
    }
}
```

### Adding Metrics Collection

```rust
impl MetricsClient {
    pub async fn get_node_metrics(&self) -> Result<Vec<Metric>> {
        // Fetch from metrics-server or Prometheus
    }
}
```

## Deployment Modes

### Mode 1: Local CLI (Current)

```
Developer Laptop
  ├─→ cilium-tui binary
  ├─→ ~/.kube/config
  └─→ kubectl + cilium CLI
```

### Mode 2: In-Cluster (Future)

```
Kubernetes Pod
  ├─→ ServiceAccount (cilium-tui)
  ├─→ Direct K8s API access
  └─→ Direct Hubble gRPC connection
```

### Mode 3: Remote (Future)

```
Local Machine ←→ SSH/VPN ←→ Kubernetes Cluster
                               │
                               ├─→ TUI displays locally
                               └─→ Data fetched remotely
```

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_policy_generation() {
        // Test YAML generation
    }

    #[test]
    fn test_flow_parsing() {
        // Test JSON parsing
    }
}
```

### Integration Tests

```bash
# tests/integration_test.sh
minikube start
cilium install
cargo run --release
# Verify policies created
# Verify flows visible
```

## Debugging

### Log Levels

```bash
cilium-tui --verbose            # DEBUG level
cilium-tui                      # INFO level
RUST_LOG=trace cilium-tui       # TRACE level
```

### Common Issues

1. **Port forward fails**
   - Check: `ps aux | grep hubble`
   - Fix: `killall cilium && cilium-tui`

2. **No flows showing**
   - Check: `cilium hubble observe`
   - Fix: Verify Hubble enabled

3. **Permission denied**
   - Check: `kubectl auth can-i --list`
   - Fix: Use cluster-admin kubeconfig

---

This architecture is designed to be:
- **Modular**: Easy to extend
- **Testable**: Clear separation of concerns
- **Maintainable**: Well-documented
- **Performant**: Async-first design
