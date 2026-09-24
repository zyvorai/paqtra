# Path investigation and policy preview

Paqtra’s investigation APIs explain connectivity failures and preview Cilium policy changes with explicit confidence.

## Confidence labels

Every step and impact claim carries one of:

| Label | Meaning |
|-------|---------|
| `observed` | Backed by indexed flows, Kubernetes objects, or change events with evidence IDs |
| `inferred` | Reasonable conclusion without direct primary evidence |
| `unavailable` | Source missing (Hubble down, empty store, unsupported construct) |

**Unsupported policy constructs are never a confident pass** — they surface as `unknown` outcomes with `unavailable` confidence.

## Flow store

- SQLite table under `PAQTRA_DATA_DIR/flows.db` (or in-memory if unset)
- Background **follow** ingest from Hubble Observer gRPC (`hubble_grpc` source; CLI only if `HUBBLE_MODE=cli|auto` falls back). Chart default is `grpc`.
- Default retention: 7 days
- Health: `GET /health` → `subsystems.flow_ingest` (`source`, `connected`, `disconnects`, `gaps`, `events_per_sec`, `lag_secs`, `hubble_mode`)

Enable persistence in Helm: `api.persistence.enabled=true` (sets `PAQTRA_DATA_DIR`).

## Explain API

```http
POST /api/v1/investigate/path
```

```json
{
  "source": { "namespace": "shop", "name": "checkout" },
  "destination": { "namespace": "shop", "name": "payments" },
  "port": 443,
  "protocol": "TCP",
  "time_window_minutes": 60
}
```

Response includes `steps[]`, `likely_owner` (`policy` | `dns` | `no_backend` | `datapath` | `unknown`), `next_actions`, and stores a redacted bundle.

```http
POST /api/v1/investigate/flow
```

Explain a single flow (from Flows “Why denied?”): identities, CNP/CCNP candidates, drop reason, and a draft minimal allow — all confidence-tagged.

```http
GET /api/v1/investigate/bundles/{id}
```

Bundles omit payloads, argv, and Secret contents ([AGENTS.md](../AGENTS.md)).

## Policy preview

`POST /api/v1/policies/simulate` runs an evidence-backed preview:

- Resolves `endpointSelector` when Kubernetes is reachable
- Matches indexed flows → `would_allow` / `would_deny` / `unknown`
- Lists `uncertainty` for FQDN, L7, deny-precedence gaps
- Suggests Cilium CRD rollback only (no BPF attach)

## UI

Investigate → **Path** (`/investigate`): form for A→B:port, step timeline, owner, evidence bundle.

Flows → **Why denied?** on a DROPPED row: opens the flow explain result (identities, CNP/CCNP candidates, drop reason, draft allow).

DNS → shows L7 query/rcode/answers when Hubble DNS visibility is present; L4-only rows are marked incomplete (never invents SERVFAIL from a policy drop).

## Boundaries

Enforcement stays in Cilium CNP/CCNP. Paqtra never writes Cilium maps or attaches programs. See [cilium-brotherhood.md](cilium-brotherhood.md).
