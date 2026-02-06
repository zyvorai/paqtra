# 🚀 Cilium Vision - Roadmap

## From TUI to eBPF-Native Control Plane

This document outlines the evolution from `cilium-tui` to `cilium-vision` - a comprehensive eBPF-native network brain.

## Current State (v2)

✅ **Implemented:**
- Zero-touch bootstrapping
- Automatic Cilium installation/upgrade
- Flow monitoring (CLI + optional gRPC)
- Endpoint discovery
- Network policy automation
- TUI with 4 tabs

## Vision State (v3+)

🎯 **Target: Full Control Plane**

Transform into an intelligent system that:
- **Observes** at eBPF layer
- **Explains** network behavior
- **Simulates** changes
- **Heals** automatically
- **Optimizes** performance

---

## Architecture Evolution

### Current (v2):
```
cilium-tui
    ↓
Hubble gRPC/CLI
    ↓
Cilium Agent
    ↓
eBPF Maps
```

### Target (v3+):
```
cilium-vision (Control Plane)
    ↓
┌────────────────────────────────┐
│ eBPF Reader (Direct Map Access)│
├────────────────────────────────┤
│ • cilium_policy                │
│ • cilium_conntrack             │
│ • cilium_lb                    │
│ • cilium_ipcache               │
│ • cilium_metrics               │
│ • cilium_drop_reason           │
└────────────────────────────────┘
    ↓
Modules (7 subsystems)
    ↓
TUI + API + Operator
```

---

## Module Roadmap

### Phase 1: Foundation (v3.0)

**Module: eBPF Reader Layer**
- [ ] Direct eBPF map reading
- [ ] Policy map parser
- [ ] Conntrack map parser
- [ ] LB map parser
- [ ] IPCache map parser
- [ ] Metrics map parser
- [ ] Drop reason map parser

**Effort**: 2-3 weeks
**Files**: `src/ebpf/reader.rs`, `src/ebpf/parser.rs`

### Phase 2: Intelligence (v3.1-3.3)

#### Module A: Self-Healer (v3.1)
**Priority**: ⭐⭐⭐ HIGH

Automatically fixes common issues.

**Features:**
- [ ] DNS drop detection & auto-fix
- [ ] MTU mismatch detection
- [ ] LB timeout healing
- [ ] Policy gap detection
- [ ] Conntrack tuning

**Example:**
```rust
// Detects DNS drops and creates allow-dns policy
healer.detect_dns_drops().await?;
healer.apply_fix(Fix::AllowDNS).await?;
```

**Effort**: 3-4 weeks
**Files**: `src/modules/healer/mod.rs`, `src/modules/healer/dns.rs`, etc.

#### Module B: Zero-Trust Autopolicy (v3.2)
**Priority**: ⭐⭐⭐ HIGH

Learns traffic and generates policies.

**Features:**
- [ ] Traffic learning (configurable duration)
- [ ] Pattern recognition
- [ ] Policy generation
- [ ] Minimal privilege policies
- [ ] Audit mode

**Example:**
```rust
// Learn for 7 days
autopolicy.start_learning(Duration::from_days(7)).await?;

// Generate policies
let policies = autopolicy.generate_policies().await?;

// Apply
autopolicy.apply(policies, AuditMode::Enabled).await?;
```

**Effort**: 4-5 weeks
**Files**: `src/modules/autopolicy/`

#### Module C: Root-Cause Engine (v3.3)
**Priority**: ⭐⭐ MEDIUM

Explains why packets drop.

**Features:**
- [ ] Drop reason parsing
- [ ] Policy evaluation replay
- [ ] Rule suggestion
- [ ] Impact analysis

**Example Output:**
```
Drop: web-1 → db-1:5432
Reason: No matching L4 policy
Suggestion: Add allow-web-to-db:5432
```

**Effort**: 2-3 weeks
**Files**: `src/modules/rootcause/`

### Phase 3: Advanced (v3.4-3.7)

#### Module D: Traffic Replay (v3.4)
**Priority**: ⭐⭐ MEDIUM

Record and replay traffic patterns.

**Features:**
- [ ] eBPF-level traffic capture
- [ ] Storage in efficient format
- [ ] Cross-cluster replay
- [ ] Performance comparison

**Effort**: 4-5 weeks
**Files**: `src/modules/replay/`

#### Module E: What-If Simulator (v3.5)
**Priority**: ⭐⭐ MEDIUM

Simulate policy changes before applying.

**Features:**
- [ ] Policy simulation engine
- [ ] Impact prediction
- [ ] Dependency analysis
- [ ] Risk scoring

**Effort**: 3-4 weeks
**Files**: `src/modules/simulator/`

#### Module F: Performance Profiler (v3.6)
**Priority**: ⭐ LOW

Per-pod eBPF performance breakdown.

**Features:**
- [ ] BPF time tracking
- [ ] Hot path identification
- [ ] Optimization suggestions
- [ ] Comparative analysis

**Effort**: 2-3 weeks
**Files**: `src/modules/profiler/`

#### Module G: Cost Optimizer (v3.7)
**Priority**: ⭐ LOW

Smart pod placement based on traffic.

**Features:**
- [ ] Traffic graph analysis
- [ ] Co-location recommendations
- [ ] Scheduler integration
- [ ] Cost calculation

**Effort**: 3-4 weeks
**Files**: `src/modules/optimizer/`

### Phase 4: Optional (v4.0+)

#### Module H: Sidecarless Mesh
**Priority**: ⭐ OPTIONAL

Service mesh without sidecars.

**Features:**
- [ ] eBPF-based mTLS
- [ ] Traffic splitting
- [ ] Canary releases
- [ ] A/B testing

**Effort**: 6-8 weeks
**Files**: `src/modules/mesh/`

---

## Integration Points

### With Existing hyper2kvm
- Pre-migration network validation
- Safe migration recommendations
- Post-migration verification

### With Existing v9s
- Built-in network intelligence
- Auto-tuning for VMs
- Policy enforcement

### With KubeVirt
- VM network optimization
- Enhanced security
- Performance tuning

---

## Technical Implementation

### New Dependencies

```toml
[dependencies]
# Existing (keep all)
kube = "0.97"
tokio = "1.42"
ratatui = "0.29"
# ... etc

# New for eBPF
aya = "0.12"              # eBPF framework
redbpf = "2.3"            # Alternative eBPF
libbpf-rs = "0.24"        # libbpf bindings

# New for ML/Analysis
petgraph = "0.6"          # Graph analysis
ndarray = "0.15"          # Numerical computing
linfa = "0.7"             # ML algorithms

# New for Storage
rocksdb = "0.21"          # For replay storage
bincode = "1.3"           # Fast serialization
```

### Project Structure

```
cilium-vision/
├── src/
│   ├── main.rs                    # Entry point
│   │
│   ├── ebpf/                      # NEW: eBPF layer
│   │   ├── mod.rs
│   │   ├── reader.rs              # Direct map reading
│   │   ├── parser.rs              # Map parsers
│   │   └── simulator.rs           # In-memory simulation
│   │
│   ├── modules/                   # NEW: Intelligence modules
│   │   ├── healer/                # Self-healing
│   │   │   ├── mod.rs
│   │   │   ├── dns.rs
│   │   │   ├── mtu.rs
│   │   │   └── policy.rs
│   │   ├── autopolicy/            # Zero-trust learning
│   │   │   ├── mod.rs
│   │   │   ├── learner.rs
│   │   │   └── generator.rs
│   │   ├── rootcause/             # Drop analysis
│   │   ├── replay/                # Traffic replay
│   │   ├── simulator/             # What-if
│   │   ├── profiler/              # Performance
│   │   └── optimizer/             # Cost optimization
│   │
│   ├── bootstrap/                 # EXISTING
│   ├── cilium/                    # EXISTING
│   ├── endpoints/                 # EXISTING
│   ├── hubble/                    # EXISTING
│   ├── kubernetes/                # EXISTING
│   ├── policies/                  # EXISTING
│   └── tui/                       # EXISTING + enhanced
│       ├── mod.rs
│       ├── flows.rs               # Tab 1 (existing)
│       ├── endpoints.rs           # Tab 2 (existing)
│       ├── policies.rs            # Tab 3 (existing)
│       ├── metrics.rs             # Tab 4 (existing)
│       ├── healer.rs              # Tab 5 (NEW)
│       ├── autopolicy.rs          # Tab 6 (NEW)
│       ├── rootcause.rs           # Tab 7 (NEW)
│       └── profiler.rs            # Tab 8 (NEW)
```

---

## TUI Evolution

### Current: 4 Tabs
1. Flows
2. Endpoints
3. Policies
4. Metrics

### Target: 8+ Tabs
1. Flows (existing)
2. Endpoints (existing)
3. Policies (existing)
4. Metrics (existing)
5. **Healer** (auto-fix status)
6. **AutoPolicy** (learning progress)
7. **Root Cause** (drop analysis)
8. **Profiler** (performance)
9. **Simulator** (what-if)
10. **Optimizer** (recommendations)

---

## Development Phases

### Phase 1: eBPF Foundation (4 weeks)
**Goal**: Direct eBPF map access

- [ ] Setup eBPF libraries
- [ ] Implement map readers
- [ ] Parse common maps
- [ ] Add tests
- [ ] Integration with existing TUI

**Milestone**: Display eBPF data in existing tabs

### Phase 2: Self-Healer (4 weeks)
**Goal**: First intelligent module

- [ ] Implement detection logic
- [ ] Build fix generators
- [ ] Add TUI tab for healer
- [ ] Auto-apply mode
- [ ] Dry-run mode

**Milestone**: Auto-fix DNS drops

### Phase 3: AutoPolicy (5 weeks)
**Goal**: Zero-trust learning

- [ ] Traffic recording
- [ ] Pattern analysis
- [ ] Policy generation
- [ ] Audit mode
- [ ] TUI visualization

**Milestone**: Generate policies from observed traffic

### Phase 4: Root Cause (3 weeks)
**Goal**: Drop explanations

- [ ] Drop reason parsing
- [ ] Policy replay
- [ ] Suggestion engine
- [ ] TUI integration

**Milestone**: Explain all drops

### Phase 5+: Advanced Modules (12+ weeks)
- Replay system
- Simulator
- Profiler
- Optimizer
- Mesh (optional)

---

## Success Metrics

### v3.0 (Foundation)
- [ ] Read all eBPF maps
- [ ] 95% accuracy in parsing
- [ ] <10ms read latency

### v3.1 (Healer)
- [ ] Auto-fix 80% of DNS issues
- [ ] <5s time to detect
- [ ] Zero false positives

### v3.2 (AutoPolicy)
- [ ] Generate 90% accurate policies
- [ ] Reduce manual policy writing by 70%
- [ ] Zero production incidents

### v3.3 (Root Cause)
- [ ] Explain 95% of drops
- [ ] <100ms analysis time
- [ ] Actionable suggestions

---

## Timeline

| Phase | Duration | Completion |
|-------|----------|------------|
| Phase 1: eBPF Foundation | 4 weeks | Q2 2026 |
| Phase 2: Self-Healer | 4 weeks | Q2 2026 |
| Phase 3: AutoPolicy | 5 weeks | Q3 2026 |
| Phase 4: Root Cause | 3 weeks | Q3 2026 |
| Phase 5: Replay | 5 weeks | Q3 2026 |
| Phase 6: Simulator | 4 weeks | Q4 2026 |
| Phase 7: Profiler | 3 weeks | Q4 2026 |
| Phase 8: Optimizer | 4 weeks | Q4 2026 |

**Total**: ~8 months to full vision

---

## Community Impact

This would be a **signature contribution** to:
- Cilium ecosystem
- eBPF community
- Kubernetes networking
- Zero-trust security

Potential for:
- Conference talks
- Research papers
- Industry adoption
- CNCF project

---

## Next Steps

1. **Read this roadmap** - Understand the vision
2. **Choose starting point**:
   - Full skeleton now?
   - Start with Healer module?
   - Build eBPF reader first?
3. **Let's implement!**

---

## Questions to Decide

1. **Scope**: Full vision or phased?
2. **Timeline**: Aggressive (6mo) or sustainable (12mo)?
3. **Focus**: TUI-first or operator-first?
4. **Integration**: Standalone or integrate with hyper2kvm/v9s?

Tell me what you want to prioritize and I'll start building! 🚀
