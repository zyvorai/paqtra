# Testing Guide

This document describes how to test the Cilium TUI project.

## Test Categories

### 1. Unit Tests

Test individual components in isolation.

```bash
# Run all unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_intra_namespace_policy_generation

# Run tests for a specific module
cargo test --package cilium-tui --lib policies::tests
```

### 2. Integration Tests

Test the full application flow with a real Kubernetes cluster.

#### Prerequisites

- Kubernetes cluster (minikube/kind/k3s)
- Cilium installed
- kubectl configured

#### Quick Integration Test

```bash
# 1. Setup demo environment
./scripts/demo-setup.sh

# 2. Build the application
cargo build --release

# 3. Run the TUI (in one terminal)
./target/release/cilium-tui

# 4. Generate traffic (in another terminal)
kubectl exec -n demo $(kubectl get pod -l app=client -n demo -o jsonpath='{.items[0].metadata.name}') -- curl frontend
kubectl exec -n demo $(kubectl get pod -l app=client -n demo -o jsonpath='{.items[0].metadata.name}') -- curl backend

# 5. Verify flows appear in the TUI
```

### 3. End-to-End Tests

Complete workflow tests from cluster detection to flow visualization.

#### Manual E2E Test

1. **Bootstrap Test**
   ```bash
   # Clean state
   kubectl delete clusterrolebinding cilium-tui-binding 2>/dev/null || true
   kubectl delete serviceaccount cilium-tui -n kube-system 2>/dev/null || true

   # Run bootstrap
   cargo run --release

   # Verify:
   # - ConfigMap created
   # - Policies created
   # - ServiceAccount created
   # - TUI launches
   ```

2. **Skip Bootstrap Test**
   ```bash
   # Run without bootstrap
   cargo run --release -- --skip-bootstrap

   # Verify:
   # - TUI launches directly
   # - Can still see flows
   ```

3. **Flow Visibility Test**
   ```bash
   # Generate various traffic types
   kubectl run test-nginx --image=nginx
   kubectl run test-curl --image=curlimages/curl -- sleep 3600
   kubectl exec test-curl -- curl test-nginx

   # Verify in TUI:
   # - Flows appear
   # - Verdicts are correct
   # - Source/destination shown
   ```

## Test Scenarios

### Scenario 1: Fresh Cluster

Test bootstrapping on a cluster without Cilium TUI setup.

```bash
# Setup
minikube start --network-plugin=cni --cni=false
cilium install

# Test
cargo run --release

# Expected:
# ✓ Cluster detected
# ✓ Cilium detected
# ✓ Features enabled
# ✓ Policies created
# ✓ TUI launches
```

### Scenario 2: Already Configured

Test behavior when resources already exist.

```bash
# Run twice
cargo run --release
cargo run --release

# Expected:
# - No errors
# - Idempotent behavior
# - Resources not duplicated
```

### Scenario 3: Multiple Namespaces

Test policy creation across namespaces.

```bash
# Create namespaces
kubectl create namespace ns1
kubectl create namespace ns2
kubectl create namespace ns3

# Run
cargo run --release

# Verify:
kubectl get ciliumnetworkpolicy -n ns1
kubectl get ciliumnetworkpolicy -n ns2
kubectl get ciliumnetworkpolicy -n ns3

# Expected: Policies in all namespaces
```

### Scenario 4: Endpoint Discovery

Test endpoint tab functionality.

```bash
# Deploy various workloads
kubectl create deployment nginx --image=nginx --replicas=3
kubectl create deployment httpbin --image=kennethreitz/httpbin --replicas=2

# Run TUI
cargo run --release

# In TUI:
# - Press Tab to go to Endpoints
# - Verify all pods shown
# - Verify status colors correct
```

### Scenario 5: Policy Verification

Test that policies are correctly applied.

```bash
# Run TUI
cargo run --release

# In another terminal, verify policies
kubectl get ciliumnetworkpolicy --all-namespaces

# Expected policies:
# - allow-intra-namespace (each namespace)
# - allow-dns (each namespace)
# - allow-hubble (kube-system)
```

## Performance Tests

### Load Test

Test with many pods and flows.

```bash
# Create load
kubectl create deployment load --image=nginx --replicas=50

# Generate traffic
for i in {1..100}; do
  kubectl run test-$i --image=curlimages/curl -- sleep 3600
done

# Run TUI
cargo run --release

# Monitor:
# - Memory usage: < 100MB
# - CPU usage: < 20%
# - TUI responsive
```

### Stress Test

Test with continuous high-volume traffic.

```bash
# Setup
./scripts/demo-setup.sh

# Generate continuous traffic
while true; do
  kubectl exec -n demo $(kubectl get pod -l app=client -n demo -o jsonpath='{.items[0].metadata.name}') -- curl -s frontend > /dev/null
  sleep 0.1
done &

# Run TUI
cargo run --release

# Verify:
# - Flows stream continuously
# - No memory leaks
# - TUI remains responsive
```

## Regression Tests

After making changes, run these to ensure nothing broke:

```bash
# 1. Code quality
cargo fmt --check
cargo clippy -- -D warnings

# 2. Unit tests
cargo test

# 3. Build check
cargo build --release

# 4. Quick integration
./scripts/demo-setup.sh
./target/release/cilium-tui --skip-bootstrap
# Press 'q' to quit

# 5. Cleanup
kubectl delete namespace demo
```

## CI/CD Tests

The GitHub Actions workflow runs:

- Format check
- Clippy lints
- Unit tests
- Build on Linux and macOS

```bash
# Run locally what CI runs
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## Troubleshooting Tests

### Test: Cilium Not Installed

```bash
# Remove Cilium
cilium uninstall

# Run TUI
cargo run --release

# Expected:
# Error: Cilium not detected
```

### Test: No Cluster Access

```bash
# Break kubeconfig
mv ~/.kube/config ~/.kube/config.bak

# Run TUI
cargo run --release

# Expected:
# Error: Failed to infer Kubernetes config

# Restore
mv ~/.kube/config.bak ~/.kube/config
```

### Test: Port Forward Fails

```bash
# Block port
nc -l 4245 &
NC_PID=$!

# Run TUI
cargo run --release

# Expected:
# Falls back to CLI mode or shows error

# Cleanup
kill $NC_PID
```

## Test Checklist

Before releasing:

- [ ] All unit tests pass
- [ ] Integration test with minikube passes
- [ ] Bootstrap creates all resources
- [ ] Flows are visible in TUI
- [ ] Endpoints tab shows pods
- [ ] Policies tab shows policies
- [ ] All tabs navigable
- [ ] Quit works (press 'q')
- [ ] Skip bootstrap works
- [ ] Verbose logging works
- [ ] Custom port works
- [ ] Demo script works
- [ ] CI/CD passes
- [ ] No memory leaks
- [ ] Build on Linux works
- [ ] Build on macOS works

## Coverage

Generate code coverage report:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# Open report
open coverage/index.html
```

## Benchmarking

```bash
# Install criterion
cargo install cargo-criterion

# Run benchmarks (if implemented)
cargo criterion
```

## Testing Tips

1. **Always test in a safe cluster** - Use minikube/kind, not production
2. **Clean state between tests** - Delete resources before testing
3. **Check logs** - Use `--verbose` flag for debugging
4. **Test edge cases** - Empty cluster, no permissions, etc.
5. **Test failure modes** - Network issues, API timeouts, etc.

## Automated Testing

Run full test suite:

```bash
make test
```

Or manually:

```bash
cargo fmt --check && \
cargo clippy -- -D warnings && \
cargo test && \
cargo build --release
```
