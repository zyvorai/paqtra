# Cilium Vision - Quick Start Guide

Get up and running with Cilium Vision in 5 minutes!

## Prerequisites

- ✅ Kubernetes cluster (local or remote)
- ✅ `kubectl` installed and configured
- ✅ Rust 1.70+ (for building from source)

## Step 1: Install Cilium

```bash
# Install Cilium CLI
CILIUM_CLI_VERSION=$(curl -s https://raw.githubusercontent.com/cilium/cilium-cli/main/stable.txt)
curl -L --fail --remote-name-all https://github.com/cilium/cilium-cli/releases/download/${CILIUM_CLI_VERSION}/cilium-linux-amd64.tar.gz
sudo tar xzvfC cilium-linux-amd64.tar.gz /usr/local/bin

# Install Cilium
cilium install --version 1.14.5

# Enable Hubble
cilium hubble enable
```

## Step 2: Build Cilium Vision

```bash
git clone https://github.com/ssahani/cilium-flow.git
cd cilium-flow
cargo build --release
```

## Step 3: Run

```bash
./target/release/cilium-tui
```

## Quick Reference

```
Global: ? = Help, q = Quit, Tab = Next tab
Flows: e = Explain packet
AutoPolicy: g = Generate, a = Apply
Simulator: s = Simulate
Replay: t = Time-travel
Chaos: Enter = Run experiment
Canary: p = Promote, r = Rollback
```

## Next Steps

- Read [FEATURES.md](FEATURES.md) for detailed feature guide
- Check [examples/](examples/) for real-world scenarios
- See [docs/architecture.md](docs/architecture.md) for architecture

---

Built with ❤️ using Rust, ratatui, and Cilium
