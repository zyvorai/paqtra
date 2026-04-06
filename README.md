# Cilium Vision

**Real-Time Network Intelligence Platform for Kubernetes**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![React](https://img.shields.io/badge/react-19-blue.svg)](https://react.dev/)
[![Tests](https://img.shields.io/badge/tests-961%20passing-brightgreen.svg)](#testing)
[![API](https://img.shields.io/badge/API-75%2B%20endpoints-blueviolet.svg)](#web-api)
[![Pages](https://img.shields.io/badge/dashboard-64%20pages-cyan.svg)](#web-dashboard)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Cilium](https://img.shields.io/badge/cilium-1.14%2B-purple.svg)](https://cilium.io/)

Cilium Vision is a full-stack observability and operations platform for Kubernetes networks powered by Cilium eBPF. It provides a **web dashboard** (64 pages), **REST API** (75+ endpoints), and **terminal TUI** (13 tabs) in a single deployable package. The dashboard includes real kernel eBPF data views powered by bpftool, exposing live conntrack tables, policy maps, IP cache, LB maps, and drop analytics.

---

## What It Does

| Area | Capabilities |
|------|-------------|
| **Flow Monitoring** | Real-time Hubble flows, per-packet explanations, verdict coloring, WebSocket streaming |
| **Policy Management** | Visual rule builder, YAML editor, ML-powered AutoPolicy, simulate before applying |
| **Security** | Zero-trust policy generation, compliance audits (CIS/NIST/SOC2), anomaly detection |
| **Root Cause** | Packet drop analysis, policy correlation, one-click fixes, network healer |
| **Chaos Engineering** | Fault injection (loss/latency/DNS), circuit breaker, preset experiments |
| **Canary Deployments** | Progressive traffic shifting, health gates, auto-promote/rollback |
| **Multi-Cluster** | Cluster registration, health checks, policy sync, topology view |
| **Networking** | Load balancer, ingress/egress, IPAM, encryption, BGP, ClusterMesh, WireGuard |
| **Observability** | Service map, heatmap, DNS monitor, latency analysis, bandwidth, cost analytics |
| **Operations** | Diagnostics, troubleshooter, packet capture, SLOs, alerts, audit log, incidents |

---

## Quick Start

### Prerequisites

- Kubernetes cluster with Cilium installed
- `kubectl` configured
- Rust 1.75+ and Node.js 18+ (for building from source)

### Install & Run

```bash
git clone https://github.com/ssahani/cilium-flow.git
cd cilium-flow

# Build everything
cargo build --release                    # TUI binary (13MB)
cd web-api && cargo build --release      # API server
cd web-ui && npm ci && npm run build     # Web dashboard

# Deploy to remote K3s cluster
./scripts/deploy-k3s-test.sh <host> <user> <password> --test

# Or run locally
cilium-tui                               # Terminal UI
cilium-tui --skip-bootstrap              # Skip auto-setup
```

### Access

| Interface | URL | Description |
|-----------|-----|-------------|
| **Web Dashboard** | `http://<host>:9191` | 64-page React dashboard |
| **REST API** | `http://<host>:9191/api/v1/` | 75+ JSON endpoints |
| **WebSocket** | `ws://<host>:9191/api/v1/ws/metrics` | Real-time metrics stream |
| **Health Check** | `http://<host>:9191/health` | API health + subsystem status |
| **Presentation** | `http://<host>:9191/presentation.html` | Client presentation (13 slides) |

---

## Architecture

```
+------------------------------------------------------------------+
|                    Web Dashboard (React 19 + TypeScript)           |
|  64 Pages | 34 Components | WebSocket | Dark/Light Theme         |
+------------------------------------------------------------------+
|                    REST API (Rust + Axum)                          |
|  75+ Endpoints | JWT Auth | Redis Cache | Rate Limiting          |
+------------------------------------------------------------------+
|                    Terminal TUI (Rust + Ratatui)                   |
|  13 Tabs | 32K Lines | Live Monitoring | Packet Explainer        |
+------------------------------------------------------------------+
|  eBPF Maps (Aya)  |  Hubble Relay  |  Kubernetes API  |  Redis   |
+------------------------------------------------------------------+
|  bpftool (real kernel data: 117 progs, 132 maps, 12K CT entries) |
+------------------------------------------------------------------+
|              Cilium Agent  |  Linux Kernel eBPF Datapath          |
+------------------------------------------------------------------+
```

---

## Web Dashboard

64 pages organized in 7 navigation groups:

### Overview
Dashboard, Flows, Topology, Endpoints, Nodes, Events, Metrics

### Observability
Service Map, Heatmap, DNS Monitor, Latency Analysis, Bandwidth, Cost Analytics, Forecasting, Service Dependencies

### Security
Policies, Policy Editor, Visual Rule Builder, Policy Templates, Anomaly Detection, Compliance, Security Dashboard, Pod Security, RBAC Visualizer, Cilium Status, Identities

### Intelligence
AutoPolicy Engine, Chaos Engineering, Canary Deployments, Network Healer, Root Cause Analysis, eBPF Profiler

### Operations
Cluster Health, Alerts, Audit Log, SLO Dashboard, Incident Timeline, Change Log, Diagnostics, Troubleshooter, Node Drain, Settings

### eBPF Data
Conntrack Table (12K+ entries), Policy Map, IP Cache (35 entries), LB Map, Drop Analytics (16 categories), eBPF Profiler with Map Explorer (117 programs, 132 maps)

### Networking
ClusterMesh, BGP Peering, Load Balancer, Ingress Gateway, Egress Gateway, IPAM, Encryption, WireGuard, Network Interfaces, Traffic Mirror, Packet Capture, Flow Exporter, Service Mesh, KubeProxy Replacement

### Dashboard Features

Every view includes:
- **Auto-refresh** (30s interval with toggle)
- **Data freshness indicator** (color aging: green/yellow/red)
- **Export** (CSV/JSON download)
- **Empty state** messaging
- **Error handling** with retry

### Policy Management

| Feature | How |
|---------|-----|
| **Create (YAML)** | Monaco editor with syntax highlighting and validation |
| **Create (Visual)** | Form-based rule builder with live YAML preview |
| **Create (Template)** | Pre-built policy templates library |
| **Create (ML)** | AutoPolicy engine learns traffic and generates policies |
| **Edit/Update** | Click pencil icon, modify, apply |
| **Delete** | Single or bulk (multi-select checkboxes) |
| **Simulate** | Dry-run impact analysis before applying |
| **Validate** | Schema validation with error details |

---

## REST API

75+ endpoints serving JSON over HTTP with JWT authentication, including 11 eBPF-specific endpoints querying real kernel data via bpftool.

```bash
# Health check
curl http://localhost:9191/health

# List flows
curl http://localhost:9191/api/v1/flows

# List policies
curl http://localhost:9191/api/v1/policies

# Generate policy from traffic
curl -X POST http://localhost:9191/api/v1/modules/autopolicy/generate \
  -H 'Content-Type: application/json' \
  -d '{"namespace":"default","observation_duration":"5m"}'
```

See full endpoint list in [docs/WEB_APP_ARCHITECTURE.md](docs/WEB_APP_ARCHITECTURE.md).

---

## Terminal TUI

13 interactive tabs with vim-style navigation:

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle tabs |
| `?` | Help overlay |
| `q` | Quit |
| `e` | Explain selected packet |
| `d` | Detect problems (Healer) |
| `s` | Simulate / Stop |
| `t` | Enter time-travel mode |

---

## Testing

```bash
cargo test              # 961 tests (0 failures)
cargo check             # 0 errors, 0 warnings
cd web-api && cargo test  # API tests
cd web-ui && npm test     # UI tests
```

---

## Deployment Options

| Method | Command |
|--------|---------|
| **Remote K3s** | `./scripts/deploy-k3s-test.sh <host> <user> <pass> --test` |
| **Docker** | `docker compose -f deployments/docker-compose.yaml up` |
| **Helm** | `helm install cilium-vision ./deployments/k8s/chart` |
| **Systemd** | `bash install.sh setup-services && bash install.sh start` |
| **Binary** | `cargo build --release && cp target/release/cilium-tui /usr/local/bin/` |

### Environment Variables

```bash
HOST=0.0.0.0              # API listen address
PORT=9191                  # API port
REDIS_URL=redis://localhost:6379
JWT_SECRET=<min-32-chars>  # Required for auth
HUBBLE_ADDRESS=localhost:4245
UI_DIST_DIR=/var/lib/cilium-vision/ui  # Path to built web UI
AUTH_DISABLED=true         # Dev mode only
```

---

## Security

- JWT authentication (HS256, 32+ char secret)
- RBAC-ready middleware
- Input validation on all kubectl-bound fields
- CORS with explicit origin allowlist
- Rate limiting
- Confirmation dialogs on destructive operations
- Secure temp files via `tempfile::NamedTempFile`

---

## Roadmap

### v1.1 - Network Intelligence (Next)
- [ ] **Live flow WebSocket in Flows page** - Stream flows in real-time instead of polling
- [ ] **Policy diff viewer** - Side-by-side YAML comparison before/after updates
- [ ] **Anomaly detection ML pipeline** - Real anomaly scoring from Hubble metrics (currently uses baseline stats)
- [ ] **eBPF program hot-reload from web UI** - Upload and attach eBPF programs via the dashboard
- [ ] **Topology graph with D3 force layout** - Interactive node-link diagram with traffic volume edges

### v1.2 - Multi-Cluster & Scale
- [ ] **Multi-cluster dashboard** - Aggregate metrics across clusters in a single pane
- [ ] **Cross-cluster policy sync UI** - Visual diff and push policies between clusters
- [ ] **Pagination on all large data views** - Server-side pagination for Flows, Events, Endpoints
- [ ] **gRPC Hubble integration** - Direct gRPC to Hubble relay instead of CLI fallback
- [ ] **Prometheus metrics export** - Expose platform metrics for Grafana dashboards

### v1.3 - Advanced Security
- [ ] **Network policy recommendation engine** - Suggest least-privilege policies based on 30-day traffic history
- [ ] **CVE-aware policy generation** - Cross-reference container CVEs with network exposure
- [ ] **Compliance report PDF export** - Generate downloadable CIS/NIST/SOC2 compliance reports
- [ ] **Secret detection in DNS** - Flag DNS queries to known bad domains or data exfiltration patterns
- [ ] **mTLS enforcement dashboard** - Track which services have mutual TLS enabled vs plaintext

### v1.4 - Operations & Automation
- [ ] **Runbook automation** - Define remediation playbooks triggered by alerts
- [ ] **GitOps policy sync** - Watch a Git repo for policy changes and auto-apply
- [ ] **Slack/PagerDuty integration** - Send alerts to incident management tools
- [ ] **Scheduled chaos experiments** - Cron-based chaos testing with result history
- [ ] **Capacity planning** - Predict when IP pools, conntrack tables, or policy maps will fill

### v1.5 - Platform
- [ ] **Multi-tenancy** - Namespace-scoped views and RBAC per user/team
- [ ] **Plugin system** - Custom dashboard widgets and API extensions
- [ ] **OpenTelemetry integration** - Correlate network events with application traces
- [ ] **Mobile-responsive dashboard** - Touch-friendly tables and navigation
- [ ] **SSO / OIDC authentication** - Integrate with corporate identity providers

---

## Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| Rust | 1.75 | latest stable |
| Node.js | 18 | 22+ |
| Kubernetes | 1.25 | 1.28+ |
| Cilium | 1.14 | 1.19+ |
| Redis | 6.0 | 7.0+ |
| API Memory | 2 MB | 10 MB |
| TUI Binary | 13 MB | - |

---

## License

Apache License 2.0

---

## Acknowledgments

- [Cilium](https://cilium.io/) - eBPF-based networking
- [Hubble](https://docs.cilium.io/en/stable/observability/hubble/) - Network observability
- [Ratatui](https://ratatui.rs/) - Terminal UI framework
- [Aya](https://aya-rs.dev/) - Rust eBPF library
- [Axum](https://github.com/tokio-rs/axum) - Rust web framework
- [React](https://react.dev/) - UI framework
- [Tailwind CSS](https://tailwindcss.com/) - Utility-first CSS
