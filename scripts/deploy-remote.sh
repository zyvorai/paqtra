#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Cilium Vision — Remote Deployment via SSH + rsync
# Supports password auth (sshpass), --quick mode, K3s deploy
#
# Usage:
#   ./scripts/deploy-remote.sh <host> <user> [password] [options]
#   ./scripts/deploy-remote.sh 10.0.1.5 root mypass --quick
#   ./scripts/deploy-remote.sh 10.0.1.5 root mypass --k3s
#   ./scripts/deploy-remote.sh 10.0.1.5 root --key  (SSH key auth)
#   ./scripts/deploy-remote.sh --fleet hosts.txt
#   ./scripts/deploy-remote.sh 10.0.1.5 root mypass --uninstall
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION="2.0.0"
REMOTE_DIR=""  # Resolved after TARGET_USER is known

# Parse args
TARGET_HOST="${1:-}"
TARGET_USER="${2:-root}"
TARGET_PASS="${3:-}"
QUICK_MODE=false
K3S_MODE=false
UNINSTALL=false
FLEET_FILE=""
KEY_AUTH=false

for arg in "$@"; do
    case "$arg" in
        --quick)     QUICK_MODE=true ;;
        --k3s)       K3S_MODE=true ;;
        --uninstall) UNINSTALL=true ;;
        --key)       KEY_AUTH=true; TARGET_PASS="" ;;
        --fleet)     FLEET_FILE="${4:-}" ;;
    esac
done

ok()   { echo "  ✅ $*"; }
fail() { echo "  ❌ $*"; }
info() { echo "  ℹ️  $*"; }
warn() { echo "  ⚠️  $*"; }

# ─── SSH / rsync wrappers with sshpass support ───────────────

SSH_OPTS="-o StrictHostKeyChecking=accept-new -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR -o ConnectTimeout=10"

_ssh() {
    if [ -n "${TARGET_PASS}" ] && command -v sshpass &>/dev/null; then
        export SSHPASS="${TARGET_PASS}"
        sshpass -e ssh ${SSH_OPTS} "${TARGET_USER}@${TARGET_HOST}" "$@"
    else
        ssh ${SSH_OPTS} "${TARGET_USER}@${TARGET_HOST}" "$@"
    fi
}

_rsync() {
    local rsync_opts="-az --delete --progress"
    if [ -n "${TARGET_PASS}" ] && command -v sshpass &>/dev/null; then
        export SSHPASS="${TARGET_PASS}"
        rsync ${rsync_opts} -e "sshpass -e ssh ${SSH_OPTS}" "$@"
    else
        rsync ${rsync_opts} -e "ssh ${SSH_OPTS}" "$@"
    fi
}

# ─── Validate ────────────────────────────────────────────────

validate() {
    if [ -z "${TARGET_HOST}" ]; then
        echo "Usage: $0 <host> <user> [password] [--quick|--k3s|--uninstall|--key]"
        echo ""
        echo "Examples:"
        echo "  $0 10.0.1.5 root mypassword              # Full deploy with password"
        echo "  $0 10.0.1.5 root mypassword --quick       # Rsync + install only"
        echo "  $0 10.0.1.5 root mypassword --k3s         # Deploy with K3s + Helm"
        echo "  $0 10.0.1.5 root --key                    # SSH key auth"
        echo "  $0 10.0.1.5 root mypassword --uninstall   # Remove everything"
        echo "  $0 --fleet hosts.txt                      # Deploy to multiple hosts"
        exit 1
    fi

    if [ -n "${TARGET_PASS}" ] && ! command -v sshpass &>/dev/null; then
        warn "sshpass not found — install it for password auth, or use --key"
        warn "  Fedora/RHEL: sudo dnf install sshpass"
        warn "  Ubuntu:      sudo apt install sshpass"
        exit 1
    fi
}

# ─── Connectivity check ─────────────────────────────────────

check_connectivity() {
    info "Testing SSH connectivity to ${TARGET_USER}@${TARGET_HOST}..."
    if _ssh "echo ok" &>/dev/null; then
        ok "SSH connected to ${TARGET_HOST}"
    else
        fail "Cannot SSH to ${TARGET_HOST}"
        exit 1
    fi

    # Resolve remote home directory
    REMOTE_HOME=$(_ssh "echo \$HOME" 2>/dev/null | tr -d '\r')
    REMOTE_HOME="${REMOTE_HOME:-/home/${TARGET_USER}}"
    REMOTE_DIR="${REMOTE_HOME}/cilium-vision"
    info "Remote deploy directory: ${REMOTE_DIR}"
}

# ─── Sync project files ─────────────────────────────────────

sync_files() {
    info "Syncing project to ${TARGET_HOST}:${REMOTE_DIR}..."

    _ssh "mkdir -p '${REMOTE_DIR}'"
    _rsync \
        --exclude '.git' \
        --exclude 'node_modules' \
        --exclude 'target' \
        --exclude 'dist' \
        --exclude '*.vmdk' \
        --exclude '*.qcow2' \
        --exclude '*.iso' \
        "${PROJECT_DIR}/" \
        "${TARGET_USER}@${TARGET_HOST}:${REMOTE_DIR}/"

    ok "Synced to ${TARGET_HOST}:${REMOTE_DIR}"
}

# ─── Sync pre-built binaries (quick mode) ────────────────────

sync_binaries() {
    info "Syncing pre-built binaries..."

    # API binary
    if [ -f "${PROJECT_DIR}/web-api/target/release/cilium-vision-api" ]; then
        _rsync \
            "${PROJECT_DIR}/web-api/target/release/cilium-vision-api" \
            "${TARGET_USER}@${TARGET_HOST}:/tmp/cilium-vision-api"
        ok "API binary synced"
    else
        warn "API binary not found — run 'make api-build' first"
    fi

    # UI dist
    if [ -d "${PROJECT_DIR}/web-ui/dist" ]; then
        _rsync \
            "${PROJECT_DIR}/web-ui/dist/" \
            "${TARGET_USER}@${TARGET_HOST}:/tmp/cilium-vision-ui/"
        ok "UI files synced"
    else
        warn "UI not built — run 'make ui-build' first"
    fi

    # Install script
    _rsync "${PROJECT_DIR}/install.sh" "${TARGET_USER}@${TARGET_HOST}:/tmp/cilium-vision-install.sh"
}

# ─── Uninstall old version ───────────────────────────────────

uninstall_old() {
    info "Removing old version..."
    _ssh bash <<'REMOTE'
systemctl stop cilium-vision-api 2>/dev/null || true
systemctl stop cilium-vision-ui 2>/dev/null || true
systemctl disable cilium-vision-api cilium-vision-ui 2>/dev/null || true
rm -f /usr/local/bin/cilium-vision-api
rm -f /usr/lib/systemd/system/cilium-vision-api.service
rm -f /usr/lib/systemd/system/cilium-vision-ui.service
systemctl daemon-reload 2>/dev/null || true
REMOTE
    ok "Old version removed"
}

# ─── Install on remote (quick mode) ─────────────────────────

install_quick() {
    info "Installing binaries on ${TARGET_HOST}..."
    _ssh bash <<'REMOTE'
set -e

# Install API binary
if [ -f /tmp/cilium-vision-api ]; then
    install -m 755 /tmp/cilium-vision-api /usr/local/bin/cilium-vision-api
    echo "  API binary installed"
fi

# Install UI files
if [ -d /tmp/cilium-vision-ui ]; then
    mkdir -p /var/lib/cilium-vision/ui
    cp -r /tmp/cilium-vision-ui/* /var/lib/cilium-vision/ui/
    echo "  UI files installed"
fi

# Create user if needed
id cilium-vision &>/dev/null || useradd -r -s /sbin/nologin -m -d /var/lib/cilium-vision cilium-vision 2>/dev/null || true

# Create config
mkdir -p /etc/cilium-vision /var/log/cilium-vision
if [ ! -f /etc/cilium-vision/config.env ]; then
    JWT=$(openssl rand -hex 32 2>/dev/null || head -c 64 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 64)
    cat > /etc/cilium-vision/config.env <<EOF
CILIUM_VISION_HOST=0.0.0.0
CILIUM_VISION_PORT=9191
HUBBLE_ADDRESS=localhost:4245
REDIS_URL=redis://localhost:6379
JWT_SECRET=${JWT}
RUST_LOG=info
UI_DIST_DIR=/var/lib/cilium-vision/ui
EOF
    chmod 600 /etc/cilium-vision/config.env
fi

# Systemd service
cat > /usr/lib/systemd/system/cilium-vision-api.service <<'EOF'
[Unit]
Description=Cilium Vision API Server
After=network-online.target redis.service
Wants=network-online.target

[Service]
Type=simple
User=cilium-vision
EnvironmentFile=/etc/cilium-vision/config.env
ExecStart=/usr/local/bin/cilium-vision-api
Restart=on-failure
RestartSec=5
LimitNOFILE=65536
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/log/cilium-vision /var/lib/cilium-vision
PrivateTmp=yes
NoNewPrivileges=yes

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload

# Install and start Redis if available
if command -v redis-server &>/dev/null; then
    systemctl enable redis --now 2>/dev/null || true
fi

# Start API
systemctl enable cilium-vision-api --now 2>/dev/null || true
sleep 2

# Health check
if curl -sf http://localhost:9191/health >/dev/null 2>&1; then
    echo "  ✅ API health check passed"
else
    echo "  ⚠️  API not responding (may need Redis)"
fi
REMOTE
    ok "Installation complete"
}

# ─── Setup K8s + Cilium + Hubble stack ─────────────────────

setup_k8s_stack() {
    info "Setting up K3s + Cilium + Hubble on ${TARGET_HOST}..."
    _ssh bash <<REMOTE
set -e

# ── K3s ──────────────────────────────────────────────────────
if ! command -v k3s &>/dev/null; then
    echo "  Installing K3s (no flannel, no kube-proxy, no traefik)..."
    curl -sfL https://get.k3s.io | INSTALL_K3S_EXEC="--disable=traefik --flannel-backend=none --disable-network-policy --disable-kube-proxy" sh -
    echo "  Waiting for K3s API..."
    for i in \$(seq 1 60); do
        KUBECONFIG=/etc/rancher/k3s/k3s.yaml kubectl get nodes &>/dev/null && break
        sleep 2
    done
    echo "  ✅ K3s installed"
else
    echo "  ✅ K3s already installed"
fi

export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

# ── Helm ─────────────────────────────────────────────────────
if ! command -v helm &>/dev/null; then
    echo "  Installing Helm..."
    curl -fsSL https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
    echo "  ✅ Helm installed"
fi

# ── Cilium CLI ───────────────────────────────────────────────
if ! command -v cilium &>/dev/null; then
    echo "  Installing Cilium CLI..."
    CILIUM_CLI_VERSION=\$(curl -s https://raw.githubusercontent.com/cilium/cilium-cli/main/stable.txt)
    CLI_ARCH=amd64
    if [ "\$(uname -m)" = "aarch64" ]; then CLI_ARCH=arm64; fi
    curl -L --fail --remote-name-all "https://github.com/cilium/cilium-cli/releases/download/\${CILIUM_CLI_VERSION}/cilium-linux-\${CLI_ARCH}.tar.gz{,.sha256sum}"
    sha256sum --check cilium-linux-\${CLI_ARCH}.tar.gz.sha256sum 2>/dev/null || true
    sudo tar xzvf cilium-linux-\${CLI_ARCH}.tar.gz -C /usr/local/bin
    rm -f cilium-linux-\${CLI_ARCH}.tar.gz{,.sha256sum}
    echo "  ✅ Cilium CLI installed"
fi

# ── Cilium CNI ───────────────────────────────────────────────
if ! kubectl get daemonset -n kube-system cilium &>/dev/null; then
    echo "  Installing Cilium CNI with Hubble..."
    helm repo add cilium https://helm.cilium.io/ 2>/dev/null || true
    helm repo update cilium 2>/dev/null || true
    helm install cilium cilium/cilium --namespace kube-system \
        --set operator.replicas=1 \
        --set kubeProxyReplacement=true \
        --set hubble.relay.enabled=true \
        --set hubble.ui.enabled=false \
        --set hubble.metrics.enabled="{dns,drop,tcp,flow,icmp,http}" \
        --set hubble.metrics.enableOpenMetrics=true \
        --set prometheus.enabled=true \
        --set operator.prometheus.enabled=true \
        --wait --timeout 300s
    echo "  ✅ Cilium + Hubble installed"
else
    echo "  ✅ Cilium already installed"

    # Ensure Hubble relay is enabled (might be missing from older installs)
    HUBBLE_RELAY=\$(kubectl get deploy -n kube-system hubble-relay 2>/dev/null | grep -c hubble-relay || true)
    if [ "\$HUBBLE_RELAY" = "0" ]; then
        echo "  Enabling Hubble relay..."
        helm upgrade cilium cilium/cilium --namespace kube-system --reuse-values \
            --set hubble.relay.enabled=true \
            --set hubble.metrics.enabled="{dns,drop,tcp,flow,icmp,http}" \
            --wait --timeout 120s 2>/dev/null || true
        echo "  ✅ Hubble relay enabled"
    else
        echo "  ✅ Hubble relay already running"
    fi
fi

# ── Wait for Cilium + Hubble to be ready ─────────────────────
echo "  Waiting for Cilium pods to be ready..."
kubectl -n kube-system wait --for=condition=ready pod -l k8s-app=cilium --timeout=120s 2>/dev/null || true
kubectl -n kube-system wait --for=condition=ready pod -l k8s-app=hubble-relay --timeout=120s 2>/dev/null || true

# ── Port-forward Hubble relay for local access ───────────────
# Kill any existing port-forward
pkill -f "kubectl.*port-forward.*hubble-relay" 2>/dev/null || true
sleep 1

# Start port-forward in background so the API can reach Hubble at localhost:4245
nohup kubectl -n kube-system port-forward deploy/hubble-relay 4245:4245 --address=127.0.0.1 \
    > /tmp/hubble-relay-port-forward.log 2>&1 &
echo "  ✅ Hubble relay port-forwarded to localhost:4245"

# ── Verify ───────────────────────────────────────────────────
echo ""
echo "  === Cluster Status ==="
kubectl get nodes
echo ""
echo "  === Cilium Status ==="
cilium status --brief 2>/dev/null || kubectl -n kube-system get pods -l k8s-app=cilium
echo ""
echo "  === Hubble Relay ==="
kubectl -n kube-system get pods -l k8s-app=hubble-relay
echo ""
REMOTE
    ok "K8s + Cilium + Hubble stack ready"
}

# ─── Full install (build on remote) ─────────────────────────

install_full() {
    info "Running full installation on ${TARGET_HOST}..."

    # First ensure K8s + Cilium + Hubble are set up
    setup_k8s_stack

    info "Building and installing Cilium Vision..."
    _ssh bash <<REMOTE
set -e
cd ${REMOTE_DIR}
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

# Install system deps
if command -v dnf &>/dev/null; then
    dnf install -y gcc make openssl-devel pkg-config curl wget git redis nodejs npm 2>/dev/null || true
elif command -v apt-get &>/dev/null; then
    apt-get update -qq && apt-get install -y -qq build-essential pkg-config libssl-dev curl wget git redis-server nodejs npm 2>/dev/null || true
fi

# Install Rust if needed
if ! command -v rustc &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "\$HOME/.cargo/env"
fi

# Build API
echo "Building API..."
cd web-api && cargo build --release && cd ..

# Build UI
echo "Building UI..."
cd web-ui && npm ci --silent && npm run build && cd ..

# Install the built binary
sudo systemctl stop cilium-vision-api 2>/dev/null || true
sudo install -m 755 web-api/target/release/cilium-vision-api /usr/local/bin/cilium-vision-api
echo "  API binary installed to /usr/local/bin/"

# Install UI files
if [ -d web-ui/dist ]; then
    sudo mkdir -p /var/lib/cilium-vision/ui
    sudo cp -r web-ui/dist/* /var/lib/cilium-vision/ui/
    echo "  UI files installed"
fi

# Update config to point at Hubble relay
sudo mkdir -p /etc/cilium-vision
if [ -f /etc/cilium-vision/config.env ]; then
    # Ensure HUBBLE_ADDRESS is set correctly
    if ! grep -q "HUBBLE_ADDRESS" /etc/cilium-vision/config.env; then
        echo "HUBBLE_ADDRESS=localhost:4245" | sudo tee -a /etc/cilium-vision/config.env > /dev/null
    fi
    # Ensure K8S_CONTEXT is not set (use default kubeconfig)
    if ! grep -q "KUBECONFIG" /etc/cilium-vision/config.env; then
        echo "KUBECONFIG=/etc/rancher/k3s/k3s.yaml" | sudo tee -a /etc/cilium-vision/config.env > /dev/null
    fi
fi

# Run installer for services/config
bash install.sh setup-services
bash install.sh start

# Create a systemd service for Hubble port-forward (survives reboots)
sudo tee /usr/lib/systemd/system/hubble-port-forward.service > /dev/null <<'SVCEOF'
[Unit]
Description=Hubble Relay Port Forward
After=k3s.service
Wants=k3s.service

[Service]
Type=simple
Environment=KUBECONFIG=/etc/rancher/k3s/k3s.yaml
ExecStartPre=/bin/sh -c 'until kubectl -n kube-system get deploy hubble-relay; do sleep 5; done'
ExecStart=/usr/local/bin/kubectl -n kube-system port-forward deploy/hubble-relay 4245:4245 --address=127.0.0.1
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
SVCEOF
sudo systemctl daemon-reload
sudo systemctl enable hubble-port-forward --now 2>/dev/null || true
sleep 3

# Final health check
echo ""
echo "  === Final Health Check ==="
curl -sf http://localhost:9191/health 2>/dev/null | python3 -m json.tool 2>/dev/null || curl -sf http://localhost:9191/health || echo "  API not responding yet"
REMOTE
    ok "Full installation complete"
}

# ─── K3s deployment ──────────────────────────────────────────

deploy_k3s() {
    info "Deploying Cilium Vision on K3s..."

    # Set up K8s + Cilium + Hubble stack first
    setup_k8s_stack

    _ssh bash <<REMOTE
set -e
export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

# Deploy Cilium Vision via Helm if chart exists
if [ -d ${REMOTE_DIR}/chart ]; then
    echo "Installing Cilium Vision via Helm chart..."
    helm upgrade --install cilium-vision ${REMOTE_DIR}/chart \
        --namespace cilium-system --create-namespace \
        --set redis.enabled=true \
        --set api.replicas=1 \
        --set ui.replicas=1 \
        --wait --timeout 120s 2>/dev/null || {
            echo "  Helm install failed, deploying via kubectl..."
            kubectl apply -f ${REMOTE_DIR}/deployments/k8s/ 2>/dev/null || true
        }
else
    # Fallback to raw manifests
    kubectl apply -f ${REMOTE_DIR}/deployments/k8s/ 2>/dev/null || true
fi

echo ""
echo "  K3s cluster status:"
kubectl get nodes
echo ""
echo "  Cilium pods:"
kubectl -n kube-system get pods -l k8s-app=cilium
echo ""
echo "  Hubble Relay:"
kubectl -n kube-system get pods -l k8s-app=hubble-relay
echo ""
echo "  Cilium Vision pods:"
kubectl -n cilium-system get pods 2>/dev/null || echo "  (deployed via systemd, not K8s)"
echo ""
echo "  ✅ K3s deployment complete"
REMOTE
    ok "K3s deployment complete on ${TARGET_HOST}"
}

# ─── Uninstall ───────────────────────────────────────────────

do_uninstall() {
    info "Uninstalling Cilium Vision from ${TARGET_HOST}..."
    _ssh bash <<REMOTE
set -e

# Stop services
systemctl stop cilium-vision-api cilium-vision-ui hubble-port-forward 2>/dev/null || true
systemctl disable cilium-vision-api cilium-vision-ui hubble-port-forward 2>/dev/null || true
rm -f /usr/lib/systemd/system/hubble-port-forward.service

# Remove K8s deployment
if command -v kubectl &>/dev/null; then
    kubectl delete namespace cilium-system 2>/dev/null || true
fi
if command -v helm &>/dev/null; then
    helm uninstall cilium-vision -n cilium-system 2>/dev/null || true
fi

# Remove files
rm -f /usr/local/bin/cilium-vision-api /usr/local/bin/cilium-tui
rm -f /usr/lib/systemd/system/cilium-vision-api.service
rm -f /usr/lib/systemd/system/cilium-vision-ui.service
rm -f /etc/pam.d/cilium-vision
rm -rf /var/lib/cilium-vision /etc/cilium-vision /var/log/cilium-vision
rm -rf ${REMOTE_DIR}
userdel cilium-vision 2>/dev/null || true
systemctl daemon-reload

echo "  ✅ Cilium Vision uninstalled"
REMOTE
    ok "Uninstall complete on ${TARGET_HOST}"
}

# ─── Fleet deploy ────────────────────────────────────────────

deploy_fleet() {
    local hosts_file="$1"
    [ -f "$hosts_file" ] || { fail "File not found: $hosts_file"; exit 1; }

    local count=0
    while IFS=' ' read -r host user pass opts; do
        [ -z "$host" ] && continue
        [[ "$host" =~ ^# ]] && continue

        echo ""
        echo "🚀 ═══ Deploying to ${host} ═══"
        TARGET_HOST="$host"
        TARGET_USER="${user:-root}"
        TARGET_PASS="${pass:-}"

        if [[ "$opts" == *"--quick"* ]] || [ "$QUICK_MODE" = true ]; then
            check_connectivity && uninstall_old && sync_binaries && install_quick
        elif [[ "$opts" == *"--k3s"* ]] || [ "$K3S_MODE" = true ]; then
            check_connectivity && sync_files && deploy_k3s
        else
            check_connectivity && sync_files && uninstall_old && install_full
        fi

        count=$((count + 1))
    done < "$hosts_file"

    echo ""
    ok "Deployed to ${count} host(s)"
}

# ─── Verify ──────────────────────────────────────────────────

verify() {
    info "Verifying deployment on ${TARGET_HOST}..."
    _ssh bash <<'REMOTE'
echo ""
echo "=== Service Status ==="
systemctl is-active cilium-vision-api 2>/dev/null && echo "  API: ✅ active" || echo "  API: ❌ inactive"
systemctl is-active hubble-port-forward 2>/dev/null && echo "  Hubble Port-Forward: ✅ active" || echo "  Hubble Port-Forward: ⚠️  inactive"

echo ""
echo "=== Health Check ==="
HEALTH=$(curl -sf http://localhost:9191/health 2>/dev/null)
if [ -n "$HEALTH" ]; then
    echo "  $HEALTH" | python3 -m json.tool 2>/dev/null || echo "  $HEALTH"
else
    echo "  ❌ API not responding"
fi

echo ""
echo "=== Versions ==="
cilium-vision-api --version 2>/dev/null || echo "  API version: unknown"

if command -v k3s &>/dev/null; then
    echo ""
    echo "=== K3s ==="
    k3s --version
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml

    echo ""
    echo "=== Cilium ==="
    cilium status --brief 2>/dev/null || kubectl -n kube-system get pods -l k8s-app=cilium 2>/dev/null || true

    echo ""
    echo "=== Hubble ==="
    kubectl -n kube-system get pods -l k8s-app=hubble-relay 2>/dev/null || true

    echo ""
    echo "=== Hubble Connectivity ==="
    if curl -sf --connect-timeout 2 localhost:4245 2>/dev/null; then
        echo "  ✅ Hubble relay reachable at localhost:4245"
    elif nc -z localhost 4245 2>/dev/null; then
        echo "  ✅ Hubble relay port open at localhost:4245"
    else
        echo "  ⚠️  Hubble relay not reachable at localhost:4245"
    fi
fi
REMOTE
}

# ─── Main ────────────────────────────────────────────────────

main() {
    echo ""
    echo "🔷 ══════════════════════════════════════════════════"
    echo "🔷    Cilium Vision v${VERSION} — Remote Deploy"
    echo "🔷 ══════════════════════════════════════════════════"
    echo ""

    # Fleet mode
    if [ "${FLEET_FILE}" != "" ]; then
        deploy_fleet "${FLEET_FILE}"
        exit 0
    fi

    validate
    check_connectivity

    if [ "$UNINSTALL" = true ]; then
        do_uninstall
        exit 0
    fi

    if [ "$QUICK_MODE" = true ]; then
        uninstall_old
        sync_binaries
        install_quick
    elif [ "$K3S_MODE" = true ]; then
        sync_files
        deploy_k3s
    else
        sync_files
        uninstall_old
        install_full
    fi

    verify

    echo ""
    echo "✅ ══════════════════════════════════════════════════"
    echo "✅    Deployment complete: ${TARGET_HOST}"
    echo "✅ ══════════════════════════════════════════════════"
    echo ""
    echo "  API:  http://${TARGET_HOST}:9191"
    echo "  UI:   http://${TARGET_HOST}:3001"
    echo ""
}

main
