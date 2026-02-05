# Contributing to Cilium TUI

Thank you for your interest in contributing to Cilium TUI!

## Development Setup

### Prerequisites

1. Rust toolchain (1.70+)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.up | sh
   ```

2. Kubernetes cluster for testing
   - minikube
   - kind
   - k3s
   - or any other K8s cluster

3. Cilium installed
   ```bash
   cilium install
   ```

### Building

```bash
git clone <repo>
cd cilium-tui
cargo build
```

### Running

```bash
cargo run
```

Or with make:

```bash
make run
```

## Project Structure

```
cilium-tui/
├── src/
│   ├── main.rs          # Entry point & CLI args
│   ├── bootstrap/       # Auto-detection & setup
│   ├── kubernetes/      # K8s API client wrapper
│   ├── cilium/          # Cilium-specific operations
│   ├── hubble/          # Hubble integration
│   ├── policies/        # Network policy generation
│   └── tui/             # Terminal UI
├── examples/            # Example policies
├── Cargo.toml
├── README.md
└── QUICKSTART.md
```

## Making Changes

### Code Style

We use `rustfmt` for formatting:

```bash
cargo fmt
```

### Linting

We use `clippy` for linting:

```bash
cargo clippy
```

### Testing

Run tests with:

```bash
cargo test
```

### Development Workflow

1. Create a branch for your feature
   ```bash
   git checkout -b feature/my-feature
   ```

2. Make your changes

3. Format and lint
   ```bash
   make dev
   ```

4. Test locally with a real cluster
   ```bash
   cargo run
   ```

5. Commit with meaningful messages
   ```bash
   git commit -m "Add feature: X"
   ```

6. Push and create a PR
   ```bash
   git push origin feature/my-feature
   ```

## Areas to Contribute

### High Priority

- [ ] Direct gRPC connection to Hubble (replace CLI calls)
- [ ] Endpoint health monitoring
- [ ] Flow filtering and search
- [ ] Metrics visualization

### Medium Priority

- [ ] Policy recommendation engine
- [ ] Export flows to JSON/CSV
- [ ] Custom policy templates
- [ ] Better error handling

### Low Priority

- [ ] Multi-cluster support
- [ ] Theme customization
- [ ] Configuration file support

## Adding a New Feature

### Example: Adding a New TUI Tab

1. Define the tab in `src/tui/mod.rs`:

   ```rust
   fn render_my_tab(&self, f: &mut Frame, area: Rect) {
       // Your rendering logic
   }
   ```

2. Add it to the tabs array:

   ```rust
   let titles = vec!["Flows", "Endpoints", "Policies", "Metrics", "MyTab"];
   ```

3. Handle it in the match statement:

   ```rust
   match self.selected_tab {
       // ...
       4 => self.render_my_tab(f, chunks[2]),
       _ => {}
   }
   ```

### Example: Adding a New Policy

1. Add method to `src/policies/mod.rs`:

   ```rust
   pub async fn apply_my_policy(&self, namespace: &str) -> Result<()> {
       let policy = format!(r#"
   apiVersion: cilium.io/v2
   kind: CiliumNetworkPolicy
   metadata:
     name: my-policy
     namespace: {namespace}
   spec:
     # ... your spec
   "#);

       self.k8s_client.apply_custom_resource(Some(namespace), &policy).await
   }
   ```

2. Call it from bootstrap:

   ```rust
   policy_mgr.apply_my_policy(ns).await?;
   ```

## Testing with a Real Cluster

### Setup Test Environment

```bash
# Create a test cluster
minikube start
cilium install

# Deploy test workloads
kubectl create deployment nginx --image=nginx
kubectl create deployment curl --image=curlimages/curl -- sleep 3600
```

### Test the TUI

```bash
cargo run
```

### Generate test traffic

```bash
kubectl exec -it deployment/curl -- curl nginx
```

Watch the flows appear in the TUI!

## Documentation

When adding features:

1. Update README.md
2. Add examples to examples/
3. Update QUICKSTART.md if user-facing

## Commit Messages

Use conventional commits:

```
feat: Add flow filtering
fix: Correct port-forward detection
docs: Update README with new feature
refactor: Simplify bootstrap logic
test: Add tests for policy generation
```

## Questions?

Open an issue or discussion on GitHub!
