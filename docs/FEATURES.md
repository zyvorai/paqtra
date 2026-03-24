# Cilium Flow - Features Guide

## Core Observability (Tabs 0-4)

### Flows
Live packet stream from Hubble with verdict coloring (FORWARDED/DROPPED), source/destination pods, namespaces, and IPs. Navigate with arrow keys, press `e` to explain any packet.

### Connections
Enriched connection tracking combining eBPF conntrack data with Kubernetes pod metadata. Shows active TCP/UDP connections with identity resolution.

### Endpoints
Auto-discovered Cilium endpoints with labels, identity numbers, and policy status. Data sourced from the Kubernetes API.

### Policies
Active CiliumNetworkPolicy and NetworkPolicy resources visualized with rule counts and target selectors.

### Metrics
Aggregate eBPF datapath metrics: forwarded/dropped counts, policy verdict breakdown, endpoint health.

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
