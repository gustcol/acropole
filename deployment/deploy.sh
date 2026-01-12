#!/usr/bin/env bash
set -euo pipefail

# Acropole Integrity System Deployment Script
# This script deploys the integrity agent and related files to a target VM via SSH.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Default configuration
DEFAULT_SSH_USER="root"
DEFAULT_REMOTE_DIR="/usr/local/bin"
DEFAULT_SERVICE_DIR="/etc/systemd/system"
DEFAULT_CONFIG_DIR="/etc/acropole"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Show usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS] TARGET_HOST

Deploy the Acropole integrity system to a target VM via SSH.

ARGUMENTS:
    TARGET_HOST         Target host (IP or hostname) for deployment

OPTIONS:
    -h, --help              Show this help message
    -u, --user USER         SSH user (default: $DEFAULT_SSH_USER)
    -i, --identity FILE     SSH identity file (private key)
    -p, --port PORT         SSH port (default: 22)
    -b, --bin-dir DIR       Remote directory for binaries (default: $DEFAULT_REMOTE_DIR)
    -s, --service-dir DIR   Remote systemd directory (default: $DEFAULT_SERVICE_DIR)
    -c, --config-dir DIR    Remote config directory (default: $DEFAULT_CONFIG_DIR)
    -m, --metadata-url URL  Metadata service URL (default: http://metadata-service:8080)
    -I, --image-id ID       Initial image ID (default: ubuntu-v1)
    --skip-binaries         Skip copying binaries (assume they're already there)
    --skip-service          Skip installing systemd service
    --skip-config           Skip creating config directory
    --dry-run               Show what would be done without executing

ENVIRONMENT VARIABLES:
    SSH_USER                SSH user
    SSH_IDENTITY            SSH identity file
    SSH_PORT                SSH port
    METADATA_URL            Metadata service URL
    IMAGE_ID                Initial image ID

EXAMPLES:
    # Basic deployment
    $0 192.168.1.100

    # Custom SSH user and key
    $0 -u admin -i ~/.ssh/my_key 192.168.1.100

    # Custom metadata service
    $0 -m http://192.168.1.50:8080 192.168.1.100

    # Dry run
    $0 --dry-run 192.168.1.100

EOF
}

# Parse command line arguments
parse_args() {
    local args=()

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
            ;;
            -u|--user)
                SSH_USER="$2"
                shift 2
            ;;
            -i|--identity)
                SSH_IDENTITY="$2"
                shift 2
            ;;
            -p|--port)
                SSH_PORT="$2"
                shift 2
            ;;
            -b|--bin-dir)
                REMOTE_BIN_DIR="$2"
                shift 2
            ;;
            -s|--service-dir)
                REMOTE_SERVICE_DIR="$2"
                shift 2
            ;;
            -c|--config-dir)
                REMOTE_CONFIG_DIR="$2"
                shift 2
            ;;
            -m|--metadata-url)
                METADATA_URL="$2"
                shift 2
            ;;
            -I|--image-id)
                IMAGE_ID="$2"
                shift 2
            ;;
            --skip-binaries)
                SKIP_BINARIES=true
                shift
            ;;
            --skip-service)
                SKIP_SERVICE=true
                shift
            ;;
            --skip-config)
                SKIP_CONFIG=true
                shift
            ;;
            --dry-run)
                DRY_RUN=true
                shift
            ;;
            -*)
                log_error "Unknown option: $1"
                usage
                exit 1
            ;;
            *)
                args+=("$1")
                shift
            ;;
        esac
    done

    # Set target host from remaining arguments
    if [[ ${#args[@]} -eq 1 ]]; then
        TARGET_HOST="${args[0]}"
        elif [[ ${#args[@]} -gt 1 ]]; then
        log_error "Too many arguments. Only one target host is expected."
        usage
        exit 1
    else
        log_error "Target host is required"
        usage
        exit 1
    fi
}

# Initialize variables from environment or defaults
init_vars() {
    SSH_USER="${SSH_USER:-$DEFAULT_SSH_USER}"
    SSH_PORT="${SSH_PORT:-22}"
    REMOTE_BIN_DIR="${REMOTE_BIN_DIR:-$DEFAULT_REMOTE_DIR}"
    REMOTE_SERVICE_DIR="${REMOTE_SERVICE_DIR:-$DEFAULT_SERVICE_DIR}"
    REMOTE_CONFIG_DIR="${REMOTE_CONFIG_DIR:-$DEFAULT_CONFIG_DIR}"
    METADATA_URL="${METADATA_URL:-http://metadata-service:8080}"
    IMAGE_ID="${IMAGE_ID:-ubuntu-v1}"

    # Build SSH command base
    SSH_BASE="ssh -p $SSH_PORT"
    if [[ -n "${SSH_IDENTITY:-}" ]]; then
        SSH_BASE="$SSH_BASE -i $SSH_IDENTITY"
    fi

    log_info "Configuration:"
    log_info "  Target: $SSH_USER@$TARGET_HOST:$SSH_PORT"
    log_info "  Binaries: $REMOTE_BIN_DIR"
    log_info "  Service: $REMOTE_SERVICE_DIR"
    log_info "  Config: $REMOTE_CONFIG_DIR"
    log_info "  Metadata URL: $METADATA_URL"
    log_info "  Image ID: $IMAGE_ID"
}

# Execute command on remote host
remote_exec() {
    local cmd="$1"

    if [[ "${DRY_RUN:-false}" == "true" ]]; then
        log_info "[DRY RUN] Would execute: $SSH_BASE $TARGET_HOST '$cmd'"
        return 0
    fi

    log_info "Executing: $cmd"
    $SSH_BASE "$TARGET_HOST" "$cmd"
}

# Copy file to remote host
remote_copy() {
    local src="$1"
    local dst="$2"
    local scp_cmd="scp -P $SSH_PORT"

    if [[ -n "${SSH_IDENTITY:-}" ]]; then
        scp_cmd="$scp_cmd -i $SSH_IDENTITY"
    fi

    if [[ "${DRY_RUN:-false}" == "true" ]]; then
        log_info "[DRY RUN] Would copy: $src -> $dst"
        return 0
    fi

    log_info "Copying: $src -> $dst"
    $scp_cmd "$src" "$SSH_USER@$TARGET_HOST:$dst"
}

# Check SSH connectivity
check_connectivity() {
    log_step "Checking SSH connectivity..."

    if ! remote_exec "echo 'SSH connection successful'"; then
        log_error "Failed to connect to $TARGET_HOST via SSH"
        exit 1
    fi

    log_info "SSH connectivity confirmed"
}

# Deploy binaries
deploy_binaries() {
    if [[ "${SKIP_BINARIES:-false}" == "true" ]]; then
        log_step "Skipping binary deployment"
        return 0
    fi

    log_step "Deploying binaries..."

    # Check if local binaries exist
    local agent_bin="$SCRIPT_DIR/../target/release/integrity-agent"
    local collector_bin="$SCRIPT_DIR/../target/release/baseline-collector"

    if [[ ! -f "$agent_bin" ]]; then
        log_error "Integrity agent binary not found: $agent_bin"
        log_error "Build the project first with: cargo build --release"
        exit 1
    fi

    if [[ ! -f "$collector_bin" ]]; then
        log_error "Baseline collector binary not found: $collector_bin"
        log_error "Build the project first with: cargo build --release"
        exit 1
    fi

    # Create remote directory
    remote_exec "mkdir -p $REMOTE_BIN_DIR"

    # Copy binaries
    remote_copy "$agent_bin" "$REMOTE_BIN_DIR/integrity-agent"
    remote_copy "$collector_bin" "$REMOTE_BIN_DIR/baseline-collector"

    # Make binaries executable
    remote_exec "chmod +x $REMOTE_BIN_DIR/integrity-agent"
    remote_exec "chmod +x $REMOTE_BIN_DIR/baseline-collector"

    log_info "Binaries deployed successfully"
}

# Deploy systemd service
deploy_service() {
    if [[ "${SKIP_SERVICE:-false}" == "true" ]]; then
        log_step "Skipping service deployment"
        return 0
    fi

    log_step "Deploying systemd service..."

    # Copy service file
    local service_file="$SCRIPT_DIR/integrity-agent.service"
    remote_copy "$service_file" "$REMOTE_SERVICE_DIR/integrity-agent.service"

    # Update service file with correct paths and configuration
    remote_exec "sed -i 's|/usr/local/bin/integrity-agent|$REMOTE_BIN_DIR/integrity-agent|g' $REMOTE_SERVICE_DIR/integrity-agent.service"
    remote_exec "sed -i 's|Environment=\"IMAGE_ID=.*|Environment=\"IMAGE_ID=$IMAGE_ID\"|g' $REMOTE_SERVICE_DIR/integrity-agent.service"
    remote_exec "sed -i 's|Environment=\"METADATA_URL=.*|Environment=\"METADATA_URL=$METADATA_URL\"|g' $REMOTE_SERVICE_DIR/integrity-agent.service"

    # Reload systemd
    remote_exec "systemctl daemon-reload"

    # Enable service
    remote_exec "systemctl enable integrity-agent"

    log_info "Systemd service deployed and enabled"
}

# Create configuration directory
create_config() {
    if [[ "${SKIP_CONFIG:-false}" == "true" ]]; then
        log_step "Skipping config directory creation"
        return 0
    fi

    log_step "Creating configuration directory..."

    # Create config directory
    remote_exec "mkdir -p $REMOTE_CONFIG_DIR"

    # Create a basic config file (optional)
    remote_exec "cat > $REMOTE_CONFIG_DIR/agent.conf << 'EOF'
# Acropole Integrity Agent Configuration
IMAGE_ID=$IMAGE_ID
METADATA_URL=$METADATA_URL
WATCH_PATHS=/bin,/sbin,/usr/bin,/usr/sbin,/etc
EOF"

    log_info "Configuration directory created"
}

# Start the service
start_service() {
    log_step "Starting integrity agent service..."

    # Stop service first if it's running
    remote_exec "systemctl stop integrity-agent || true"

    # Start service
    if remote_exec "systemctl start integrity-agent"; then
        log_info "Service started successfully"
    else
        log_error "Failed to start service"
        log_error "Check logs with: journalctl -u integrity-agent -n 50"
        return 1
    fi

    # Check service status
    sleep 2
    if remote_exec "systemctl is-active integrity-agent"; then
        log_info "Service is running"
    else
        log_error "Service is not running"
        return 1
    fi
}

# Verify deployment
verify_deployment() {
    log_step "Verifying deployment..."

    # Check if binaries exist
    if ! remote_exec "test -x $REMOTE_BIN_DIR/integrity-agent"; then
        log_error "Integrity agent binary not found or not executable"
        return 1
    fi

    if ! remote_exec "test -x $REMOTE_BIN_DIR/baseline-collector"; then
        log_error "Baseline collector binary not found or not executable"
        return 1
    fi

    # Check if service file exists
    if ! remote_exec "test -f $REMOTE_SERVICE_DIR/integrity-agent.service"; then
        log_error "Service file not found"
        return 1
    fi

    # Check service status
    if ! remote_exec "systemctl is-active integrity-agent"; then
        log_error "Service is not active"
        return 1
    fi

    log_info "Deployment verification completed successfully"
}

# Show deployment summary
show_summary() {
    log_info "Deployment Summary:"
    log_info "  Target: $SSH_USER@$TARGET_HOST:$SSH_PORT"
    log_info "  Service: integrity-agent"
    log_info "  Status: $(remote_exec "systemctl is-active integrity-agent" && echo "Running" || echo "Not running")"
    log_info "  Image ID: $IMAGE_ID"
    log_info "  Metadata URL: $METADATA_URL"

    if [[ "${DRY_RUN:-false}" != "true" ]]; then
        log_info ""
        log_info "Useful commands:"
        log_info "  Check status: systemctl status integrity-agent"
        log_info "  View logs: journalctl -u integrity-agent -f"
        log_info "  Stop service: systemctl stop integrity-agent"
        log_info "  Start service: systemctl start integrity-agent"
        log_info "  Update packages: $SCRIPT_DIR/update_vm_and_baseline.sh"
    fi
}

# Main function
main() {
    log_info "Starting Acropole integrity system deployment"

    init_vars
    check_connectivity
    deploy_binaries
    deploy_service
    create_config
    start_service
    verify_deployment
    show_summary

    log_info "Deployment completed successfully!"
}

# Parse arguments and run
parse_args "$@"
main
