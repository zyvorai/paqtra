.PHONY: build run check test clean install help lint-fix type-check security

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-25s\033[0m %s\n", $$1, $$2}'

# ─── Core TUI ──────────────────────────────────────────────────────────
build: ## Build TUI release binary
	cargo build --release

build-dev: ## Build TUI debug binary
	cargo build

run: ## Run the TUI (debug mode)
	cargo run

run-release: ## Run the TUI release build
	cargo run --release

run-skip: ## Run with bootstrap skipped
	cargo run -- --skip-bootstrap

check: ## Check TUI code without building
	cargo check

test: ## Run TUI tests
	cargo test

fmt: ## Format TUI code
	cargo fmt

clippy: ## Run clippy on TUI
	cargo clippy -- -D warnings

clean: ## Clean all build artifacts
	cargo clean
	cd web-api && cargo clean
	cd web-ui && rm -rf dist node_modules

install: build ## Install TUI to /usr/local/bin
	sudo cp target/release/paqtra /usr/local/bin/

uninstall: ## Uninstall TUI from /usr/local/bin
	sudo rm -f /usr/local/bin/paqtra

dev: ## TUI development workflow: fmt + check + build
	cargo fmt
	cargo check
	cargo build

all: fmt clippy test build ## Run all TUI checks and build

# ─── Aya eBPF ─────────────────────────────────────────────────────────
build-aya: ## Build with Aya eBPF support
	cargo build --release --features aya-ebpf

test-aya: ## Run tests including Aya feature
	cargo test --features aya-ebpf

# ─── Web API ───────────────────────────────────────────────────────────
api-build: ## Build web-api release binary
	cd web-api && cargo build --release

api-dev: ## Run web-api in dev mode
	cd web-api && cargo run

api-check: ## Check web-api code
	cd web-api && cargo check && cargo clippy -- -D warnings

api-test: ## Run web-api tests
	cd web-api && cargo test

# ─── Web UI ────────────────────────────────────────────────────────────
ui-install: ## Install web-ui dependencies
	cd web-ui && npm ci

ui-dev: ## Run web-ui dev server
	cd web-ui && npm run dev

ui-build: ## Build web-ui for production
	cd web-ui && npm run build

ui-lint: ## Lint web-ui code
	cd web-ui && npm run lint

ui-test: ## Run web-ui tests
	cd web-ui && npx vitest run

ui-typecheck: ## Type-check web-ui
	cd web-ui && npx tsc --noEmit

# ─── Docker ────────────────────────────────────────────────────────────
docker-up: ## Start all services with docker compose
	cd deployments && docker compose up --build -d

docker-down: ## Stop all services
	cd deployments && docker compose down

docker-logs: ## Show service logs
	cd deployments && docker compose logs -f

docker-api: ## Build API Docker image only
	docker build -t paqtra-api:latest -f web-api/Dockerfile web-api/

docker-ui: ## Build UI Docker image only
	docker build -t paqtra-ui:latest -f web-ui/Dockerfile web-ui/

docker-combined: ## Build combined API+UI image
	docker build -t paqtra:latest -f Dockerfile.combined .

docker-push: ## Push images to registry
	docker push ghcr.io/ssahani/paqtra-api:latest
	docker push ghcr.io/ssahani/paqtra-ui:latest
	docker push ghcr.io/ssahani/paqtra:latest

# ─── Installation ─────────────────────────────────────────────────────
install-full: ## Full system installation (build + install + systemd)
	bash install.sh install

install-services: ## Configure systemd services only
	bash install.sh setup-services

install-start: ## Start all services
	bash install.sh start

install-stop: ## Stop all services
	bash install.sh stop

install-status: ## Show service status
	bash install.sh status

install-uninstall: ## Full uninstall
	bash install.sh uninstall

# ─── Kubernetes Deployment ────────────────────────────────────────────
k8s-deploy: ## Deploy to Kubernetes cluster
	bash deployments/k8s/deploy.sh deploy

k8s-delete: ## Remove from Kubernetes
	bash deployments/k8s/deploy.sh delete

k8s-status: ## Show K8s deployment status
	bash deployments/k8s/deploy.sh status

k8s-logs: ## Stream K8s API logs
	bash deployments/k8s/deploy.sh logs api

k8s-port-forward: ## Port-forward API and UI
	bash deployments/k8s/deploy.sh port-forward

k8s-build: ## Build and push images for K8s
	bash deployments/k8s/deploy.sh build

# ─── Remote Deployment ────────────────────────────────────────────────
deploy-remote: ## Deploy to remote: make deploy-remote H=10.0.1.5 U=root P=pass
	bash scripts/deploy-remote.sh $(H) $(U) $(P)

deploy-remote-quick: ## Quick deploy (binaries only): make deploy-remote-quick H=ip U=root P=pass
	bash scripts/deploy-remote.sh $(H) $(U) $(P) --quick

deploy-remote-k3s: ## Deploy with K3s: make deploy-remote-k3s H=ip U=root P=pass
	bash scripts/deploy-remote.sh $(H) $(U) $(P) --k3s

deploy-remote-key: ## Deploy via SSH key: make deploy-remote-key H=ip U=root
	bash scripts/deploy-remote.sh $(H) $(U) --key

deploy-remote-uninstall: ## Uninstall remote: make deploy-remote-uninstall H=ip U=root P=pass
	bash scripts/deploy-remote.sh $(H) $(U) $(P) --uninstall

deploy-fleet: ## Deploy to fleet: make deploy-fleet FILE=hosts.txt
	bash scripts/deploy-remote.sh --fleet $(FILE)

# ─── Hyper SDK Cloud ──────────────────────────────────────────────────
hyper-deploy: ## Deploy to Hyper cloud
	bash scripts/deploy-hyper.sh deploy

hyper-build-push: ## Build and push for Hyper
	bash scripts/deploy-hyper.sh build-push

hyper-compose: ## Deploy via Hyper Compose
	bash scripts/deploy-hyper.sh compose

hyper-status: ## Show Hyper deployment status
	bash scripts/deploy-hyper.sh status

hyper-logs: ## Stream Hyper container logs
	bash scripts/deploy-hyper.sh logs

hyper-teardown: ## Remove Hyper deployment
	bash scripts/deploy-hyper.sh teardown

# ─── Quality & Security ───────────────────────────────────────────────
lint-fix: ## Auto-fix linting issues across TUI and web-ui
	cd web-ui && npx eslint --fix src/
	cargo fmt

type-check: ## Type-check web-ui TypeScript
	cd web-ui && npx tsc --noEmit

security: ## Run security audits on Rust and Node dependencies
	cargo audit 2>/dev/null || echo "Install cargo-audit: cargo install cargo-audit"
	cd web-ui && npm audit --audit-level=high

# ─── Full Project ──────────────────────────────────────────────────────
check-all: check api-check ui-typecheck ui-lint ## Check all components
test-all: test api-test ui-test ## Run all tests
build-all: build api-build ui-build ## Build everything
