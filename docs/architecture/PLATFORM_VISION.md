# 🚀 CILIUM-VISION PLATFORM (v1 Design)

> *An eBPF-first autonomous network brain for Kubernetes.*

## Platform Architecture

```
                        ┌─────────────────────────────────┐
                        │        CILIUM-VISION UI         │
                        │  (TUI + Web + CLI + API)       │
                        └───────────────▲────────────────┘
                                        │
                     ┌──────────────────┼───────────────────┐
                     │        Cilium-Vision Control Plane   │
                     │  (Rust, distributed, HA-ready)       │
                     └───────▲────────────▲────────────▲────┘
                             │            │            │
                eBPF Maps ───┘            │            └── Kubernetes API
                                          │
                                   Hubble gRPC
```

## Module Status Overview

### ✅ COMPLETED (Phase 1 - Core)

#### Module A — Self-Healer (Autopilot)
**Status:** ✅ Production Ready
**Location:** `src/modules/healer/`

**Pipeline:**
```
Observe → Diagnose → Propose → (optional) Auto-Apply
```

**Capabilities:**
- DNS failure detection and auto-fix
- MTU mismatch detection and resolution
- Load balancer timeout handling
- Policy gap identification
- Connection tracking monitoring

**Example Output:**
```
Problem: web-1 → dns timeout
Cause: L4 policy drop
Fix: created allow-dns policy
```

---

#### Module B — Zero-Trust AutoPolicy
**Status:** ✅ Production Ready
**Location:** `src/modules/autopolicy/`

**Phase 1 — Learn (7 days default):**
- Records all traffic patterns
- Tracks: who → who, ports, protocols
- Builds communication graph

**Phase 2 — Generate least-privilege policy:**
```yaml
apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: auto-zero-trust
spec:
  endpointSelector:
    matchLabels:
      app: web
  egress:
    - toEndpoints:
        - matchLabels: { app: db }
      toPorts:
        - ports:
            - port: "5432"
              protocol: TCP
```

**Default:** Everything else = **deny**

---

#### Module C — Root-Cause Engine
**Status:** ✅ Production Ready
**Location:** `src/modules/rootcause/`

**Capabilities:**
For every drop, provides:
```
Packet: web-1 → db-1:5432

Drop reason:
- Evaluated in map: cilium_l4_policy
- No matching allow rule
- Missing port 5432

Suggested fix:
Add allow-web-to-db:5432
```

**Drop Reasons Supported:**
- PolicyDenied, PortNotAllowed
- ServiceNotFound, NoBackend, LBError
- CTStateMismatch, FragNeeded (MTU)
- InvalidPacket, UnknownDestination
- And 15+ more...

---

### 📋 PLANNED - Phase 2 (Automation)

#### Module D — Traffic Replay
**Status:** 📋 Not Started
**Priority:** High
**Estimated Effort:** 2-3 weeks

**Capabilities:**
```bash
# Record traffic
cilium-vision record --pod=web-1 --time=10s

# Replay in another cluster
cilium-vision replay \
  --from=cluster-A \
  --to=cluster-B \
  --pod=web-1

# Compare outcomes
Compare:
- Drops
- Latency
- Policy outcomes
```

**Use Cases:**
- Test policy changes safely
- Validate fixes before production
- Compare cluster behaviors
- Incident replay for debugging

---

#### Module E — What-If Simulator
**Status:** 📋 Not Started
**Priority:** **CRITICAL** (Next to implement)
**Estimated Effort:** 3-4 weeks

**Capabilities:**
User asks:
```
What if I block 8.8.8.8?
```

System answers:
```
Impact:
- 12 pods depend on it for DNS
- 2 services will fail health checks

Recommendation:
Use kube-dns instead.
```

**How It Works:**
1. Load current network state
2. Apply simulated policy change
3. Replay historical flows through simulated engine
4. Report impact analysis

**Features:**
- Policy impact prediction
- Service dependency analysis
- Risk assessment scoring
- Rollback recommendations

---

#### Module F — Performance Profiler
**Status:** 📋 Not Started
**Priority:** Medium
**Estimated Effort:** 3-4 weeks

**Capabilities:**
Per-pod breakdown:
```
Pod: web-1
Ingress BPF: 120µs
LB lookup:   35µs
Policy eval: 50µs
Conntrack:   20µs
Total:       225µs
```

**Derived from:**
- Cilium metrics
- eBPF maps
- Hubble flow latency

**Outputs:**
- Latency hotspots
- Policy evaluation overhead
- Network path analysis
- Optimization recommendations

---

### 📋 PLANNED - Phase 3 (Enterprise)

#### Module G — Cost Optimizer + Scheduler
**Status:** 📋 Not Started
**Priority:** Medium
**Estimated Effort:** 4-6 weeks

**Step 1 — Traffic Graph:**
```
web-1 ↔ redis-1 (heavy, cross-AZ)
web-2 ↔ redis-1 (heavy, cross-AZ)
```

**Step 2 — Recommendation:**
```
Move redis-1 to node-2
Reason: 80% clients already there
Savings: $150/month (reduced cross-AZ)
```

**Step 3 — Scheduler Plugin:**
A custom Kubernetes scheduler that:
- Co-locates chatty pods
- Minimizes cross-zone traffic
- Reduces network cost
- Optimizes for latency

**Cost Analysis:**
- Cross-AZ traffic breakdown
- NAT gateway costs
- Per-namespace spending
- Optimization opportunities

---

#### Module H — Multi-Cluster Control
**Status:** 📋 Not Started
**Priority:** High
**Estimated Effort:** 4-5 weeks

**Capabilities:**
Single dashboard for many clusters:
```
Compare A vs B:
Policy drift detected:
- Cluster A allows web→db:5432
- Cluster B blocks web→db:5432
```

**Features:**
- Unified policy view
- Drift detection
- Auto-sync policies
- Cross-cluster compliance
- Cluster mesh support

---

#### Module I — Drift Guard (GitOps)
**Status:** 📋 Not Started
**Priority:** High
**Estimated Effort:** 2-3 weeks

**Watches:**
- CiliumNetworkPolicy changes
- Actual Hubble behavior
- Git repository state

**On Drift:**
```
Cluster deviated from Git → auto-revert
```

**Integrations:**
- ArgoCD
- Flux
- Custom GitOps

**Modes:**
- Alert only
- Auto-revert
- Create PR for approval

---

#### Module J — Compliance Engine
**Status:** 📋 Not Started
**Priority:** Medium
**Estimated Effort:** 3-4 weeks

**Generates reports:**
```
CIS Network Compliance:
✔ Default deny everywhere
✔ DNS explicitly allowed
⚠ 3 namespaces allow all egress

Score: 87/100
```

**Standards Supported:**
- CIS Kubernetes Benchmark
- PCI-DSS
- SOC2
- ISO 27001
- NIST

**Features:**
- Continuous compliance monitoring
- Policy gap analysis
- Remediation suggestions
- Audit trail

---

### 📋 PLANNED - Phase 4 (Advanced)

#### Module K — L7 Security (HTTP/gRPC)
**Status:** 📋 Not Started
**Priority:** Medium
**Estimated Effort:** 4-5 weeks

**Enforce:**
- Block `/admin` from external
- Rate-limit abusive clients
- Block unsafe gRPC methods
- Validate JWT tokens

**Example auto-generated rule:**
```yaml
Deny POST /admin from non-internal pods
Allow GET /health from everywhere
```

**Features:**
- HTTP method filtering
- Path-based policies
- gRPC method control
- Rate limiting
- Header inspection

---

#### Module L — Time-Travel Debugging
**Status:** 📋 Not Started
**Priority:** Low
**Estimated Effort:** 3-4 weeks

**Capabilities:**
Users can ask:
```
Show me network state 2 hours ago
```

**System provides:**
- Historical topology
- Past policy decisions
- Flow replay
- Incident timeline

**Use Cases:**
- Incident response
- Root-cause of past issues
- Compliance audits
- Forensic analysis

---

#### Module M — Fault Injection (Chaos)
**Status:** 📋 Not Started
**Priority:** Low
**Estimated Effort:** 3-4 weeks

**Inject via eBPF:**
- Packet loss (e.g., 10%)
- Latency (e.g., +50ms)
- DNS failures
- Connection resets

**Example:**
```bash
cilium-vision chaos inject \
  --from web \
  --to db \
  --latency 20ms \
  --duration 5m
```

**Use Cases:**
- Chaos engineering
- Resilience testing
- SLA validation
- Failure mode testing

---

## Shared Data Layer (The Brain)

### Continuous Ingestion

| Source               | What it feeds          |
| -------------------- | ---------------------- |
| `cilium_policy`      | Root-cause + simulator |
| `cilium_conntrack`   | Latency + failures     |
| `cilium_lb`          | Service behavior       |
| `cilium_ipcache`     | Identity graph         |
| `cilium_drop_reason` | Debugging              |
| Hubble flows         | Learning + replay      |
| Kubernetes API       | Policy + topology      |
| Metrics              | Performance            |

### Event Store Structure

```
event_store/
 ├── flows/
 │   ├── ingress
 │   ├── egress
 │   └── dropped
 ├── drops/
 │   ├── by_reason
 │   └── by_namespace
 ├── latency/
 │   ├── p50, p95, p99
 │   └── per_service
 ├── policy_decisions/
 │   ├── allow
 │   ├── deny
 │   └── redirect
 └── topology_snapshots/
     ├── hourly
     └── on_change
```

---

## User Interfaces

### A. CLI

```bash
# Status
cilium-vision status

# Secure namespace with zero-trust
cilium-vision secure ns prod

# Replay traffic
cilium-vision replay --from cluster-A --to cluster-B

# Simulate policy change
cilium-vision simulate --policy new-policy.yaml

# Diagnose pod issues
cilium-vision diagnose pod/web-1

# Record traffic
cilium-vision record --pod web-1 --time 60s

# Cost analysis
cilium-vision cost --namespace prod
```

### B. TUI (Operator Console)

**Screens:**
1. **Topology** - Visual service mesh
2. **Endpoints** - Pod/service list with health
3. **Policies** - CNP viewer and editor
4. **Live Flows** - Real-time traffic
5. **Node Health** - Cilium agent status
6. **Profiler** - Performance analysis
7. **Simulator** - What-if analysis
8. **Healer** - Auto-fix dashboard
9. **AutoPolicy** - Learning progress
10. **RootCause** - Drop analysis

### C. Web UI

**Visual Graph:**
```
web → db (green, 1000 req/s)
web → external (red risk warning)
api → cache (yellow, high latency)
```

**Dashboards:**
- Network topology
- Policy coverage
- Security score
- Cost breakdown
- Compliance status

### D. gRPC API

For programmatic access:
```protobuf
service CiliumVision {
  rpc AnalyzeDrops(DropQuery) returns (DropAnalysis);
  rpc SimulatePolicy(PolicyChange) returns (Impact);
  rpc GetTopology(TopologyRequest) returns (NetworkGraph);
  rpc ApplyFix(Fix) returns (Result);
}
```

---

## Deployment Model

### Single Cluster

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: cilium-vision-controller
spec:
  replicas: 2  # HA
  template:
    spec:
      containers:
      - name: controller
        image: cilium-vision:latest
        volumeMounts:
        - name: ebpf-maps
          mountPath: /sys/fs/bpf
      - name: hubble-client
        image: cilium-vision:latest
        command: ["hubble-relay-client"]
```

### Multi-Cluster (Hub and Spoke)

```
┌───────────────────────────────────────┐
│     Cilium-Vision Central Hub         │
│  (Aggregates all cluster data)        │
└────────┬─────────┬──────────┬─────────┘
         │         │          │
    ┌────▼────┐ ┌──▼───┐  ┌──▼───┐
    │ Cluster │ │Cluster│ │Cluster│
    │    A    │ │   B   │ │   C  │
    │  Agent  │ │ Agent │ │Agent │
    └─────────┘ └───────┘ └──────┘
```

---

## Implementation Roadmap

### ✅ Phase 1 — Core (COMPLETED)
**Timeline:** Months 1-4
**Status:** ✅ DONE

- [x] eBPF reader foundation
- [x] Hubble ingestion
- [x] Root-cause engine
- [x] Self-healer
- [x] Zero-trust autopolicy
- [x] Basic TUI

### 🔄 Phase 2 — Automation (IN PROGRESS)
**Timeline:** Months 5-7
**Status:** 🔄 Starting

**Next Up:**
- [ ] **What-if simulator** (CRITICAL - START HERE)
- [ ] Traffic replay
- [ ] Performance profiler

**Expected Completion:** Month 7

### 📋 Phase 3 — Enterprise
**Timeline:** Months 8-10
**Status:** 📋 Planned

- [ ] Multi-cluster control
- [ ] Drift guard (GitOps)
- [ ] Compliance engine
- [ ] Cost optimizer

**Expected Completion:** Month 10

### 📋 Phase 4 — Advanced
**Timeline:** Months 11-16
**Status:** 📋 Planned

- [ ] Scheduler plugin
- [ ] L7 security
- [ ] Time-travel debugging
- [ ] Fault injection

**Expected Completion:** Month 16

---

## Technical Stack

### Core Platform
- **Language:** Rust (performance + safety)
- **Async Runtime:** Tokio
- **eBPF Access:** libbpf-rs
- **K8s Client:** kube-rs
- **gRPC:** Tonic

### UI Layer
- **TUI:** Ratatui
- **Web:** Axum + React (planned)
- **CLI:** Clap

### Storage
- **Time-series:** InfluxDB or custom
- **Events:** In-memory + disk persistence
- **State:** etcd (for HA)

---

## Success Metrics

### Technical
- Drop analysis accuracy: >95%
- Auto-fix success rate: >80%
- Policy learning completeness: >90%
- Simulator prediction accuracy: >85%

### Business
- Time to diagnose: <5 minutes (vs hours)
- Cost reduction: 20-40% (cross-AZ traffic)
- Security posture: 100% zero-trust
- Compliance: Automated reports

---

## Current Status Summary

**Modules Built:** 3/13 (23%)
**Code Complete:** ~3,500 / ~15,000 lines (23%)
**Tests Passing:** 25/25 (100% of written tests)
**Documentation:** 5 comprehensive guides

**Next Milestone:**
🎯 **Build What-If Simulator** (Module E)
- Critical for safety before auto-apply
- Enables policy testing
- Risk-free change validation

---

## Questions & Decisions

### What to build next?

Based on priority and dependencies:

**Option 1: What-If Simulator** (RECOMMENDED)
- Highest impact
- Enables safe policy changes
- Foundation for other modules
- 3-4 week effort

**Option 2: Traffic Replay**
- Complements simulator
- Enables testing
- 2-3 week effort

**Option 3: Performance Profiler**
- Independent module
- High value for ops teams
- 3-4 week effort

**Recommendation:** Start with **What-If Simulator** as it's the safety layer for all automation.

---

**Last Updated:** 2026-02-05
**Platform Version:** v2.0 (Intelligence Layer)
**Next Target:** v2.5 (Simulation + Replay)
