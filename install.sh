#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Cilium Vision — Multi-OS Installer
# Supports: Fedora, RHEL/CentOS, Ubuntu/Debian, openSUSE, Arch
# ─────────────────────────────────────────────────────────────
set -euo pipefail

VERSION="2.0.0"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/cilium-vision"
DATA_DIR="/var/lib/cilium-vision"
LOG_DIR="/var/log/cilium-vision"
SERVICE_USER="cilium-vision"
API_PORT="${CILIUM_VISION_PORT:-9191}"
UI_PORT="${CILIUM_VISION_UI_PORT:-3001}"







info()  { echo "ℹ️  $*"; }
ok()    { echo "✅ $*"; }
warn()  { echo "⚠️  $*"; }
err()   { echo "❌ $*" >&2; }
die()   { err "$*"; exit 1; }

# ─── OS Detection ────────────────────────────────────────────
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS_ID="${ID}"
        OS_VERSION="${VERSION_ID:-unknown}"
        OS_NAME="${PRETTY_NAME:-$ID}"
    else
        die "Cannot detect OS. /etc/os-release not found."
    fi
    info "Detected: ${OS_NAME}"
}

# ─── Package Manager ────────────────────────────────────────
install_deps() {
    info "Installing dependencies..."
    case "${OS_ID}" in
        fedora)
            sudo dnf install -y gcc make openssl-devel pkg-config \
                curl wget git redis nodejs npm kubectl \
                pam-devel firewalld 2>/dev/null || true
            ;;
        rhel|centos|rocky|alma)
            sudo dnf install -y gcc make openssl-devel pkg-config \
                curl wget git redis nodejs npm \
                pam-devel firewalld 2>/dev/null || true
            ;;
        ubuntu|debian|pop)
            sudo apt-get update -qq
            sudo apt-get install -y -qq build-essential pkg-config \
                libssl-dev curl wget git redis-server \
                nodejs npm libpam0g-dev ufw 2>/dev/null || true
            ;;
        opensuse*|sles)
            sudo zypper install -y gcc make libopenssl-devel pkg-config \
                curl wget git redis nodejs npm pam-devel \
                firewalld 2>/dev/null || true
            ;;
        arch|manjaro)
            sudo pacman -Sy --noconfirm gcc make openssl pkg-config \
                curl wget git redis nodejs npm pam \
                ufw 2>/dev/null || true
            ;;
        *)
            warn "Unknown OS '${OS_ID}' — install deps manually"
            ;;
    esac
    ok "Dependencies installed"
}

# ─── Rust ────────────────────────────────────────────────────
install_rust() {
    if command -v rustc &>/dev/null; then
        ok "Rust already installed: $(rustc --version)"
        return
    fi
    info "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    ok "Rust installed: $(rustc --version)"
}

# ─── Node.js Check ───────────────────────────────────────────
check_node() {
    if ! command -v node &>/dev/null; then
        die "Node.js not found. Install Node.js 18+ first."
    fi
    local node_ver
    node_ver=$(node -v | sed 's/v//' | cut -d. -f1)
    if [ "$node_ver" -lt 18 ]; then
        die "Node.js 18+ required. Found: $(node -v)"
    fi
    ok "Node.js: $(node -v)"
}

# ─── kubectl Check ───────────────────────────────────────────
check_kubectl() {
    if command -v kubectl &>/dev/null; then
        ok "kubectl: $(kubectl version --client --short 2>/dev/null || kubectl version --client 2>/dev/null | head -1)"
    else
        warn "kubectl not found — install for Kubernetes integration"
    fi
}

# ─── Kubeconfig Check ───────────────────────────────────────
check_kubeconfig() {
    if [ -f "${KUBECONFIG:-$HOME/.kube/config}" ]; then
        ok "Kubeconfig found: ${KUBECONFIG:-$HOME/.kube/config}"
        if kubectl cluster-info &>/dev/null 2>&1; then
            ok "Kubernetes cluster reachable"
        else
            warn "Kubernetes cluster not reachable"
        fi
    else
        warn "No kubeconfig found — required for Cilium/Hubble integration"
    fi
}

# ─── Cilium Check ────────────────────────────────────────────
check_cilium() {
    if command -v cilium &>/dev/null; then
        ok "Cilium CLI: $(cilium version --client 2>/dev/null | head -1)"
    else
        warn "Cilium CLI not found — some features may be limited"
    fi

    if command -v hubble &>/dev/null; then
        ok "Hubble CLI found"
    else
        warn "Hubble CLI not found — flow monitoring requires Hubble"
    fi
}

# ─── Build ───────────────────────────────────────────────────
build_api() {
    info "Building Web API (Rust)..."
    cd web-api
    cargo build --release
    cd ..
    ok "Web API built: web-api/target/release/cilium-vision-api"
}

build_ui() {
    info "Building Web UI (React)..."
    cd web-ui
    npm ci --silent 2>/dev/null || npm install --silent
    npm run build
    cd ..
    ok "Web UI built: web-ui/dist/"
}

build_tui() {
    info "Building TUI..."
    cargo build --release
    ok "TUI built: target/release/cilium-tui"
}

# ─── Install ─────────────────────────────────────────────────
install_binaries() {
    info "Installing binaries to ${INSTALL_DIR}..."
    sudo install -m 755 web-api/target/release/cilium-vision-api "${INSTALL_DIR}/"
    sudo install -m 755 target/release/cilium-tui "${INSTALL_DIR}/" 2>/dev/null || true
    ok "Binaries installed"
}

install_ui_files() {
    info "Installing UI files..."
    sudo mkdir -p "${DATA_DIR}/ui"
    sudo cp -r web-ui/dist/* "${DATA_DIR}/ui/"
    sudo chown -R root:root "${DATA_DIR}/ui"
    ok "UI files installed to ${DATA_DIR}/ui"
}

# ─── Config ──────────────────────────────────────────────────
create_config() {
    info "Creating configuration..."
    sudo mkdir -p "${CONFIG_DIR}" "${LOG_DIR}" "${DATA_DIR}"

    if [ ! -f "${CONFIG_DIR}/config.env" ]; then
        local jwt_secret
        jwt_secret=$(openssl rand -hex 32 2>/dev/null || head -c 64 /dev/urandom | base64 | tr -dc 'a-zA-Z0-9' | head -c 64)
        sudo tee "${CONFIG_DIR}/config.env" > /dev/null <<EOF
# Cilium Vision Configuration
CILIUM_VISION_HOST=0.0.0.0
CILIUM_VISION_PORT=${API_PORT}
HUBBLE_ADDRESS=localhost:4245
REDIS_URL=redis://localhost:6379
JWT_SECRET=${jwt_secret}
RUST_LOG=info
ALLOWED_ORIGINS=http://localhost:${UI_PORT},http://localhost:3000
UI_DIST_DIR=${DATA_DIR}/ui
EOF
        sudo chmod 600 "${CONFIG_DIR}/config.env"
        ok "Config created: ${CONFIG_DIR}/config.env"
    else
        ok "Config already exists: ${CONFIG_DIR}/config.env"
    fi
}

# ─── PAM Service ─────────────────────────────────────────────
create_pam_service() {
    if [ ! -f /etc/pam.d/cilium-vision ]; then
        info "Creating PAM service..."
        sudo tee /etc/pam.d/cilium-vision > /dev/null <<'EOF'
#%PAM-1.0
auth       required     pam_unix.so
account    required     pam_unix.so
EOF
        ok "PAM service created: /etc/pam.d/cilium-vision"
    fi
}

# ─── Systemd Service ─────────────────────────────────────────
create_systemd_service() {
    info "Creating systemd service..."
    sudo tee /usr/lib/systemd/system/cilium-vision-api.service > /dev/null <<EOF
[Unit]
Description=Cilium Vision API Server
Documentation=https://github.com/ssahani/cilium-flow
After=network-online.target redis.service
Wants=network-online.target
Requires=redis.service

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_USER}
EnvironmentFile=${CONFIG_DIR}/config.env
ExecStart=${INSTALL_DIR}/cilium-vision-api
Restart=on-failure
RestartSec=5
LimitNOFILE=65536
TimeoutStopSec=30

# Security hardening
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=${LOG_DIR} ${DATA_DIR}
PrivateTmp=yes
NoNewPrivileges=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes

[Install]
WantedBy=multi-user.target
EOF

    sudo tee /usr/lib/systemd/system/cilium-vision-ui.service > /dev/null <<EOF
[Unit]
Description=Cilium Vision Web UI (nginx)
After=network-online.target cilium-vision-api.service
Wants=cilium-vision-api.service

[Service]
Type=simple
ExecStart=/usr/bin/python3 -m http.server ${UI_PORT} --directory ${DATA_DIR}/ui
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    ok "Systemd services created"
}

# ─── Service User ────────────────────────────────────────────
create_service_user() {
    if ! id "${SERVICE_USER}" &>/dev/null; then
        info "Creating service user..."
        sudo useradd -r -s /sbin/nologin -m -d "${DATA_DIR}" "${SERVICE_USER}"
        ok "User '${SERVICE_USER}' created"
    fi
    sudo chown -R "${SERVICE_USER}:${SERVICE_USER}" "${DATA_DIR}" "${LOG_DIR}" 2>/dev/null || true
}

# ─── Firewall ────────────────────────────────────────────────
configure_firewall() {
    info "Configuring firewall..."
    if command -v firewall-cmd &>/dev/null && systemctl is-active firewalld &>/dev/null; then
        sudo firewall-cmd --permanent --add-port="${API_PORT}/tcp" 2>/dev/null || true
        sudo firewall-cmd --permanent --add-port="${UI_PORT}/tcp" 2>/dev/null || true
        sudo firewall-cmd --reload 2>/dev/null || true
        ok "Firewalld: ports ${API_PORT} and ${UI_PORT} opened"
    elif command -v ufw &>/dev/null; then
        sudo ufw allow "${API_PORT}/tcp" 2>/dev/null || true
        sudo ufw allow "${UI_PORT}/tcp" 2>/dev/null || true
        ok "UFW: ports ${API_PORT} and ${UI_PORT} allowed"
    elif command -v iptables &>/dev/null; then
        sudo iptables -A INPUT -p tcp --dport "${API_PORT}" -j ACCEPT 2>/dev/null || true
        sudo iptables -A INPUT -p tcp --dport "${UI_PORT}" -j ACCEPT 2>/dev/null || true
        ok "iptables: ports ${API_PORT} and ${UI_PORT} opened"
    else
        warn "No firewall detected — manually open ports ${API_PORT} and ${UI_PORT}"
    fi
}

# ─── Health Check ────────────────────────────────────────────
health_check() {
    info "Running health check..."
    local retries=10
    local delay=2
    for i in $(seq 1 $retries); do
        if curl -sf "http://localhost:${API_PORT}/health" >/dev/null 2>&1; then
            ok "API server healthy at http://localhost:${API_PORT}"
            return 0
        fi
        sleep $delay
    done
    warn "API not responding on port ${API_PORT} (may need manual start)"
    return 1
}

# ─── Remote Deploy ───────────────────────────────────────────
deploy_remote() {
    local target_host="${1:?Usage: $0 deploy-remote <user@host>}"
    info "Deploying to ${target_host}..."

    # Sync binaries
    rsync -avz --progress \
        web-api/target/release/cilium-vision-api \
        "${target_host}:/tmp/cilium-vision-api"

    # Sync UI
    rsync -avz --progress \
        web-ui/dist/ \
        "${target_host}:/tmp/cilium-vision-ui/"

    # Sync install script
    rsync -avz install.sh "${target_host}:/tmp/cilium-vision-install.sh"

    # Remote install
    ssh "${target_host}" bash <<'REMOTE_EOF'
set -e
sudo install -m 755 /tmp/cilium-vision-api /usr/local/bin/
sudo mkdir -p /var/lib/cilium-vision/ui
sudo cp -r /tmp/cilium-vision-ui/* /var/lib/cilium-vision/ui/
echo "Binaries deployed. Run 'sudo bash /tmp/cilium-vision-install.sh setup-services' to configure systemd."
REMOTE_EOF

    ok "Remote deployment complete to ${target_host}"
}

# ─── Setup Services (post-deploy) ────────────────────────────
setup_services() {
    create_service_user
    create_config
    create_pam_service
    create_systemd_service
    configure_firewall
    ok "Services configured. Start with: sudo systemctl start cilium-vision-api"
}

# ─── Start / Stop / Status ───────────────────────────────────
start_services() {
    info "Starting Cilium Vision..."
    sudo systemctl start redis 2>/dev/null || true
    sudo systemctl start cilium-vision-api
    sudo systemctl start cilium-vision-ui 2>/dev/null || true
    sudo systemctl enable cilium-vision-api 2>/dev/null || true
    sleep 2
    health_check
}

stop_services() {
    info "Stopping Cilium Vision..."
    sudo systemctl stop cilium-vision-ui 2>/dev/null || true
    sudo systemctl stop cilium-vision-api 2>/dev/null || true
    ok "Services stopped"
}

show_status() {
    echo ""
    echo "🔷 ═══════════════════════════════════════════════════"
    echo "🔷    Cilium Vision v${VERSION} — Status"
    echo "🔷 ═══════════════════════════════════════════════════"
    echo ""

    for svc in cilium-vision-api cilium-vision-ui redis; do
        if systemctl is-active "$svc" &>/dev/null; then
            echo -e "  ${GREEN}●${NC} ${svc}: active"
        else
            echo -e "  ${RED}●${NC} ${svc}: inactive"
        fi
    done

    echo ""
    if curl -sf "http://localhost:${API_PORT}/health" >/dev/null 2>&1; then
        echo -e "  ${GREEN}●${NC} API: http://localhost:${API_PORT}"
    else
        echo -e "  ${RED}●${NC} API: not responding"
    fi
    echo -e "  ${BLUE}●${NC} UI:  http://localhost:${UI_PORT}"
    echo ""
}

# ─── Uninstall ───────────────────────────────────────────────
uninstall() {
    warn "Uninstalling Cilium Vision..."
    stop_services 2>/dev/null || true
    sudo systemctl disable cilium-vision-api cilium-vision-ui 2>/dev/null || true
    sudo rm -f /usr/lib/systemd/system/cilium-vision-api.service
    sudo rm -f /usr/lib/systemd/system/cilium-vision-ui.service
    sudo rm -f /etc/pam.d/cilium-vision
    sudo rm -f "${INSTALL_DIR}/cilium-vision-api"
    sudo rm -f "${INSTALL_DIR}/cilium-tui"
    sudo rm -rf "${DATA_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"
    sudo userdel "${SERVICE_USER}" 2>/dev/null || true
    sudo systemctl daemon-reload
    ok "Cilium Vision uninstalled"
}

# ─── Full Install ────────────────────────────────────────────
full_install() {
    echo ""
    echo "🔷 ═══════════════════════════════════════════════════"
    echo "🔷    Cilium Vision v${VERSION} — Installer"
    echo "🔷 ═══════════════════════════════════════════════════"
    echo ""

    detect_os
    install_deps
    install_rust
    check_node
    check_kubectl
    check_kubeconfig
    check_cilium

    echo ""
    info "Building components..."
    build_api
    build_ui
    build_tui 2>/dev/null || warn "TUI build skipped (optional)"

    echo ""
    info "Installing..."
    install_binaries
    install_ui_files
    setup_services

    echo ""
    info "Starting services..."
    start_services

    echo ""
    show_status
    echo "✅ Installation complete!"
    echo ""
}

# ─── Usage ───────────────────────────────────────────────────
usage() {
    cat <<EOF
Cilium Vision v${VERSION} — Installer

Usage: $0 <command>

Commands:
  install           Full installation (build + install + configure + start)
  build             Build all components (API + UI + TUI)
  build-api         Build Web API only
  build-ui          Build Web UI only
  setup-services    Configure systemd, user, PAM, firewall
  start             Start all services
  stop              Stop all services
  status            Show service status
  health            Run health check
  deploy-remote     Deploy to remote host: $0 deploy-remote user@host
  uninstall         Remove everything
  help              Show this help

Environment:
  CILIUM_VISION_PORT      API port (default: 9191)
  CILIUM_VISION_UI_PORT   UI port (default: 3001)

EOF
}

# ─── Main ────────────────────────────────────────────────────
case "${1:-help}" in
    install)        full_install ;;
    build)          build_api; build_ui; build_tui 2>/dev/null || true ;;
    build-api)      build_api ;;
    build-ui)       build_ui ;;
    setup-services) setup_services ;;
    start)          start_services ;;
    stop)           stop_services ;;
    status)         show_status ;;
    health)         health_check ;;
    deploy-remote)  deploy_remote "${2:-}" ;;
    uninstall)      uninstall ;;
    help|--help|-h) usage ;;
    *)              err "Unknown command: $1"; usage; exit 1 ;;
esac
