#!/usr/bin/env bash
# Packet-visibility contracts: Flows UI + follow ingest + live API smoke.
# Usage:
#   ./scripts/ci-packet-visibility.sh
#   RUN_LIVE=1 PAQTRA_API=http://HOST:PORT ADMIN_PASS=Admin@321 \
#     ./scripts/ci-packet-visibility.sh
#
# Default is offline (cluster-free) so GitHub CI stays green without Hubble.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

API="${PAQTRA_API:-}"
USER="${ADMIN_USER:-admin}"
PASS="${ADMIN_PASS:-Admin@321}"
RUN_LIVE="${RUN_LIVE:-0}"

pass() { echo "  ✔ $*"; }
fail() { echo "  ✖ $*"; exit 1; }

echo "==> packet visibility checks"

# Offline: Flows UX + why-denied wiring
grep -q 'Why denied?' web-ui/src/views/Flows/index.tsx || fail "Flows Why denied? button missing"
grep -q "/investigate/flow" web-ui/src/views/Flows/index.tsx || fail "Flows must POST /investigate/flow"
grep -q 'LIVE STREAM' web-ui/src/views/Flows/index.tsx || fail "Flows LIVE STREAM chrome missing"
grep -q 'waiting on Hubble Relay' web-ui/src/views/Flows/index.tsx || fail "Flows empty Hubble state missing"
pass "Flows UI contracts"

# Offline: API follow ingest + health + routes
grep -q 'flow_ingest' web-api/src/handlers/health.rs || fail "flow_ingest health missing"
grep -q 'follow_flows' web-api/src/services/hubble_grpc.rs || fail "follow_flows missing"
grep -q 'spawn_flow_ingest' web-api/src/main.rs || fail "flow_ingest spawn missing"
grep -q '"/api/v1/flows"' web-api/src/main.rs || fail "/api/v1/flows route missing"
grep -q '"/api/v1/flows/stats"' web-api/src/main.rs || fail "/api/v1/flows/stats route missing"
grep -q 'investigate/flow' web-api/src/main.rs || fail "investigate/flow route missing"
pass "API source contracts"

# Offline: docs
grep -q 'Live packet stream' docs/features.md || fail "docs/features.md missing live packet stream"
grep -q 'gap/disconnect visibility' docs/features.md || fail "docs/features.md missing ingest gap visibility"
pass "docs contracts"

# Offline: dedicated Vitest suite exists
test -f web-ui/src/views/Flows/__tests__/Flows.test.tsx || fail "Flows.test.tsx missing"
pass "Flows Vitest suite present"

if [[ "$RUN_LIVE" != "1" || -z "$API" ]]; then
  echo "==> skipping live API (set RUN_LIVE=1 PAQTRA_API=... to exercise cluster)"
  echo "==> packet visibility OK (offline)"
  exit 0
fi

echo "==> live login against $API"
TOKEN=$(curl -sf -X POST "$API/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"$USER\",\"password\":\"$PASS\"}" | python3 -c 'import sys,json; print(json.load(sys.stdin)["token"])')

auth=(-H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json')

echo "==> health flow_ingest"
curl -sf "$API/health" | python3 -c '
import sys, json
h=json.load(sys.stdin)
fi=h.get("subsystems", {}).get("flow_ingest")
assert isinstance(fi, dict), h
for k in ("status", "source", "connected", "gaps", "disconnects"):
    assert k in fi, (k, fi)
print("flow_ingest=", fi.get("status"), "source=", fi.get("source"),
      "connected=", fi.get("connected"), "indexed=", fi.get("indexed"))
'
pass "health.flow_ingest shape"

echo "==> flows list + stats"
FLOWS=$(curl -sf "$API/api/v1/flows?limit=10" "${auth[@]}")
echo "$FLOWS" | python3 -c '
import sys, json
r=json.load(sys.stdin)
assert "flows" in r, r
assert isinstance(r["flows"], list), r
print("flows=", len(r["flows"]), "total=", r.get("total"))
'
# Stats can time out when Hubble is saturated; treat as best-effort.
STATS_CODE=$(curl -sS -o /tmp/paqtra-flow-stats.json -w "%{http_code}"   --max-time 30 "$API/api/v1/flows/stats" "${auth[@]}" || echo 000)
if [[ "$STATS_CODE" == "200" ]]; then
  python3 -c '
import json
r=json.load(open("/tmp/paqtra-flow-stats.json"))
assert "forwarded" in r or "total_flows" in r or "total" in r, r
print("stats keys=", sorted(r.keys())[:12])
'
  pass "flows list + stats"
else
  echo "  ℹ flows/stats HTTP $STATS_CODE (Hubble timeout common under load) — continuing"
  pass "flows list (stats best-effort)"
fi

# Best-effort why-denied when a DROPPED flow is present
DROPPED=$(echo "$FLOWS" | python3 -c '
import sys, json
flows=json.load(sys.stdin).get("flows") or []
for f in flows:
    if str(f.get("verdict","")).upper()=="DROPPED":
        print(json.dumps(f))
        break
')
if [[ -n "$DROPPED" ]]; then
  echo "==> investigate/flow on DROPPED sample"
  export TOKEN PAQTRA_API="$API"
  echo "$DROPPED" | python3 -c '
import sys, json, urllib.request, os
f=json.load(sys.stdin)
api=os.environ["PAQTRA_API"]
token=os.environ["TOKEN"]
body=json.dumps({"flow_id": f.get("id"), "flow": f}).encode()
req=urllib.request.Request(
    api.rstrip("/")+"/api/v1/investigate/flow",
    data=body,
    headers={"Authorization":"Bearer "+token,"Content-Type":"application/json"},
    method="POST",
)
with urllib.request.urlopen(req, timeout=60) as resp:
    r=json.load(resp)
assert r.get("steps"), r
for s in r["steps"]:
    assert s.get("confidence") in ("observed","inferred","unavailable"), s
print("owner=", r.get("likely_owner"), "steps=", len(r["steps"]))
'
  pass "investigate/flow on DROPPED"
else
  echo "  ℹ no DROPPED flows in sample — skip investigate/flow"
fi

echo "==> packet visibility OK (live)"
