# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Rule-level policy editing**: `POST|PUT|DELETE /api/v1/policies/{id}/rules` add,
  replace or delete one rule of a CiliumNetworkPolicy through the CRD, with
  `resource_version` concurrency (409), `?dry_run=true`, audit entries
  (`policy.rule.*`), and the cluster's validation message as a 400. `GET
  /policies/{id}` now returns `spec`, `resource_version` and `rule_counts`. New
  **Policy Rules** page (`/policy-rules`) with templates for entities, CIDR sets,
  ICMP, FQDN, DNS and L7 HTTP rules. A policy's last rule cannot be deleted
  (Cilium rejects rule-less policies).
- **Alert rule CRUD**: `POST /alerts/rules`, `PUT /alerts/rules/{id}/definition`,
  `DELETE /alerts/rules/{id}`; conditions are validated against what the engine can
  evaluate; built-in rules need `?force=true`.
- **Cilium Insights** (`/cilium-insights`) and read-only endpoints:
  `/cilium/features` (from `cilium-config`), `/hubble/nodes` (`GetNodes`, buffer
  fill), `/hubble/metrics` and `/cilium/metrics` (Prometheus), `/cilium/resources/{kind}`
  (BGP v2, LB-IPAM, L2, pod IP pools, CIDR groups, Gateway API, ...),
  `/cilium/agent/{what}` (allow-listed `cilium-dbg` queries).
- Flows carry an optional `hubble` object (identities, labels, direction,
  `policy_match_type`, observation point, node); identities are persisted in the
  flow store.
- Chart value `api.env.prometheusUrl` (`PROMETHEUS_URL`); chart and manifest RBAC
  now grant read access to the Cilium CRDs the views use (nodes, egress gateway,
  BGP, LB-IPAM, L2, pod IP pools, CIDR groups, Gateway API).
- Quiet Hubble follow streams flush every **2s** (in addition to the 64-flow batch)
  so low-volume clusters persist evidence promptly.
- `GET /api/v1/changes/{id}/impact` — before/after flow correlation for Change Log
  (filters, chart, evidence-backed; gaps → `inconclusive`; not causation).
- Investigation bundle export + **time-limited share** with redacted incident card:
  `…/export`, `…/share`, `GET …/investigate/share/{token}`.
- Declared connectivity paths (observe-only): sustained multi-sample alerts,
  silence (`POST …/connectivity/alerts/{id}/silence`), evidence deep-links.
- Flow store ops: `GET /api/v1/flows/store`, admin `POST …/purge`; gap timeline on
  Cluster Health.
- Overview per-tile **8s timeouts** and **stale** badges.
- CI `RUN_LIVE` gates for connectivity, impact, investigate export/share, flows/store.

### Changed

- `GET /flows?verdict=` is filtered by Hubble itself, so `limit` returns the most
  recent flows with that verdict rather than the few matches among the last N.
- Renamed the invented `cilium.io/canary-*` and `cilium.io/mtu` annotations to
  `paqtra.io/*`; Cilium never read them. Docs no longer claim chaos loads eBPF programs.
- `/health` and `/ready` stay cheap via background sampler; cache SQLite I/O uses
  `spawn_blocking`; bpftool concurrency capped.
- Overview auto-refreshes every 15s; `/ebpf/summary` defaults to counts-only
  (`?detail=full` for CT dumps); `/cluster/health` runs kubectl probes in parallel
  with a short cache.
- Chart API memory defaults: request **512Mi**, limit **2Gi** (durable flow index
  + bpftool inventory need headroom; 512Mi limits were OOM-killing under load).
- Connectivity list stays cheap; per-path status is
  `GET /api/v1/connectivity/paths/{id}/status`. Change-impact / status queries use
  `spawn_blocking` so SQLite does not stall the async runtime.

### Fixed

- `/ebpf/drops` and `/ebpf/summary` counted forwarded traffic as drops (only
  `cilium_metrics` reasons >= 130 are drops), read four key bytes instead of the
  reason byte, missed per-CPU values, and used invented reason names; now decoded
  correctly and named with Hubble's `DropReason`.
- Five `kubectl exec -l k8s-app=cilium` calls (invalid) now resolve an agent pod
  first: ClusterMesh, WireGuard/encryption and NAT/CT counts returned nothing.
- Policy rule/apply failures no longer surface as "Internal server error".

## [2.1.0] - 2026-09-24

### Added

- Continuous Hubble **follow** ingest with disconnect/gap counters, events/sec, and
  lag on `/health` → `subsystems.flow_ingest`.
- Real DNS L7 fields on flows (query, rcode, answers, latency); DNS UI no longer
  invents SERVFAIL from policy drops.
- `POST /api/v1/investigate/flow` — why-denied workflow from a selected flow
  (identities, CNP/CCNP candidates, drop reason, draft allow). Flows UI button
  on DROPPED rows.

### Changed

- Chart default `HUBBLE_MODE=grpc`; ingest labels `hubble_grpc` when Observer gRPC
  produced the data.
- Combined image defaults Hubble Relay Service port **80**.
- README and product docs updated for gRPC follow ingest, DNS evidence, and
  deny explanation.

[2.1.0]: https://github.com/zyvorai/paqtra/releases/tag/v2.1.0

## [2.0.0] - 2026-09-24

### Added

- **Investigate Path** — `POST /investigate/path` returns a confidence-tagged
  path report (`observed` | `inferred` | `unavailable`) with evidence from
  Hubble flows, EndpointSlices, and Cilium policies. UI: Investigate Path view.
- SQLite flow store with Hubble ingest, query APIs, and offline golden CI
  (`ci-investigate-golden.sh`).
- Evidence-backed policy **simulate** (no speculative allow without flow or
  policy evidence).
- Hubble **Observer gRPC** client (`HUBBLE_MODE=grpc|cli|auto`); CLI remains a
  fallback. Chart defaults and lab deploy use the in-cluster Hubble Service.
- Helm chart persistence (`PAQTRA_DATA_DIR`), expanded ClusterRole for
  EndpointSlices / CCNP / events, and in-cluster kubectl via ServiceAccount.
- Docs: `docs/investigate.md`; AGENTS.md read-only boundary (no BPF attach).

### Changed

- CLI, API, UI, install script, and Helm chart aligned on **2.0.0**.
- Chart `auth.adminPassword` defaults empty (auto-generate). Lab demo pin
  `Admin@321` is applied only by `scripts/deploy-remote.sh`.
- Repository home [`zyvorai/paqtra`](https://github.com/zyvorai/paqtra);
  images under `ghcr.io/zyvorai/paqtra*`.
- Documentation restructured to flat kebab-case product docs.
- Full Apache License 2.0, `NOTICE`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and
  GitHub community templates.

### Fixed

- In-cluster kubectl no longer targets `localhost:8080`; uses SA token/CA.
- Hubble health against Service port **80** (not container 4245).
- Flow JSON unwrap of Hubble `{"flow":...}` envelopes; IP source/destination
  parsing so Flows UI no longer shows UNKNOWN rows.
- Vite 8 Rolldown `manualChunks` and Tailwind 4 `@tailwindcss/postcss` for
  reliable UI builds.
- gRPC build via `tonic-prost-build`; TUI clippy clean-ups.

### Removed

- Gated `aya-ebpf` / map-writing / program-attach code that violated the
  read-only boundary (`AGENTS.md`). Map reads stay on `bpftool`.
- Session/status milestone docs under `docs/status/`.
- Obsolete Cilium-Vision branding and confidential client markings.

[2.0.0]: https://github.com/zyvorai/paqtra/releases/tag/v2.0.0
