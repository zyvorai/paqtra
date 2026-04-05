# Cilium Flow

**Real-time Network Observability & Intelligence Platform for Kubernetes**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-961%20passing-brightgreen.svg)](#testing)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Cilium](https://img.shields.io/badge/cilium-1.14%2B-purple.svg)](https://cilium.io/)
[![Lines of Code](https://img.shields.io/badge/LoC-32k%20Rust-informational.svg)](#)

Cilium Flow is a terminal-based platform that turns Cilium's eBPF data plane into an intelligent observability and operations console. It combines live flow monitoring, ML-enhanced policy automation, chaos engineering, canary deployments, and multi-cluster orchestration in a single binary.

---

## Key Capabilities

| Area | What It Does |
|------|-------------|
| **Live Flows** | Stream Hubble flows with verdict coloring, per-packet explanations |
| **AutoPolicy** | Learn traffic patterns, generate CiliumNetworkPolicy with ML confidence scores |
| **Root Cause** | Detect drops, correlate with policies, propose one-click fixes |
| **Simulator** | Dry-run policy changes with impact analysis and risk scoring |
| **Time-Travel** | Record flows, replay with VCR controls, jump to events |
| **Chaos** | Inject packet loss / latency / DNS failures via tc-netem, circuit breaker |
| **Canary** | Progressive traffic shifting with promote / rollback / health gates |
| **Multi-Cluster** | Register clusters, health-check, topology view, policy sync |
| **Zero-Trust** | Generate default-deny + discovered-allow policies from Hubble/kubectl |
| **Profiler** | CPU / memory / network sampling via /proc with flame-graph output |

---

## Quick Start

### Prerequisites

- Kubernetes cluster with Cilium installed
- `kubectl` configured and pointing at the cluster
- Rust 1.75+ (for building from source)

### Install

```bash
git clone https://github.com/ssahani/cilium-flow.git
cd cilium-flow
cargo build --release
sudo cp target/release/cilium-tui /usr/local/bin/
```

### Run

```bash
# Full bootstrap (auto-detects cluster, enables Hubble, starts port-forward)
cilium-tui

# Skip bootstrap if Cilium & Hubble are already running
cilium-tui --skip-bootstrap

# Custom Hubble port
cilium-tui --hubble-port 4245

# Auto-install Cilium if missing (no prompts)
cilium-tui --auto-install
```

On first run the tool will:
1. Detect your Kubernetes cluster context
2. Verify Cilium is installed (offer to install if missing)
3. Enable Hubble relay if not running
4. Start port-forward to hubble-relay
5. Launch the interactive TUI

---

## Architecture

```
+-------------------------------------------------------------+
|                      Terminal UI (ratatui)                    |
|  Flows | Conns | Endpoints | Policies | Metrics | ...       |
+-------------------------------------------------------------+
|  Healer | AutoPolicy | RootCause | Simulator | Replay       |
|  Chaos  | Canary     | MultiCluster                         |
+-------------------------------------------------------------+
|  eBPF Maps (Aya/bpftool)  |  Hubble CLI  |  K8s API (kube)  |
+-------------------------------------------------------------+
|              Cilium Agent  |  Kernel eBPF datapath            |
+-------------------------------------------------------------+
```

**92 source files** across 13 modules, with 9 integration test suites.

---

## Tabs & Navigation

### Global Keys

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle through 13 tabs |
| `?` | Toggle help overlay |
| `q` | Quit |
| `Esc` | Cancel / close overlay |

### Per-Tab Keys

| Tab | Keys | Actions |
|-----|------|---------|
| **Flows** | `e`, Up/Down | Explain packet, navigate |
| **Healer** | `d` | Detect problems |
| **AutoPolicy** | `u` `g` `v` `a` `r` `A` `R` | Update, generate, view, apply, rollback, batch apply/rollback |
| **RootCause** | `a`, Up/Down | Apply fix, navigate |
| **Simulator** | `s` `c`, Up/Down | Simulate, clear, navigate scenarios |
| **Replay** | `t` Space Left/Right `[` `]` `+` `-` | Time-travel, play/pause, step, jump, speed |
| **Chaos** | Enter `v` `s` `S` `b` | Run preset, toggle view, stop, stop-all, circuit breaker |
| **Canary** | `p` `r` `+` `d` | Promote, rollback, progress traffic, details |
| **MultiCluster** | `v` `h` | Cycle views, health-check all clusters |

---

## Module Details

### Chaos Engineering

Seven presets map to real `tc-netem` fault injection:

| Preset | Experiment | Default |
|--------|-----------|---------|
| Network Partition | PacketDrop | 20% loss |
| Latency Spike | Latency | 500ms +/- 50ms |
| DNS Outage | DNSFailure | 30% failure |
| Connection Reset | ConnectionKill | 15% kill |
| Bandwidth Limit | Bandwidth | 10 Mbps cap |
| Packet Corruption | PacketCorruption | 5% corrupt |
| Total Partition | PacketDrop | 100% loss |

Safety: max 50% drop rate, max 5s latency, circuit breaker, 5-minute auto-cleanup.

### Canary Deployments

Progressive traffic shifting via Cilium L7 annotations:

- Start at 10%, increment by 10% per step
- Auto-promote at 99% success rate, auto-rollback below 90%
- Real-time metrics: success rate, latency comparison, error rate
- Health gauges in TUI

### Multi-Cluster Autopilot

- Register clusters with provider/region metadata
- Health-check via `kubectl get nodes --context <name>`
- Topology view with connected pairs and region grouping
- Policy sync and workload placement recommendations

### Zero-Trust Policy Generator

- Discovers traffic patterns via `hubble observe` or `kubectl get pods`
- Generates: default-deny, allow-dns, allow-k8s-api, per-service-pair allow rules
- All policies are valid CiliumNetworkPolicy YAML

---

## Testing

```bash
cargo test          # 961 Rust tests (unit + integration)
cargo clippy        # 0 warnings
cargo build --release  # Optimized binary

cd web-api && cargo test  # 26 API tests
cd web-ui && npm test     # 68 UI tests
```

| Suite | Tests | Notes |
|-------|-------|-------|
| Library unit tests | 365 | Core logic, parsers, engines |
| Library (release) | 365 | Optimized build verification |
| Integration: modules | 51 | Cross-module interaction |
| Integration: autopolicy | 27 | ML confidence, policy gen |
| Integration: healer | 22 | Problem detection, fixes |
| Integration: simulator | 22 | Policy simulation, impact |
| Integration: replay | 20 | Recording, time-travel |
| Integration: cross-module | 24 | End-to-end workflows |
| Integration: tui-tabs | 17 | Tab navigation, rendering |
| Other suites | 48 | Remaining test files |
| Web API tests | 26 | Config, auth, models |
| Web UI tests | 68 | Components, stores, hooks |
| **Total** | **1,055** | **0 failures** |

---

## Configuration

```yaml
# ~/.config/cilium-vision/config.yaml
kubernetes:
  context: "my-cluster"

hubble:
  port: 4245

autopolicy:
  enabled: true
  ml_confidence_threshold: 0.75

chaos:
  enabled: true
  max_drop_rate: 0.5
  max_latency_ms: 5000
  require_confirmation: true

canary:
  initial_traffic_pct: 10
  auto_promote_threshold: 0.99
  auto_rollback_threshold: 0.90

multicluster:
  auto_sync_policies: true
  health_check_interval_secs: 30
```

---

## Project Structure

```
cilium-flow/
  src/
    main.rs                    Entry point & CLI args
    bootstrap/                 Auto-detection & setup
    tui/                       Terminal UI (13 tabs, views, event handlers)
    modules/
      autopolicy/              ML-enhanced policy generation
      canary/                  Sidecarless canary deployments
      chaos/                   eBPF chaos engineering
      healer/                  Self-healing problem detection
      multicluster/            Multi-cluster autopilot
      packet_explainer/        AI-like packet analysis
      replay/                  Flow recording & time-travel
      rootcause/               Drop root-cause analysis
      simulator/               Dry-run policy simulator
      anomaly_detection/       AI/ML anomaly detection
      ebpf_advanced/           Profiling, packet filters
      security_compliance/     Zero-trust, compliance
      dev_tools/               Traffic replay, shadowing
    ebpf/                      BPF map readers (Aya + bpftool)
    hubble/                    Hubble CLI/gRPC client
    kubernetes/                K8s API client
    integration/               Enriched data provider
    cilium/                    Cilium CLI manager
    endpoints/                 Endpoint discovery
  tests/                       9 integration test suites
  docs/                        Architecture, guides, status
  web-api/                     REST API server (Axum + Redis + JWT auth)
  web-ui/                      React dashboard (58 views, 25 components, 5 hooks)
```

---

## Web Dashboard

Cilium Flow includes a full-featured web UI built with React 19, TypeScript, and Tailwind CSS.

**58 views** covering observability, security, intelligence, operations, and networking. Dark-first design with real-time WebSocket updates, interactive charts, and keyboard shortcuts.

```bash
cd web-ui
npm install && npm run dev   # Dev server on port 3000
npm run build                # Production build
npm run test                 # 68 tests
```

See [docs/WEB_APP_README.md](docs/WEB_APP_README.md) for full details.

---

## Security

The platform includes defense-in-depth security controls:

- **JWT authentication** with HS256 and minimum 32-char secret enforcement
- **RBAC-ready** middleware — decoded claims injected into request context with `require_admin()` helper
- **Input validation** on all kubectl-bound fields (RFC 1123 DNS name regex, flag injection prevention)
- **Typed request structs** for all POST endpoints (no raw `Json<Value>` handlers)
- **CORS** with explicit origin allowlist; credentials only enabled for non-localhost origins
- **Graceful shutdown** with SIGTERM/SIGINT handling
- **Secure temp files** via `tempfile::NamedTempFile` (no predictable paths)
- **Confirmation dialogs** on destructive operations (node drain, chaos experiments)

Set `AUTH_DISABLED=true` only for development. The flag is read once at startup and logged as a warning.

---

## Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Rust | 1.75 | latest stable |
| Kubernetes | 1.25 | 1.28+ |
| Cilium | 1.14 | 1.16+ |
| Memory | 50 MB | 100 MB |
| CPU overhead | < 1% | - |

---

## License

Apache License 2.0

---

## Acknowledgments

- [Cilium](https://cilium.io/) - eBPF-based networking
- [Ratatui](https://ratatui.rs/) - Terminal UI framework
- [Hubble](https://docs.cilium.io/en/stable/observability/hubble/) - Network observability
- [Aya](https://aya-rs.dev/) - Rust eBPF library
