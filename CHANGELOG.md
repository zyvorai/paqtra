# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Repository home moved to [`zyvorai/paqtra`](https://github.com/zyvorai/paqtra); container images under `ghcr.io/zyvorai/paqtra*`.
- Documentation restructured to flat kebab-case product docs (Netra-style layout).
- Default lab credentials (`Admin@321`) removed; `ADMIN_PASSWORD` is required when
  auth is enabled, and Helm auto-generates secrets when unset.
- Full Apache License 2.0 text, `NOTICE`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and
  GitHub community templates added for open-source release.

### Removed

- The gated `aya-ebpf` feature and everything behind it: the Aya map writer, event stream and
  reader, and the `ebpf_advanced` module (program hot-loading, XDP packet filter, probe-based
  profiler, CO-RE compile). They wrote BPF maps and attached programs, which Paqtra must not do
  (see `AGENTS.md`). Map reads go through `bpftool`. The unused `libbpf-rs` and `bytes`
  dependencies were dropped with them.
- Session/status milestone docs under `docs/status/`.
- Obsolete Cilium-Vision branding and confidential client markings.
