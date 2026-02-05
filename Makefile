.PHONY: build run check test clean install help

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

build: ## Build release binary
	cargo build --release

build-dev: ## Build debug binary
	cargo build

run: ## Run the application (debug mode)
	cargo run

run-release: ## Run the release build
	cargo run --release

run-skip: ## Run with bootstrap skipped
	cargo run -- --skip-bootstrap

check: ## Check code without building
	cargo check

test: ## Run tests
	cargo test

fmt: ## Format code
	cargo fmt

clippy: ## Run clippy linter
	cargo clippy -- -D warnings

clean: ## Clean build artifacts
	cargo clean

install: build ## Install to /usr/local/bin
	sudo cp target/release/cilium-tui /usr/local/bin/

uninstall: ## Uninstall from /usr/local/bin
	sudo rm -f /usr/local/bin/cilium-tui

dev: ## Development workflow: fmt + check + build
	cargo fmt
	cargo check
	cargo build

all: fmt clippy test build ## Run all checks and build
