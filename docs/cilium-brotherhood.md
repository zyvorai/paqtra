# Cilium ↔ Paqtra brotherhood

Paqtra is the **Cilium-native** observe/ops sibling. Netra is the **independent eBPF** sibling. They must not fight.

## Roles

| Role | Owns | Must not |
|------|------|----------|
| **Cilium** | CNI, policy maps, identity, datapath (`cil_*`) | — |
| **Paqtra** | Install/status CLI, DaemonSet agent, Hubble/API/UI, **read-only** map + program inventory | Attach/replace Cilium programs; write Cilium maps; second CNI |
| **Netra** | Own programs under `/sys/fs/bpf/netra`, TCX/XDP, netlink | Modify Cilium maps |

## Hard boundaries

- Never write Cilium BPF maps or pin over `cil_*`
- Agent = observe / health / inventory; enforcement = Cilium CNP
- Prefer Hubble for flows; maps for node-local enrichment
- Attachment inventory **classifies** `cil_*` / `netra_*` / other — never attach, detach, or replace

## CLI

```bash
paqtra features              # discovery catalog (observe tiers)
paqtra ebpf attachments      # read-only program inventory
paqtra status                # includes BPF drift rows when available
```

## Agent HTTP (node-local)

```http
GET /health
GET /attachments
GET /drift
```

## Classification

Program names (full or truncated to 15 characters) map to owners:

- `cilium` — `cil_*`, `cilium*`
- `netra` — `netra_*`
- `other` — everything else

## Drift findings (warn-only)

| Kind | Severity | Meaning |
|------|----------|---------|
| `bpf-inventory-unavailable` | info | bpftool missing/failed |
| `bpf-cilium-missing` | warning | No `cil_*` programs visible |
| `bpf-inventory-truncated` | info | Cap hit (200 programs) |

None of these findings mutate the datapath.

## Network change assurance

Paqtra explains failures and previews Cilium policy changes; it does not become a second datapath:

- `POST /api/v1/investigate/path` — why can’t A reach B (evidence + confidence)
- `GET /api/v1/investigate/bundles/{id}/export` — redacted JSON or Markdown incident export
- `GET /api/v1/changes/{id}/impact` — before/after flow correlation (not causation)
- `GET|POST /api/v1/connectivity/paths` — declared path monitoring (observe-only; quiet = unknown)
- `POST /api/v1/policies/simulate` — flow-matched preview; unsupported constructs → unknown
- Apply / rollback only via Cilium CRDs (`CiliumNetworkPolicy`)

See [investigate.md](investigate.md).
