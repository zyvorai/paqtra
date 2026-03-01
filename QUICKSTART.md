# Cilium Vision - Quick Start Guide

Get up and running with Cilium Vision in minutes.

## Prerequisites

| Component | Requirement |
|-----------|-------------|
| Rust | 1.70+ (TUI and web-api) |
| Node.js | 20+ with npm (web-ui) |
| Docker | Optional, for containerised deployment |
| kubectl | Required for TUI; cluster must have Cilium + Hubble enabled |

## Quick Start (TUI)

```bash
git clone https://github.com/ssahani/cilium-flow.git
cd cilium-flow

cargo build --release
./target/release/cilium-tui
```

### Key Bindings

```
? = Help   q = Quit   Tab = Next tab
e = Explain packet          (Flows)
g = Generate  a = Apply    (AutoPolicy)
s = Simulate                (Simulator)
t = Time-travel             (Replay)
Enter = Run experiment      (Chaos)
p = Promote  r = Rollback  (Canary)
```

## Quick Start (Web Stack)

### Option A: Docker Compose (recommended)

```bash
# 1. Configure environment
cp deployments/.env.example deployments/.env
# Edit deployments/.env and set a strong JWT_SECRET (min 32 chars)

# 2. Start all services (API + UI + Redis)
make docker-up

# UI: http://localhost:3001   API: http://localhost:9191
```

To stop:

```bash
make docker-down
```

### Option B: Run manually

```bash
# Terminal 1 - build and run the API
cd web-api
cargo build --release
JWT_SECRET=change-me-to-a-real-secret-at-least-32-chars \
  cargo run --release

# Terminal 2 - build and run the UI
cd web-ui
npm ci
npm run build     # production build in dist/
npm run dev       # or start the dev server on :3000
```

## Development

```bash
make dev          # TUI: format, check, build
make api-dev      # Web API dev server
make ui-dev       # Web UI Vite dev server (port 3000, proxies /api to :9191)
make test-all     # Run all tests (TUI + API)
make check-all    # Lint/check all components
make build-all    # Production build of everything
```

## Kubernetes Deployment

Production-ready manifests live in `deployments/k8s/`. The directory contains Deployments, Services, Ingress, RBAC, ConfigMap, and Secrets resources for the full stack (API, UI, Redis).

```bash
kubectl apply -f deployments/k8s/
```

Review and update `deployments/k8s/secrets.yaml` with your own JWT secret before applying.

## Next Steps

- Read [FEATURES.md](FEATURES.md) for a detailed feature guide
- Explore [examples/](examples/) for real-world scenarios
- See [docs/architecture.md](docs/architecture.md) for system architecture
