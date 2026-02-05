# 🚀 Cilium TUI - Zero-Touch Observability

A fully automatic, zero-configuration Terminal User Interface (TUI) for Cilium network observability with intelligent bootstrapping.

## Features

✨ **Zero-Touch Setup**
- Auto-detects Kubernetes cluster and context
- Auto-detects Cilium installation
- Auto-enables required features (Hubble, L7 proxy, metrics)
- Auto-creates default network policies
- Auto-configures port forwarding
- Auto-creates necessary RBAC permissions

🛡️ **Automatic Security Policies**
- Intra-namespace communication allowed by default
- DNS resolution enabled automatically
- Hubble observability traffic permitted
- Best-practice policies for common patterns

📊 **Real-Time Observability**
- Live network flow monitoring
- Endpoint discovery and tracking
- Policy visualization
- Metrics dashboard

## Quick Start

Just run:

```bash
cilium-tui
```

That's it! The tool will:

1. ✔ Detect your cluster
2. ✔ Verify Cilium is installed
3. ✔ Enable Hubble and required features
4. ✔ Apply default network policies
5. ✔ Setup port forwarding
6. ✔ Launch the interactive TUI

## Installation

### From Source

```bash
git clone https://github.com/yourusername/cilium-tui.git
cd cilium-tui
cargo build --release
sudo cp target/release/cilium-tui /usr/local/bin/
```

### Using Cargo

```bash
cargo install cilium-tui
```

## Usage

### Standard Mode (Automatic Bootstrap)

```bash
cilium-tui
```

### Auto-Install Cilium

If Cilium is not installed, automatically install it:

```bash
# Interactive (prompts for confirmation)
cilium-tui

# Fully automatic (no prompts)
cilium-tui --auto-install
```

### Auto-Upgrade Cilium

Automatically upgrade Cilium to the latest version:

```bash
cilium-tui --auto-upgrade
```

### Combined Auto-Install & Auto-Upgrade

```bash
cilium-tui --auto-install --auto-upgrade
```

### Skip Bootstrap

If you've already run bootstrap and just want to launch the TUI:

```bash
cilium-tui --skip-bootstrap
```

### Custom Hubble Port

```bash
cilium-tui --hubble-port 4246
```

### Verbose Logging

```bash
cilium-tui --verbose
```

### All CLI Options

```bash
cilium-tui --help

Options:
      --skip-bootstrap     Skip bootstrap and go straight to TUI
      --hubble-port <PORT> Hubble port (default: 4245)
  -v, --verbose            Enable verbose logging
      --auto-install       Automatically install Cilium if not present
      --auto-upgrade       Automatically upgrade Cilium to latest version
  -h, --help               Print help
```

## What Gets Automatically Created

### 1. Cilium ConfigMap

The tool ensures these features are enabled:

```yaml
enable-hubble: true
hubble-metrics-enabled: true
hubble-listen-address: :4244
hubble-relay-enabled: true
monitor-aggregation: medium
enable-l7-proxy: true
```

### 2. Network Policies

Three default policies per namespace:

**Allow Intra-Namespace Traffic:**
```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-intra-namespace
spec:
  endpointSelector: {}
  ingress:
    - fromEndpoints:
        - {}
```

**Allow DNS:**
```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-dns
spec:
  endpointSelector: {}
  egress:
    - toEndpoints:
        - matchLabels:
            k8s-app: kube-dns
      toPorts:
        - ports:
            - port: "53"
              protocol: UDP
```

**Allow Hubble Observability:**
```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: allow-hubble
  namespace: kube-system
spec:
  endpointSelector:
    matchLabels:
      k8s-app: cilium
  ingress:
    - fromEndpoints:
        - matchLabels:
            app: cilium-tui
```

### 3. Service Account & RBAC

Creates `cilium-tui` ServiceAccount with cluster-admin permissions for observability.

## TUI Navigation

- **Tab**: Switch between views
- **Shift+Tab**: Previous view
- **q**: Quit

### Views

1. **Flows**: Real-time network traffic
2. **Endpoints**: Discovered endpoints
3. **Policies**: Active network policies
4. **Metrics**: Cluster metrics

## Requirements

- Kubernetes cluster (1.23+)
- Cilium installed (1.12+)
- kubectl configured
- cilium CLI (for port-forwarding)

## Architecture

```
cilium-tui
├── bootstrap/     - Auto-detection and setup
├── kubernetes/    - K8s API client wrapper
├── cilium/        - Cilium-specific operations
├── hubble/        - Hubble gRPC integration
├── policies/      - Network policy generation
└── tui/           - Terminal UI components
```

## Development

### Build

```bash
cargo build
```

### Run

```bash
cargo run
```

### Test

```bash
cargo test
```

## Future Enhancements

- [ ] gRPC direct connection to Hubble (currently uses CLI)
- [ ] Endpoint health monitoring
- [ ] Flow filtering and search
- [ ] Policy recommendation engine
- [ ] Export flows to JSON/CSV
- [ ] Multi-cluster support
- [ ] Custom policy templates

## License

Apache-2.0

## Contributing

Contributions welcome! Please open an issue or PR.

## Credits

Built with:
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [kube-rs](https://github.com/kube-rs/kube) - Kubernetes client
- [tokio](https://tokio.rs/) - Async runtime
