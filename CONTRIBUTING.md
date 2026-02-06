# Contributing to Cilium Vision

Thank you for your interest in contributing to Cilium Vision! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Guidelines](#coding-guidelines)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Areas for Contribution](#areas-for-contribution)

## Code of Conduct

This project follows the Rust Code of Conduct. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Kubernetes cluster (local or remote)
- Cilium 1.14+ installed on the cluster
- Basic understanding of:
  - Rust programming
  - Kubernetes networking
  - eBPF concepts (helpful but not required)

### Fork and Clone

```bash
# Fork the repository on GitHub first, then:
git clone https://github.com/YOUR_USERNAME/cilium-flow.git
cd cilium-flow

# Add upstream remote
git remote add upstream https://github.com/ssahani/cilium-flow.git
```

## Development Setup

### Build from Source

```bash
# Development build (fast compilation, with debug symbols)
cargo build

# Run with logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Check for issues
cargo clippy

# Format code
cargo fmt
```

## Project Structure

```
cilium-flow/
├── src/
│   ├── main.rs              # Application entry point
│   ├── tui/                 # Terminal UI components
│   ├── modules/             # Intelligence modules
│   ├── hubble/              # Hubble gRPC client
│   ├── kubernetes/          # Kubernetes API client
│   └── ebpf/                # eBPF data structures
├── examples/                # Example configs and scenarios
├── docs/                    # Architecture documentation
└── README.md               # Main README
```

## Coding Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Write meaningful error messages
- Document public APIs

## Submitting Changes

### Commit Messages

Follow conventional commits format:

```
<type>(<scope>): <subject>

Co-Authored-By: Your Name <your.email@example.com>
```

**Types**: feat, fix, docs, style, refactor, test, chore

## Areas for Contribution

1. Prometheus Integration
2. Web UI
3. Additional Chaos Experiments
4. Policy Templates
5. Documentation

## Getting Help

- Questions: [GitHub Discussions](https://github.com/ssahani/cilium-flow/discussions)
- Bugs: [GitHub Issues](https://github.com/ssahani/cilium-flow/issues)

---

Built with ❤️ by the community
