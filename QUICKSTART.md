# Quick Start Guide

## Prerequisites

Before running `cilium-tui`, ensure you have:

1. **Kubernetes cluster** with kubectl configured
   ```bash
   kubectl cluster-info
   ```

2. **Cilium installed** in your cluster
   ```bash
   cilium status
   ```

   If not installed:
   ```bash
   cilium install
   ```

3. **Cilium CLI** installed (for port-forwarding)
   ```bash
   cilium version
   ```

## Installation

### Build from source

```bash
git clone <repo-url>
cd cilium-tui
cargo build --release
sudo cp target/release/cilium-tui /usr/local/bin/
```

## First Run

Simply execute:

```bash
cilium-tui
```

You'll see:

```
🚀 Bootstrapping Cilium-TUI...
✔ Cluster detected: minikube
✔ Cilium detected
✔ Hubble enabled
✔ Default policies applied
✔ DNS allowed
✔ Hubble port-forward started
✔ Connected to Hubble

🎉 Launching TUI...
```

## What Happens Automatically

### 1. Cilium Configuration Enhanced

The tool updates the Cilium ConfigMap with:

```yaml
enable-hubble: true
hubble-metrics-enabled: true
hubble-listen-address: :4244
hubble-relay-enabled: true
monitor-aggregation: medium
enable-l7-proxy: true
```

### 2. Network Policies Created

For each namespace:

- **allow-intra-namespace**: All pods in same namespace can talk
- **allow-dns**: All pods can reach kube-dns

For kube-system:

- **allow-hubble**: Enables Hubble observability

### 3. RBAC Setup

Creates ServiceAccount `cilium-tui` with cluster-admin permissions.

### 4. Hubble Port-Forward

Automatically runs:
```bash
cilium hubble port-forward
```

in the background.

## Using the TUI

Once launched, you'll see a multi-tab interface:

### Tab 1: Flows

Real-time network traffic with color-coded verdicts:
- 🟢 Green: FORWARDED
- 🔴 Red: DROPPED
- 🟡 Yellow: Other

### Tab 2: Endpoints

(Coming soon) Discovered endpoints in the cluster.

### Tab 3: Policies

Shows all active Cilium Network Policies.

### Tab 4: Metrics

(Coming soon) Cluster and network metrics.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Next view |
| `Shift+Tab` | Previous view |
| `q` | Quit |

## Testing It Works

After launching, generate some traffic:

```bash
# In another terminal
kubectl run test --image=nginx
kubectl run client --image=curlimages/curl --command -- sleep 3600
kubectl exec -it client -- curl test
```

You should see flows appear in the TUI!

## Troubleshooting

### "Cilium not detected"

Install Cilium:
```bash
cilium install
```

### "Failed to port-forward"

Manually start:
```bash
cilium hubble port-forward
```

Then run with skip-bootstrap:
```bash
cilium-tui --skip-bootstrap
```

### No flows showing

1. Check Hubble is running:
   ```bash
   cilium hubble observe
   ```

2. Ensure port-forward is active:
   ```bash
   ps aux | grep "hubble port-forward"
   ```

### Permission denied

The tool needs cluster-admin to create policies and read all resources. Ensure your kubeconfig has appropriate permissions.

## Advanced Usage

### Skip Bootstrap

If you've already run bootstrap once:

```bash
cilium-tui --skip-bootstrap
```

### Custom Port

```bash
cilium-tui --hubble-port 4246
```

### Verbose Logging

```bash
cilium-tui --verbose
```

## Next Steps

1. Explore the different tabs
2. Watch flows in real-time
3. Review auto-created policies
4. (Future) Create custom policy templates
5. (Future) Export flows to JSON

## Cleanup

To remove auto-created resources:

```bash
kubectl delete clusterrolebinding cilium-tui-binding
kubectl delete serviceaccount cilium-tui -n kube-system
kubectl delete ciliumnetworkpolicy allow-intra-namespace --all-namespaces
kubectl delete ciliumnetworkpolicy allow-dns --all-namespaces
kubectl delete ciliumnetworkpolicy allow-hubble -n kube-system
```
