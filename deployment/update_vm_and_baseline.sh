#!/usr/bin/env bash
set -euo pipefail

# Acropole VM Update and Re-baseline Script
# This script safely updates a VM and re-baselines it for the integrity system.
# It must be run as root.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_NAME="integrity-agent"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

# Default paths (can be overridden by environment variables)
AGENT_BIN="${AGENT_BIN:-/usr/local/bin/integrity-agent}"
COLLECTOR_BIN="${COLLECTOR_BIN:-/usr/local/bin/baseline-collector}"
METADATA_URL="${METADATA_URL:-http://metadata-service:8080}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root"
        exit 1
    fi
}

# Check if required binaries exist
check_binaries() {
    local missing=0

    if [[ ! -x "$AGENT_BIN" ]]; then
        log_error "Integrity agent binary not found or not executable: $AGENT_BIN"
        missing=1
    fi

    if [[ ! -x "$COLLECTOR_BIN" ]]; then
        log_error "Baseline collector binary not found or not executable: $COLLECTOR_BIN"
        missing=1
    fi

    if [[ $missing -eq 1 ]]; then
        exit 1
    fi

    log_info "Required binaries found"
}

# Stop the integrity agent service
stop_agent() {
    log_info "Stopping integrity agent service..."

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        systemctl stop "$SERVICE_NAME"

        # Wait for service to stop
        local attempts=0
        while systemctl is-active --quiet "$SERVICE_NAME" && [[ $attempts -lt 30 ]]; do
            sleep 1
            ((attempts++))
        done

        if systemctl is-active --quiet "$SERVICE_NAME"; then
            log_error "Failed to stop integrity agent service"
            exit 1
        fi

        log_info "Integrity agent service stopped successfully"
    else
        log_warn "Integrity agent service is not running"
    fi
}

# Update system packages
update_packages() {
    log_info "Updating package lists..."
    apt-get update

    log_info "Upgrading packages..."
    apt-get upgrade -y

    log_info "Package update completed"
}

# Generate a new image ID based on current timestamp
generate_image_id() {
    local timestamp
    timestamp=$(date +"%Y%m%d-%H%M%S")
    echo "ubuntu-updated-${timestamp}"
}

# Create new baseline
create_baseline() {
    local new_image_id
    new_image_id=$(generate_image_id)

    log_info "Creating new baseline with image ID: $new_image_id"

    "$COLLECTOR_BIN" \
    --scan-path / \
    --image-id "$new_image_id" \
    --metadata-url "$METADATA_URL"

    log_info "New baseline created successfully"
    echo "$new_image_id"
}

# Update service configuration with new image ID
update_service_config() {
    local new_image_id=$1

    log_info "Updating service configuration with new image ID: $new_image_id"

    # Update the environment variable in the service file
    sed -i "s/^Environment=\"IMAGE_ID=.*/Environment=\"IMAGE_ID=$new_image_id\"/" "$SERVICE_FILE"

    # Reload systemd
    systemctl daemon-reload

    log_info "Service configuration updated"
}

# Start the integrity agent service
start_agent() {
    log_info "Starting integrity agent service..."

    systemctl start "$SERVICE_NAME"

    # Wait for service to start
    sleep 2

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        log_info "Integrity agent service started successfully"
    else
        log_error "Failed to start integrity agent service"
        log_error "Check logs with: journalctl -u $SERVICE_NAME -n 50"
        exit 1
    fi
}

# Verify the service is working correctly
verify_service() {
    log_info "Verifying service status..."

    # Check if service is running
    if ! systemctl is-active --quiet "$SERVICE_NAME"; then
        log_error "Service is not running"
        return 1
    fi

    # Check recent logs for errors
    local error_count
    error_count=$(journalctl -u "$SERVICE_NAME" --since "1 minute ago" --no-pager -q | grep -i error | wc -l)

    if [[ $error_count -gt 0 ]]; then
        log_warn "Found $error_count error(s) in recent logs"
        log_warn "Check logs with: journalctl -u $SERVICE_NAME -n 50"
    else
        log_info "Service verification completed - no recent errors found"
    fi
}

# Main function
main() {
    log_info "Starting VM update and re-baseline process"

    check_root
    check_binaries

    # Stop the agent before making changes
    stop_agent

    # Update packages
    update_packages

    # Create new baseline
    local new_image_id
    new_image_id=$(create_baseline)

    # Update service configuration
    update_service_config "$new_image_id"

    # Start the agent with new baseline
    start_agent

    # Verify everything is working
    verify_service

    log_info "VM update and re-baseline process completed successfully!"
    log_info "New image ID: $new_image_id"
    log_info "The integrity agent is now monitoring the updated system."
}

# Show usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Safely update VM packages and re-baseline the integrity system.

OPTIONS:
    -h, --help          Show this help message
    -m, --metadata-url  Metadata service URL (default: http://metadata-service:8080)
    -a, --agent-bin     Path to integrity-agent binary (default: /usr/local/bin/integrity-agent)
    -c, --collector-bin Path to baseline-collector binary (default: /usr/local/bin/baseline-collector)

ENVIRONMENT VARIABLES:
    METADATA_URL        Metadata service URL
    AGENT_BIN           Path to integrity-agent binary
    COLLECTOR_BIN       Path to baseline-collector binary

EXAMPLES:
    # Basic usage
    sudo $0

    # Custom metadata service
    sudo $0 --metadata-url http://192.168.1.100:8080

    # Custom binary paths
    sudo $0 --agent-bin /opt/acropole/bin/integrity-agent --collector-bin /opt/acropole/bin/baseline-collector

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
        ;;
        -m|--metadata-url)
            METADATA_URL="$2"
            shift 2
        ;;
        -a|--agent-bin)
            AGENT_BIN="$2"
            shift 2
        ;;
        -c|--collector-bin)
            COLLECTOR_BIN="$2"
            shift 2
        ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
        ;;
    esac
done

# Run main function
main
