---
title: Features
sidebar_position: 3
---

## Core Observability (Tabs 0-4)

### Flows
Live packet stream from Hubble with verdict coloring (FORWARDED/DROPPED), source/destination pods, namespaces, and IPs. Navigate with arrow keys, press `e` to explain any packet. Auto-refresh every 30s with DataFreshness indicator. Export CSV/JSON.

### Connections
Enriched connection tracking combining eBPF conntrack data with Kubernetes pod metadata. Shows active TCP/UDP connections with identity resolution. Auto-refresh every 30s with DataFreshness indicator. Export CSV/JSON.

### Endpoints
Auto-discovered Cilium endpoints with labels, identity numbers, and policy status. Data sourced from the Kubernetes API. Auto-refresh every 30s with DataFreshness indicator. Export CSV/JSON.

### Policies
Active CiliumNetworkPolicy and NetworkPolicy resources visualized with rule counts and target selectors. Full CRUD: create (YAML editor, Visual Rule Builder, templates, ML-generated), edit/update, delete, and bulk delete. Auto-refresh every 30s. Export CSV/JSON.

### Metrics
Aggregate eBPF datapath metrics: forwarded/dropped counts, policy verdict breakdown, endpoint health. Auto-refresh every 30s with DataFreshness indicator. Export CSV/JSON.

---

## Intelligence Modules (Tabs 5-9)

### Healer (Tab 5)
Detects network problems by analyzing eBPF drop counters and proposes fixes.

| Key | Action |
|-----|--------|
| `d` | Run problem detection |

Detects: PolicyDenied drops, unreachable backends, MTU issues, DNS failures.
Proposes: Allow rules, endpoint repairs, configuration changes.
Mode: dry-run by default (no changes applied without confirmation).

### AutoPolicy (Tab 6)
Learns traffic patterns from conntrack data and generates CiliumNetworkPolicy YAML with ML confidence scores.

| Key | Action |
|-----|--------|
| `u` | Update learning from live data |
| `g` | Generate policies from learned patterns |
| `v` | Toggle policy detail view |
| `a` | Apply selected policy via kubectl |
| `r` | Rollback selected policy |
| `A` | Batch apply all unapplied policies |
| `R` | Batch rollback all applied policies |

Confidence scoring (7 features): temporal stability, traffic volume, port trust, namespace isolation, bidirectional traffic, protocol consistency, label specificity.

### RootCause (Tab 7)
Analyzes packet drops to identify root causes and offers one-click fixes.

| Key | Action |
|-----|--------|
| Up/Down | Navigate fix list |
| `a` | Apply selected fix (with confirmation) |

Built-in fixes: allow-8080 policy, DNS egress policy, MTU adjustment, DB access policy.

### Simulator (Tab 8)
Dry-run policy changes against historical flow data with impact analysis.

| Key | Action |
|-----|--------|
| Up/Down | Select scenario |
| `s` | Run simulation |
| `c` | Clear results |

7 built-in scenarios: AddPolicy, RemovePolicy, ModifyPolicy, BlockTraffic, AllowTraffic, BlockExternalIP, DefaultDeny.
Output: flows affected, services impacted, risk level (Low/Medium/High/Critical).

### Replay (Tab 9)
Record flows to disk and replay with time-travel controls.

| Key | Action |
|-----|--------|
| `r` | Refresh recording list |
| `t` | Enter time-travel mode |
| Space | Play/pause |
| Left/Right | Step backward/forward |
| `[` / `]` | Jump to prev/next event |
| `+` / `-` | Increase/decrease speed |
| Esc | Exit time-travel |

---

## Operations Modules (Tabs 10-12)

### Chaos Engineering (Tab 10)
Controlled fault injection using tc-netem rules applied via kubectl exec.

| Key | Action |
|-----|--------|
| Up/Down | Navigate presets/experiments |
| Enter | Run selected preset (with confirmation) |
| `v` | Toggle presets / active experiments |
| `s` | Stop selected experiment |
| `S` | Stop all experiments |
| `b` | Toggle circuit breaker |

**7 Presets:**

| Preset | Type | Parameters |
|--------|------|-----------|
| Network Partition | PacketDrop | 20% loss |
| Latency Spike | Latency | 500ms +/- 50ms jitter |
| DNS Outage | DNSFailure | 30% failure rate |
| Connection Reset | ConnectionKill | 15% kill rate |
| Bandwidth Limit | Bandwidth | 10 Mbps |
| Packet Corruption | PacketCorruption | 5% corruption |
| Total Partition | PacketDrop | 100% loss |

**Safety limits** (configurable): max 50% drop rate, max 5000ms latency, DNS max 90%, circuit breaker for emergency shutdown, 5-minute auto-cleanup.

### Canary Deployments (Tab 11)
Progressive traffic shifting using Cilium L7 service annotations.

| Key | Action |
|-----|--------|
| Up/Down | Navigate canaries |
| `p` | Promote (with confirmation) |
| `r` | Rollback (with confirmation) |
| `+` | Progress traffic by 10% |
| `d` | Toggle detail view |

Default config: start at 10%, step by 10%, auto-promote at 99% success, auto-rollback below 90%, 1 hour max duration.

View shows: traffic split bars, canary health gauge, success rate, latency comparison, error rate.

### Multi-Cluster Autopilot (Tab 12)
Cross-cluster orchestration and monitoring.

| Key | Action |
|-----|--------|
| Up/Down | Navigate clusters |
| `v` | Cycle views (Clusters / Topology / Syncs / Placements) |
| `h` | Run health check on all clusters |

Health check queries `kubectl get nodes --context <name>` for each registered cluster, updates node readiness, derives cluster state (Active/Degraded/Unreachable).

4 views: cluster list with provider/region/state, topology map with connected pairs, active policy syncs, workload placement recommendations.

---

## Supplementary Modules (Library APIs)

These modules are implemented as library APIs and can be integrated into custom tooling.

### Anomaly Detection
Multi-algorithm threat detection: Z-Score, Isolation Forest, LSTM, MACD, Seasonal Hybrid ESD.

### Advanced eBPF
Performance profiling (CPU/memory/network/syscall/lock), packet filtering, CO-RE support, flame graph generation.

### Security & Compliance
Zero-trust policy generation, compliance framework auditing (PCI-DSS, SOC2, HIPAA, GDPR, ISO 27001, NIST).

### Developer Tools
Traffic shadowing, request replay, environment mirroring.

---

## Web Dashboard (web-ui)

Full-featured React 19 web application with 64 views, 34 shared components, and 6 custom hooks.

### Design
- Dark-first theme (slate-950 base) with light mode toggle
- Gradient icon headers, card-glow hover effects, shimmer skeletons
- Navbar with hover dropdowns, WebSocket connection indicator, global search
- Terminal-style code views with traffic light dots

### Key Views
| Category | Count | Highlights |
|----------|-------|-----------|
| Overview | 7 | Dashboard with live metrics, cluster health, Cilium agent status |
| Observability | 12 | Flows, topology, service map, heatmap, latency, DNS, bandwidth |
| Security | 15 | Policies, policy editor (Monaco), anomalies, encryption, RBAC, compliance |
| Intelligence | 6 | AutoPolicy ML engine, healer, root cause, diagnostics, forecasting |
| Operations | 11 | Chaos engineering, canary, replay, packet capture, multi-cluster, eBPF |
| eBPF Data | 5 | Conntrack table, policy map, IP cache, LB map, drop analytics |
| Networking | 8 | Load balancer, ingress/egress, service mesh, IPAM, cost analytics |

### Policy Management
- **Visual Rule Builder**: Form-based policy creation without writing YAML
- **YAML Editor**: Monaco-powered editor with syntax highlighting and validation
- **Template Library**: Pre-built policy templates for common use cases
- **ML-Generated Policies**: AutoPolicy engine generates policies from observed traffic
- **Full CRUD**: Create, edit/update, delete, and bulk delete operations
- **YAML Injection Prevention**: `yaml_escape` helper sanitizes all user input

### Real-Time Features
- **WebSocket Streaming**: Live flow and metrics data via WebSocket connections
- **Auto-Refresh**: 30-second refresh interval on all data views with DataFreshness indicator
- **Export**: CSV and JSON export on all tabular data views

### Reusable Components (34)
StatCard, ChartContainer, SortableTable, Badge, ProgressBar, ScoreGauge, Accordion, ToggleSwitch, AlertsList, EmptyState, Toast, GlobalSearch, DataFreshness, and more.

### Tech Stack
React 19, TypeScript 5.8, Tailwind CSS 3.4, Zustand 5, TanStack React Query, Recharts, D3 (d3-force, d3-selection, d3-drag), Monaco Editor, Vite 6, Vitest (68 tests).

---

## eBPF Kernel Data

Real kernel eBPF data views powered by bpftool queries (not mocked data):

| View | Data Source | Details |
|------|------------|---------|
| **Conntrack Table** | `bpftool map dump` | 12,066 active connection tracking entries |
| **Policy Map** | `bpftool map dump` | Per-endpoint policy verdict maps |
| **IP Cache** | `bpftool map dump` | 35 identity-to-CIDR mappings |
| **LB Map** | `bpftool map dump` | Load balancer service/backend maps |
| **Drop Analytics** | `bpftool map dump` | 16 drop reason categories with counts |
| **eBPF Profiler** | `bpftool prog/map list` | 117 programs, 132 maps, Map Explorer |

All data is queried from the running kernel via 11 dedicated eBPF API endpoints.

---

## Web API (75+ REST Endpoints)

Rust/Axum backend serving the web dashboard with 75+ REST API endpoints, including 11 eBPF-specific endpoints querying real kernel data via bpftool.

### Authentication & Authorization
- **JWT Authentication**: Token-based auth on all protected endpoints
- **RBAC Enforcement**: `require_admin()` guard on all destructive operations (delete, bulk delete, apply, rollback)
- **Role-Based Access**: Admin and viewer roles with granular permission checks

### Security
- **Rate Limiting**: Per-IP request throttling with periodic cleanup of stale entries
- **WebSocket Connection Limit**: Maximum 100 concurrent WebSocket connections
- **Hubble Address Validation**: Startup validation of Hubble gRPC endpoint
- **YAML Injection Prevention**: `yaml_escape` helper sanitizes user-supplied strings
- **Concurrent Health Checks**: Parallel health probes with 3-second timeout
- **Input Validation**: RFC 1123 regex on kubectl-bound names/namespaces, flag injection prevention
- **Type-Safe Handlers**: All POST endpoints use typed `Deserialize` structs, no raw `Json<Value>`
- **Redis Caching**: Response caching for frequently accessed data

### API Categories
| Category | Endpoints | Highlights |
|----------|-----------|-----------|
| Flows & Observability | 12 | Live flows, connections, metrics, WebSocket streaming |
| Policies | 10 | CRUD, bulk delete, templates, visual builder, ML generation |
| Endpoints & Identity | 8 | Cilium endpoints, identity resolution, labels |
| Intelligence | 10 | AutoPolicy, healer, root cause, simulator, anomaly detection |
| eBPF Data | 11 | Conntrack, policy map, ipcache, LB map, drops, programs, maps |
| Operations | 8 | Chaos experiments, canary deployments, replay |
| Cluster & Infra | 8 | Multi-cluster, health checks, node status, agent info |
| Auth & Admin | 4 | Login, token refresh, RBAC management |
| Search & Export | 4 | Global search, CSV/JSON export |

---

## Platform Metrics

| Metric | Value |
|--------|-------|
| Rust tests | 961 passing, 0 failures |
| TUI tabs | 13 |
| Rust lines of code | 32,000+ |
| Web dashboard pages | 64 |
| Web components | 34 |
| Custom hooks | 6 |
| REST API endpoints | 75+ |
| Web UI tests | 68 passing |
| Compiler warnings | 0 (Rust + TypeScript) |
