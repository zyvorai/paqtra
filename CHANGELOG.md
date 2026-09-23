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

- Session/status milestone docs under `docs/status/`.
- Obsolete Cilium-Vision branding and confidential client markings.
