# Cilium Flow - Project Summary

## Overview

Cilium Flow is a Rust-based terminal UI platform that provides real-time network observability and intelligent operations for Kubernetes clusters running Cilium. It reads eBPF maps, streams Hubble flows, and exposes 13 interactive tabs covering everything from live packet inspection to chaos engineering and multi-cluster orchestration.

## By the Numbers

| Metric | Value |
|--------|-------|
| Language | Rust + TypeScript |
| Rust source files | 92 |
| Rust lines of code | 32,000+ |
| TUI tabs | 13 |
| Rust tests | 961 passing |
| Web API tests | 26 passing |
| Web UI views | 58 |
| Web UI components | 25 |
| Web UI hooks | 5 |
| Web UI tests | 68 passing |
| Total tests | 1,055 |
| Compiler warnings | 0 (Rust + TypeScript) |

## Module Status

### Fully Wired (Engine + View + Handlers + Async)

| Module | Engine | TUI View | Key Handlers | Async Ops | Status |
|--------|--------|----------|-------------|-----------|--------|
| Healer | SelfHealer | render_modules | d: detect | run/run_enriched | Complete |
| AutoPolicy | AutoPolicy | autopolicy_view | u/g/v/a/r/A/R | update/update_enriched | Complete |
| RootCause | RootCauseEngine | rootcause_view | a: apply fix | - | Complete |
| Simulator | Simulator | simulator_view | s/c: simulate/clear | - | Complete |
| Replay | ReplayEngine | replay_view | t/Space/arrows | - | Complete |
| Chaos | ChaosEngine | chaos_view | Enter/s/S/b | start/stop/stop_all | Complete |
| Canary | CanaryEngine | canary_view | p/r/+/d | promote/rollback/progress | Complete |
| MultiCluster | MultiClusterAutopilot | multicluster_view | v/h | health_check_all | Complete |

### Observability (Data-Driven, No Engine)

| Tab | Data Source | Status |
|-----|-----------|--------|
| Flows | Hubble CLI | Complete |
| Connections | IntegratedDataProvider | Complete |
| Endpoints | EndpointManager | Complete |
| Policies | K8s API | Complete |
| Metrics | eBPF maps | Complete |

### Supplementary Modules (Library Only)

| Module | Purpose | Status |
|--------|---------|--------|
| anomaly_detection | Multi-algorithm threat detection | Library |
| ebpf_advanced | Profiling, packet filters, CO-RE | Library |
| security_compliance | Zero-trust, compliance frameworks | Library |
| dev_tools | Traffic shadowing, replay, mirroring | Library |

## Architecture Layers

```
Terminal UI Layer        ratatui + crossterm, 13 tabs, 60 FPS
Event Handler Layer      Sync handlers (navigation, confirmation)
                         Async handlers (engine operations)
Engine Layer             8 engines with real K8s/kubectl integration
Data Layer               eBPF maps (Aya/bpftool), Hubble CLI, K8s API
Infrastructure           Cilium Agent, Kernel eBPF datapath
```

## Key Design Decisions

1. **Dual data path**: EnrichedMapReader (real eBPF) with MockMapReader fallback
2. **Sync/async split**: UI navigation is sync, engine operations are async
3. **Confirmation workflow**: Destructive actions require y/n confirmation
4. **Engine independence**: Chaos/Canary/MultiCluster engines are standalone (no MapReader dependency)
5. **kill_on_drop**: Hubble port-forward process stored in global OnceLock, cleaned up on exit
6. **Safety validation**: All 7 chaos experiment types validated against configurable limits
7. **tokio::process::Command**: All subprocess calls in async context use non-blocking I/O
8. **Input validation**: RFC 1123 regex on kubectl-bound names/namespaces, flag injection prevention
9. **JWT RBAC**: Claims injected into request extensions, `require_admin()` for destructive ops
10. **Type-safe handlers**: All POST endpoints use typed `Deserialize` structs, no raw `Json<Value>`

## Recent Changes

- Replaced unsafe libc::kill with safe Child::kill via Arc<Mutex>
- Converted all blocking std::process::Command to tokio::process::Command in async fns
- Wired ChaosEngine, CanaryEngine, MultiClusterAutopilot to TUI with real data
- Connected all key handlers to actual engine methods
- Added async handlers for chaos start/stop, canary promote/rollback, health checks
- Completed safety validation for all 7 chaos experiment types
- Eliminated all compiler warnings (0 warnings across all tests)
- Fixed zero-trust policy generator to skip unknown L4 protocols
- Removed resource leaks (orphaned cleanup tasks, unnecessary clones)
- Security hardening: JWT RBAC, input validation, typed request structs, graceful shutdown
- Fixed auth token storage mismatch (localStorage vs sessionStorage)
- Fixed DropReason u32→u8 truncation, pattern_counts double-counting
- Fixed port-forward process lifetime (global OnceLock)
- Replaced RefCell with Mutex in async context
- Added confirmation dialogs for destructive operations
- Added auto-dismiss for success messages via useAutoDismiss hook
- Removed dead code: unused i18n system, chartTheme, service worker, date-fns, react-table
- Tree-shaken d3 imports (d3-selection, d3-force, d3-drag instead of full d3)
- Consolidated duplicate HubbleService methods
- Added accessibility: aria-labels, role="dialog", aria-modal on all modals

## Web Dashboard (web-ui)

| Metric | Value |
|--------|-------|
| Framework | React 19 + TypeScript 5.6 |
| Views | 58 (lazy-loaded) |
| Components | 25 shared UI components |
| Hooks | 5 custom hooks |
| Stores | 3 Zustand stores |
| API types | 49 interfaces, 70+ functions |
| Tests | 68 passing |
| TypeScript | 0 errors |

Design: Dark-first theme (HyperSDK-aligned) with gradient icon headers, card-glow effects, navbar with hover dropdowns, global search command palette, toggle switches, sortable tables, score gauges, and full light theme support.

## Build & Test

```bash
# TUI (Rust)
cargo build --release   # 13 MB optimized binary
cargo test              # 961 tests, 0 failures
cargo clippy            # 0 warnings

# Web API (Axum)
cd web-api
cargo test              # 26 tests, 0 failures

# Web UI (React)
cd web-ui
npm run build           # Production build
npm run test            # 68 tests
```

## Dependencies

**Rust**: tokio, ratatui, crossterm, kube, k8s-openapi, anyhow, tracing, serde, uuid, chrono, serde_yaml, serde_json, libc. Optional: aya (eBPF), tonic/prost (gRPC).

**Web UI**: React 19, TypeScript 5.8, Tailwind CSS 3.4, Zustand 5, TanStack React Query, Axios, Recharts 2.15, D3 (d3-force, d3-selection, d3-drag), Lucide React, Monaco Editor, Vite 6, Vitest.
