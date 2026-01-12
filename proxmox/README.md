# Proxmox Deployment Guide for Acropole Integrity System

This guide explains how to deploy the Acropole Golden Image Integrity System on Proxmox virtual machines using cloud-init.

## Prerequisites

- Proxmox VE 7.0 or later
- Ubuntu 22.04+ cloud image (download from [Ubuntu Cloud Images](https://cloud-images.ubuntu.com/))
- SSH access to your Proxmox server
- Basic familiarity with Proxmox web interface or CLI

## Cloud-Init Configuration Files

This directory contains three cloud-init configuration files:

- **`user-data.yaml`**: Main configuration (users, packages, commands)
- **`meta-data.yaml`**: Instance metadata (hostname, instance ID)
- **`vendor-data.yaml`**: Optional vendor-specific settings (currently minimal)

## Deployment Steps

### Method 1: Using Proxmox Web Interface

1. **Upload Cloud Image**
   - Download Ubuntu cloud image: `wget https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img`
   - Upload to Proxmox storage via web interface

2. **Create VM**
   - Create new VM with appropriate resources (2GB RAM, 20GB disk recommended)
   - Use the uploaded cloud image as the primary disk
   - Enable QEMU Guest Agent in VM options

3. **Configure Cloud-Init**
   - In VM hardware settings, add a CD-ROM drive
   - Use "Cloud-Init" type and select your storage
   - Configure the cloud-init settings in the web interface or upload the YAML files

4. **Start VM**
   - Start the VM - it will automatically apply the cloud-init configuration

### Method 2: Using Proxmox CLI

1. **Download and prepare cloud image**

```bash
# Download Ubuntu cloud image
wget https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img

# Create VM (adjust VMID, storage, and network as needed)
qm create 9000 --name acropole-template --memory 2048 --net0 virtio,bridge=vmbr0

# Import the disk
qm importdisk 9000 jammy-server-cloudimg-amd64.img local-lvm

# Attach the disk
qm set 9000 --scsihw virtio-scsi-pci --scsi0 local-lvm:vm-9000-disk-0

# Add cloud-init drive
qm set 9000 --ide2 local-lvm:cloudinit

# Configure boot
qm set 9000 --boot c --bootdisk scsi0

# Enable QEMU guest agent
qm set 9000 --agent enabled=1
```

2. **Upload cloud-init files to Proxmox snippets**

```bash
# Create snippets directory if it doesn't exist
mkdir -p /var/lib/vz/snippets

# Copy cloud-init files
cp user-data.yaml /var/lib/vz/snippets/acropole-user-data.yaml
cp meta-data.yaml /var/lib/vz/snippets/acropole-meta-data.yaml
cp vendor-data.yaml /var/lib/vz/snippets/acropole-vendor-data.yaml
```

3. **Apply cloud-init configuration**

```bash
# Set cloud-init configuration
qm set 9000 --cicustom "user=local:snippets/acropole-user-data.yaml,meta=local:snippets/acropole-meta-data.yaml,vendor=local:snippets/acropole-vendor-data.yaml"

# Optional: Set SSH key (replace with your actual key)
qm set 9000 --sshkeys "ssh-rsa AAAAB3NzaC1yc2EA... user@host"

# Convert to template (optional)
qm template 9000
```

4. **Clone and deploy VMs**

```bash
# Clone from template (if created)
qm clone 9000 100 --name acropole-vm-01
qm clone 9000 101 --name acropole-vm-02

# Or create new VMs and apply cloud-init
qm create 100 --name acropole-vm-01 --memory 2048 --net0 virtio,bridge=vmbr0
qm set 100 --ide2 local-lvm:cloudinit
qm set 100 --cicustom "user=local:snippets/acropole-user-data.yaml,meta=local:snippets/acropole-meta-data.yaml,vendor=local:snippets/acropole-vendor-data.yaml"
```

## Post-Deployment Steps

1. **Access the VM**
   - SSH to the VM using the configured user (default: `acropole`)
   - Check cloud-init logs: `sudo cat /var/log/cloud-init-output.log`

2. **Install Integrity Agent**
   - Copy the integrity agent binary to the VM
   - Run the agent with appropriate parameters:

```bash
./integrity-agent --image-id ubuntu-v1 --mode monitor --metadata-url http://your-metadata-service:8080
```

3. **Verify Setup**
   - Check that QEMU guest agent is running: `systemctl status qemu-guest-agent`
   - Verify network connectivity to metadata service
   - Test integrity monitoring functionality

## Troubleshooting

### Cloud-Init Not Applied

- Check Proxmox logs: `cat /var/log/pve/tasks/active`
- Verify cloud-init drive is properly attached
- Check VM console for error messages

### Network Issues

- Ensure correct bridge configuration (`vmbr0` or your bridge)
- Check firewall rules on Proxmox host
- Verify DHCP is working or configure static IP

### SSH Access Issues

- Verify SSH key is properly configured
- Check `/etc/ssh/sshd_config` on the VM
- Ensure firewall allows SSH (port 22)

## Security Considerations

- Always use SSH keys instead of passwords
- Configure proper firewall rules
- Keep cloud-init files secure (they contain sensitive configuration)
- Regularly update the base cloud image
- Use Proxmox's built-in backup features

## Next Steps

After successful deployment, proceed with:

1. Setting up the metadata service
2. Creating baselines for your golden images
3. Configuring the monitoring dashboard
4. Setting up alerting and response procedures

For more information, see the main project README.md.
