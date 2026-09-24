# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
