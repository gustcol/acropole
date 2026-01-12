# Acropole

**Golden Image Integrity System and Immutable Infrastructure Automation**

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Docker](https://img.shields.io/badge/Docker-20.10%2B-2496ED?logo=docker)](https://www.docker.com/)
[![Proxmox](https://img.shields.io/badge/Proxmox-7.0%2B-E57000?logo=proxmox)](https://www.proxmox.com/)

```
git clone https://github.com/gustcol/acropole.git
```

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Components](#components)
  - [Baseline Collector](#1-baseline-collector)
  - [Metadata Service](#2-metadata-service)
  - [Integrity Agent](#3-integrity-agent)
  - [Dashboard](#4-dashboard)
- [Deployment Options](#deployment-options)
- [Installation](#installation)
- [Usage](#usage)
- [Maintenance and Updates](#maintenance-and-updates)
- [Security](#security)
- [Project Structure](#project-structure)
- [Requirements](#requirements)
- [Contributing](#contributing)
- [License](#license)

---

## Overview

**Acropole** is a complete solution for maintaining secure and immutable infrastructure through real-time integrity monitoring and automated K3s cluster deployment.

The system ensures that Virtual Machines (VMs) deployed from a "Golden Image" remain in their desired state, detecting unauthorized modifications and triggering fail-closed responses.

### Key Features

- **Real-Time Integrity Monitoring**: Instant detection of modifications to critical files using fanotify
- **SHA-512 Cryptographic Hashing**: Secure file integrity verification
- **Fail-Closed Design**: Automatic response to integrity violations
- **Immutable Baselines**: Stored externally, cannot be modified from within VMs
- **Multi-Platform**: Support for KVM/Libvirt, Proxmox VE, and Docker
- **Web Dashboard**: Modern React interface for visual monitoring

---

## Architecture

```
                    +-------------------+
                    |   Build Pipeline  |
                    +-------------------+
                            |
            +---------------+---------------+
            |               |               |
            v               v               v
    +--------------+  +-----------+  +-------------+
    | Golden Image |->| Baseline  |->| Metadata    |
    |    Build     |  | Collector |  | Service     |
    +--------------+  +-----------+  +-------------+
                                            |
        +-----------------------------------+
        |           |            |          |
        v           v            v          v
    +-------+  +--------+  +--------+  +----------+
    | KVM   |  |Proxmox |  | Docker |  | Dashboard|
    +-------+  +--------+  +--------+  +----------+
        |           |            |
        +-----------+------------+
                    |
            +-------v-------+
            |  Deployed VM  |
            +---------------+
                    |
            +-------v-------+
            |   Integrity   |
            |     Agent     |
            +---------------+
                    |
        +-----------+-----------+
        |                       |
        v                       v
+---------------+       +---------------+
| File Monitor  |       | Violation     |
| (fanotify)    |       | Detection     |
+---------------+       +---------------+
                               |
                        +------v------+
                        | Fail-Closed |
                        |   Action    |
                        +-------------+
```

### Data Flow

1. **Build Phase**: Golden Image is created and scanned by the Baseline Collector
2. **Storage**: Baseline (SHA-512 hashes + metadata) sent to Metadata Service
3. **Deploy**: VMs are created from the Golden Image via KVM, Proxmox, or Docker
4. **Runtime**: Integrity Agent monitors files in real-time
5. **Detection**: Violations trigger fail-closed actions and alerts

---

## Components

### 1. Baseline Collector

Scans filesystems during Golden Image creation to create a "fingerprint" (baseline).

**Features:**
- Computes SHA-512 hashes of critical files
- Extracts metadata (permissions, owner, group)
- Excludes volatile directories (`/proc`, `/sys`, `/dev`, `/run`, `/tmp`)
- Automatic upload to Metadata Service

**Usage:**
```bash
./baseline-collector \
  --scan-path /path/to/golden/image \
  --image-id ubuntu-v1 \
  --metadata-url http://metadata-service:8080
```

### 2. Metadata Service

High-performance REST API for baseline storage and retrieval.

**Technologies:**
- Rust with Actix-web
- Sled (embedded KV database)
- RESTful API

**Endpoints:**
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/baselines` | Store new baseline |
| GET | `/baselines/{image_id}` | Retrieve baseline |
| GET | `/health` | Health check |

**Usage:**
```bash
./metadata-service --db-path /var/lib/acropole/metadata-db --port 8080
```

### 3. Integrity Agent

Agent that runs inside deployed VMs, verifying file integrity in real-time.

**Features:**
- Real-time monitoring via fanotify (Linux)
- Integrity verification against external baselines
- Fail-closed actions on violations
- Heartbeats to Metadata Service

**Detected Anomaly Types:**
- **Modified**: Hash differs from baseline
- **Metadata Changed**: Permissions/UID/GID altered
- **Added**: File exists locally but not in baseline
- **Deleted**: File in baseline but missing locally

**Usage:**
```bash
./integrity-agent \
  --image-id ubuntu-v1 \
  --mode monitor \
  --watch-paths /bin,/sbin,/usr/bin,/etc \
  --metadata-url http://metadata-service:8080
```

### 4. Dashboard

Modern web interface for real-time system monitoring.

**Features:**
- Agent status visualization
- Integrity violation alerts
- System health metrics
- Charts and visual analytics

**Stack:**
- React 18 with modern hooks
- Material-UI
- React Query
- Recharts

---

## Deployment Options

### Docker (Recommended for Development)

```bash
# Build and run all services
docker-compose up -d

# Only metadata service
docker-compose up metadata-service

# Only dashboard
docker-compose up dashboard
```

**Access:**
- Dashboard: `http://localhost:3000`
- API: `http://localhost:8080`

### Proxmox VE

Automated deployment using cloud-init:

```bash
# Upload cloud-init files
cp proxmox/*.yaml /var/lib/vz/snippets/

# Create VM with cloud-init
qm create 100 --name acropole-vm-01 --memory 2048 --net0 virtio,bridge=vmbr0
qm set 100 --ide2 local-lvm:cloudinit
qm set 100 --cicustom "user=local:snippets/user-data.yaml,meta=local:snippets/meta-data.yaml"

# Start VM
qm start 100
```

### KVM/Libvirt

Documentation available in `k8s-prov_server/` for complete K3s cluster deployment.

---

## Installation

### Prerequisites

- **Rust**: 1.70+
- **Docker**: 20.10+ (for containerized deployment)
- **Proxmox VE**: 7.0+ (for enterprise virtualization)
- **Ubuntu**: 22.04+ (for K3s deployment)

### Building Rust Components

```bash
# Clone the repository
git clone https://github.com/gustcol/acropole.git
cd acropole

# Build in release mode
cargo build --release

# Binaries available at ./target/release/
# - baseline-collector
# - metadata-service
# - integrity-agent
```

### Quick Deploy with Docker Compose

```bash
# Start the entire stack
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f
```

---

## Usage

### 1. Start the Metadata Service

```bash
./target/release/metadata-service --db-path ./metadata-db
```

### 2. Collect Baseline from Golden Image

```bash
./target/release/baseline-collector \
  --scan-path / \
  --image-id ubuntu-golden-v1 \
  --metadata-url http://localhost:8080
```

### 3. Run the Integrity Agent on VMs

```bash
./target/release/integrity-agent \
  --image-id ubuntu-golden-v1 \
  --mode monitor \
  --watch-paths /bin,/sbin,/usr/bin,/etc \
  --metadata-url http://localhost:8080
```

### 4. Install as Systemd Service

```bash
# Copy binaries
sudo cp target/release/integrity-agent /usr/local/bin/
sudo cp target/release/baseline-collector /usr/local/bin/

# Install service
sudo cp deployment/integrity-agent.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable integrity-agent
sudo systemctl start integrity-agent
```

### Advanced Configuration

Create `/etc/integrity-agent.toml`:

```toml
image_id = "ubuntu-v1"
mode = "monitor"
watch_paths = ["/bin", "/sbin", "/usr/bin", "/etc", "/opt/myapp"]
exclude_patterns = ["*.log", "/tmp/*", "/var/cache/*"]
max_consecutive_anomalies = 5
metadata_url = "http://metadata-service:8080"
```

---

## Maintenance and Updates

Since the system uses fail-closed design, package updates require a special workflow:

### Update Flow

```
Admin -> Stop Agent -> Update System -> New Baseline -> Restart Agent
```

### Automated Script

```bash
# Run update with automatic re-baseline
sudo /usr/local/bin/update_vm_and_baseline.sh

# With custom metadata service
sudo /usr/local/bin/update_vm_and_baseline.sh --metadata-url http://192.168.1.100:8080
```

### Manual Process

```bash
# 1. Stop the agent
sudo systemctl stop integrity-agent

# 2. Update packages
sudo apt-get update && sudo apt-get upgrade -y

# 3. Create new baseline
NEW_IMAGE_ID="ubuntu-updated-$(date +%Y%m%d-%H%M%S)"
sudo baseline-collector --scan-path / --image-id "$NEW_IMAGE_ID"

# 4. Update configuration and restart
sudo sed -i "s/IMAGE_ID=.*/IMAGE_ID=$NEW_IMAGE_ID/" /etc/systemd/system/integrity-agent.service
sudo systemctl daemon-reload
sudo systemctl start integrity-agent
```

---

## Security

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Insider Threats | Immutable baselines stored externally |
| External Attacks | Real-time detection + fail-closed |
| Supply Chain | SHA-512 verification of binaries |
| Persistence | Rootkit/backdoor detection |

### Security Features

- **Immutable Baselines**: Cannot be modified from within VMs
- **Cryptographic Hashing**: SHA-512 for tamper detection
- **Fail-Closed Design**: Automatic response to violations
- **Audit Trail**: Complete logging of all integrity events
- **mTLS**: Secure communication between components (planned)

---

## Project Structure

```
acropole/
|-- Cargo.toml                    # Rust workspace configuration
|-- Cargo.lock                    # Dependency lock file
|-- docker-compose.yml            # Service orchestration
|-- DESIGN.md                     # Design documentation
|-- DASHBOARD_SPEC.md             # Dashboard specification
|-- README.md                     # This file
|
|-- baseline-collector/           # Baseline collection
|   |-- Cargo.toml
|   +-- src/
|
|-- integrity-agent/              # Monitoring agent
|   |-- Cargo.toml
|   |-- Dockerfile
|   +-- src/
|
|-- integrity-common/             # Shared library
|   |-- Cargo.toml
|   +-- src/
|
|-- metadata-service/             # Metadata service
|   |-- Cargo.toml
|   |-- Dockerfile
|   +-- src/
|
|-- dashboard/                    # React dashboard
|   |-- package.json
|   |-- Dockerfile
|   |-- nginx.conf
|   +-- src/
|       |-- components/
|       |-- services/
|       +-- App.js
|
|-- deployment/                   # Deployment scripts
|   |-- integrity-agent.service   # Systemd service
|   |-- deploy.sh                 # Remote deployment
|   +-- update_vm_and_baseline.sh # Update script
|
|-- proxmox/                      # Proxmox deployment
|   |-- README.md
|   |-- user-data.yaml
|   |-- meta-data.yaml
|   +-- vendor-data.yaml
|
|-- test-data/                    # Test data
+-- test-db/                      # Test database
```

---

## Requirements

### Minimum

| Component | Version |
|-----------|---------|
| Rust | 1.70+ |
| Docker | 20.10+ |
| Ubuntu | 22.04+ |

### For Proxmox

| Component | Version |
|-----------|---------|
| Proxmox VE | 7.0+ |
| QEMU | 6.0+ |

### Rust Dependencies

- `actix-web` - Web framework
- `sled` - Embedded database
- `tokio` - Async runtime
- `sha2` - SHA-512 hashing
- `walkdir` - Filesystem traversal
- `clap` - CLI parsing
- `tracing` - Structured logging

---

## Performance

| Metric | Value |
|--------|-------|
| Baseline Collection | ~1000 files/second |
| Hash Verification | Sub-millisecond (cached) |
| Memory Usage | Proportional to file count |
| Detection Latency | Real-time via fanotify |

### Scalability Recommendations

- **Large Filesystems**: Implement incremental baseline updates
- **High Event Rates**: Add rate limiting and batching
- **Memory Constraints**: Implement LRU cache for baseline entries
- **Distributed Systems**: Use Redis/etcd for shared storage

---

## Contributing

Contributions are welcome! The project supports:

- Custom monitoring policies
- Integration with existing orchestration platforms
- Extension to additional filesystems or cloud providers
- Enhanced violation response mechanisms

### How to Contribute

1. Fork the repository
2. Create a branch for your feature (`git checkout -b feature/new-feature`)
3. Commit your changes (`git commit -m 'Add new feature'`)
4. Push to the branch (`git push origin feature/new-feature`)
5. Open a Pull Request

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

## Useful Links

- **Repository**: [github.com/gustcol/acropole](https://github.com/gustcol/acropole)
- **Issues**: [github.com/gustcol/acropole/issues](https://github.com/gustcol/acropole/issues)
- **Proxmox Documentation**: [proxmox/README.md](proxmox/README.md)
- **Design Specification**: [DESIGN.md](DESIGN.md)
- **Dashboard Specification**: [DASHBOARD_SPEC.md](DASHBOARD_SPEC.md)

---

**Acropole** - Protecting your infrastructure with immutable integrity.
