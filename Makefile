.PHONY: build run check test clean install help

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

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
	cd web-ui && rm -rf dist

install: build ## Install TUI to /usr/local/bin
	sudo cp target/release/cilium-tui /usr/local/bin/

uninstall: ## Uninstall TUI from /usr/local/bin
	sudo rm -f /usr/local/bin/cilium-tui

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

# ─── Docker ────────────────────────────────────────────────────────────
docker-up: ## Start all services with docker compose
	cd deployments && docker compose up --build -d

docker-down: ## Stop all services
	cd deployments && docker compose down

docker-logs: ## Show service logs
	cd deployments && docker compose logs -f

# ─── Full Project ──────────────────────────────────────────────────────
check-all: check api-check ui-lint ## Check all components
test-all: test api-test ## Run all tests
build-all: build api-build ui-build ## Build everything
