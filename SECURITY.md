# Security

Report suspected vulnerabilities privately to the maintainers via a private
GitHub security advisory (or email the maintainers listed in the repository).
Do not publish exploitable details in a public issue before coordination.

## Boundaries

Paqtra is observe/ops on top of Cilium. It must not write Cilium BPF maps,
replace Cilium programs, or become a second CNI. See
[`docs/cilium-brotherhood.md`](docs/cilium-brotherhood.md).

## Further reading

- [`docs/client/security-whitepaper.html`](docs/client/security-whitepaper.html) — threat model and auth defaults
- [`docs/web-architecture.md`](docs/web-architecture.md) — API auth, CORS, rate limiting
- [`README.md`](README.md#security) — high-level security features
