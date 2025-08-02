# Production Deployment Guide

## System Requirements

### Hardware Specifications

#### Minimum Requirements
- **CPU**: 8 cores, x86_64 with AVX2
- **Memory**: 32 GB DDR4
- **Storage**: 1 TB NVMe SSD
- **Network**: 10 Gbps NIC

#### Recommended Production Setup
- **CPU**: AMD EPYC 7763 or Intel Xeon Gold 6354
- **Memory**: 128 GB DDR4-3200 (8x16GB, populate all channels)
- **Storage**: 2x 3.84 TB NVMe (RAID 1)
- **Network**: Mellanox ConnectX-6 Dx (100 Gbps)

### Operating System

```bash
# Ubuntu 22.04 LTS recommended
lsb_release -a

# Kernel requirements
uname -r  # Should be 5.15+ for AF_XDP support
```

## Pre-deployment Setup
### 1. System Tuning
Create /etc/sysctl.d/99-fx-store.conf:
```bash
# Network optimizations
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_congestion_control = bbr
net.core.default_qdisc = fq

# Memory optimizations
vm.nr_hugepages = 8192
vm.max_map_count = 655300
vm.swappiness = 1

# File system
fs.file-max = 2097152
fs.aio-max-nr = 1048576
```

Apply settings:
```bash
sudo sysctl -p /etc/sysctl.d/99-fx-store.conf
```

### 2. Install Dependencies
```bash
# System packages
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libbpf-dev \
    libelf-dev \
    libssl-dev \
    pkg-config \
    clang \
    llvm \
    linux-tools-generic \
    linux-cloud-tools-generic \
    numactl \
    hwloc \
    msr-tools

# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default stable
rustup component add rustfmt clippy

# Performance tools
sudo apt-get install -y \
    linux-tools-$(uname -r) \
    bpftrace \
    bcc-tools \
    sysstat \
    iotop
```

### 3. Configure Huge Pages
```bash
# Persistent huge pages
echo "vm.nr_hugepages = 8192" | sudo tee -a /etc/sysctl.conf

# Mount hugetlbfs
sudo mkdir -p /mnt/huge
echo "hugetlbfs /mnt/huge hugetlbfs defaults 0 0" | sudo tee -a /etc/fstab
sudo mount -a

# Verify
grep HugePages /proc/meminfo
```

## Installation
#### 1. Build from Source
```bash
# Clone repository
git clone https://github.com/ziwon/fx-store.git
cd fx-store

# Production build
RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld" \
    cargo build --release

# Run tests
cargo test --release

# Install binary
sudo cp target/release/fx-store /usr/local/bin/
sudo chmod +x /usr/local/bin/fx-store
```

#### 2. Create Service User
```bash
# Create dedicated user
sudo useradd -r -s /bin/false -m -d /var/lib/fx-store fx-store

# Create directories
sudo mkdir -p /var/lib/fx-store/data
sudo mkdir -p /var/log/fx-store
sudo mkdir -p /etc/fx-store

# Set permissions
sudo chown -R fx-store:fx-store /var/lib/fx-store
sudo chown -R fx-store:fx-store /var/log/fx-store
sudo chown -R fx-store:fx-store /etc/fx-store
```

#### 3. Configuration
Create /etc/fx-store/config.toml:
```toml
[server]
bind_address = "0.0.0.0:9000"
worker_threads = 16
max_connections = 10000

[storage]
data_dir = "/var/lib/fx-store/data"
cache_size_gb = 64
compression_level = 3
sync_interval_ms = 1000

[network]
af_xdp_enabled = true
numa_node = 0
rx_queue_size = 8192
tx_queue_size = 8192

[performance]
huge_pages = true
cpu_affinity = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15]
io_uring_enabled = true

[monitoring]
prometheus_enabled = true
prometheus_port = 9001

[logging]
level = "info"
file = "/var/log/fx-store/fx-store.log"
max_size_mb = 100
max_backups = 10
```

#### 4. Systemd Service
Create /etc/systemd/system/fx-store.service:
```ini
[Unit]
Description=FX Store High-Performance Time Series Database
After=network-online.target
Wants=network-online.target

[Service]
Type=notify
User=fx-store
Group=fx-store
WorkingDirectory=/var/lib/fx-store

# Binary and config
ExecStart=/usr/local/bin/fx-store serve -c /etc/fx-store/config.toml

# Process management
Restart=always
RestartSec=5
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=30

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/fx-store /var/log/fx-store

# Performance
LimitNOFILE=1048576
LimitMEMLOCK=infinity
CPUSchedulingPolicy=fifo
CPUSchedulingPriority=99
IOSchedulingClass=realtime
IOSchedulingPriority=0

# NUMA
NUMAPolicy=bind
NUMAMask=0

# Capabilities for AF_XDP
AmbientCapabilities=CAP_NET_ADMIN CAP_SYS_ADMIN CAP_BPF
CapabilityBoundingSet=CAP_NET_ADMIN CAP_SYS_ADMIN CAP_BPF

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable fx-store
sudo systemctl start fx-store
sudo systemctl status fx-store
```

## Network Configuration

### 1. NIC Setup
```bash
# Set interface up
sudo ip link set dev eth0 up

# Configure RSS
sudo ethtool -L eth0 combined 16

# Disable offloads that interfere with AF_XDP
sudo ethtool -K eth0 lro off
sudo ethtool -K eth0 gro off
sudo ethtool -K eth0 gso off

# Set ring sizes
sudo ethtool -G eth0 rx 4096 tx 4096

# Enable flow steering
sudo ethtool -K eth0 ntuple on
```

### 2. IRQ Affinity
```bash
# Find IRQs for network interface
grep eth0 /proc/interrupts

# Set affinity (example for IRQs 24-39)
for i in {24..39}; do
    echo $((1 << ($i - 24))) | sudo tee /proc/irq/$i/smp_affinity
done
```

### 3. AF_XDP Setup
```bash
# Load AF_XDP program
sudo ip link set dev eth0 xdpgeneric obj /usr/local/lib/fx-store/xdp_prog.o sec xdp

# Verify
ip link show dev eth0 | grep xdp
```

## Storage Setup
#### 1. File System
```bash
# Format with XFS (recommended)
sudo mkfs.xfs -f -d agcount=16 /dev/nvme0n1
sudo mkdir -p /data
sudo mount -o noatime,nodiratime,nobarrier /dev/nvme0n1 /data

# Add to fstab
echo "/dev/nvme0n1 /data xfs noatime,nodiratime,nobarrier 0 0" | sudo tee -a /etc/fstab
```

### 2. I/O Scheduler
```bash
# Set to none for NVMe
echo none | sudo tee /sys/block/nvme0n1/queue/scheduler

# Increase queue depth
echo 1024 | sudo tee /sys/block/nvme0n1/queue/nr_requests
```

## Monitoring Setup

### 1. Prometheus
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'fx-store'
    static_configs:
      - targets: ['localhost:9001']
```

### 2. Grafana Dashboard
Import dashboard from monitoring/grafana-dashboard.json

Key metrics to monitor:
- Ingestion rate (messages/sec)
- Query latency (P50, P99, P99.9)
- Memory usage
- CPU utilization by core
- Network throughput
- Disk I/O

### 3. Alerting Rules
```yaml
# alerts.yml
groups:
  - name: fx-store
    rules:
      - alert: HighQueryLatency
        expr: fx_store_query_latency_p99 > 0.001
        for: 5m
        annotations:
          summary: "High query latency detected"
          
      - alert: LowIngestionRate
        expr: rate(fx_store_messages_total[5m]) < 100000
        for: 10m
        annotations:
          summary: "Ingestion rate below threshold"
```

## Backup and Recovery

### 1. Backup Strategy
```bash
#!/bin/bash
# backup.sh

# Snapshot data directory
sudo btrfs subvolume snapshot -r /data /data/.snapshots/$(date +%Y%m%d_%H%M%S)

# Compress and archive
tar -czf /backup/fx-store-$(date +%Y%m%d).tar.gz \
    --exclude='*.tmp' \
    /data/.snapshots/latest

# Upload to S3
aws s3 cp /backup/fx-store-$(date +%Y%m%d).tar.gz \
    s3://backup-bucket/fx-store/
```

### 2. Recovery Procedure
```bash
# Stop service
sudo systemctl stop fx-store

# Restore from backup
tar -xzf /backup/fx-store-20231231.tar.gz -C /

# Verify data integrity
fx-store verify --data-dir /data

# Start service
sudo systemctl start fx-store
```

## Security Hardening

### 1. Firewall Rules
```bash
# Allow only necessary ports
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 9000/tcp comment "FX-Store API"
sudo ufw allow 9001/tcp comment "Prometheus metrics"
sudo ufw enable
```

### 2. TLS Configuration
```toml
# config.toml
[server.tls]
enabled = true
cert_file = "/etc/fx-store/certs/server.crt"
key_file = "/etc/fx-store/certs/server.key"
client_auth = true
ca_file = "/etc/fx-store/certs/ca.crt"
```

## Maintenance

### Daily Tasks

Monitor disk usage
Check log files for errors
Verify backup completion

### Weekly Tasks

Review performance metrics
Update system packages
Test backup restoration

### Monthly Tasks

Analyze query patterns
Optimize indexes
Update fx-store binary

## Troubleshooting

### Common Issues

**AF_XDP not loading**
```bash
# Check kernel config
grep CONFIG_XDP /boot/config-$(uname -r)

# Verify BPF permissions
getcap /usr/local/bin/fx-store
```

**High memory usage**
```bash
# Check huge page usage
grep Huge /proc/meminfo

# Monitor per-NUMA usage
numastat -m
```

**Performance degradation**
```bash
# Check CPU frequency
cpupower frequency-info

# Monitor interrupts
watch -n1 'cat /proc/interrupts | grep eth0'
```

## Health Checks
```bash
#!/bin/bash
# health_check.sh

# API health
curl -f http://localhost:9000/health || exit 1

# Check ingestion rate
RATE=$(curl -s http://localhost:9001/metrics | grep fx_store_messages_total | awk '{print $2}')
if (( $(echo "$RATE < 100000" | bc -l) )); then
    echo "Low ingestion rate: $RATE"
    exit 1
fi

echo "Health check passed"
```

## Capacity Planning

### Storage Requirements
Daily data: ~10 GB compressed
Monthly: ~300 GB
Yearly: ~3.6 TB

Recommended: 2x capacity for compression workspace
Total: ~8 TB for 1 year of data

### Performance Scaling

| Workload         | CPU Cores | Memory  | Network  |
|------------------|-----------|---------|----------|
| Light (1M msg/s) | 4         | 16 GB   | 1 Gbps   |
| Medium (10M msg/s)| 8         | 64 GB   | 10 Gbps  |
| Heavy (100M msg/s)| 16        | 128 GB  | 100 Gbps |
