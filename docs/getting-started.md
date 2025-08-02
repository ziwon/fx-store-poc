# Getting Started

## Prerequisites

### System Requirements
- **OS**: Linux kernel 5.4+ (for AF_XDP)
- **CPU**: x86_64 with AVX2 support
- **Memory**: 32GB minimum, 128GB recommended
- **Network**: 10G+ NIC with AF_XDP support
- **Storage**: NVMe SSD with 1TB+ space

### Software Dependencies
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install system dependencies
sudo apt-get update
sudo apt-get install -y \
    libbpf-dev \
    libclang-dev \
    libelf-dev \
    linux-headers-$(uname -r) \
    pkg-config

# Enable huge pages
echo 1024 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages
```

## Installation
### From Source

```bash
# Clone repository
git clone https://github.com/ziwon/fx-store.git
cd fx-store

# Build with optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Run tests
cargo test

# Install
sudo cargo install --path .
```


### Using Docker
```bash
# Build image
docker build -t fx-store .

# Run container
docker run -d \
    --name fx-store \
    --network host \
    --privileged \
    -v /dev/hugepages:/dev/hugepages \
    -v ./data:/data \
    fx-store
```

### Basic Usage
#### 1. Import Historical Data
```bash
# Import single file
fx-store import \
    --symbol EURUSD \
    --file data/EURUSD_2023.csv

# Batch import
fx-store import \
    --dir data/ \
    --pattern "*_2023.csv"

# Import with custom settings
fx-store import \
    --symbol EURUSD \
    --file data/EURUSD_2023.csv \
    --compression-level 3 \
    --block-size 1440
```

#### 2. Query Data
```bash
# Simple time range query
fx-store query \
    --symbol EURUSD \
    --start "2023-01-01T00:00:00Z" \
    --end "2023-12-31T23:59:59Z" \
    --output results.csv

# Query with filtering
fx-store query \
    --symbol EURUSD \
    --start "2023-06-01" \
    --end "2023-06-30" \
    --min-price 1.0800 \
    --max-price 1.0900

# Export to different formats
fx-store query \
    --symbol EURUSD \
    --last "1 week" \
    --format json > output.json
```

#### 3. Real-time Streaming
```bash
# Stream live data
fx-store stream \
    --symbol EURUSD \
    --output-format json

# Stream with aggregation
fx-store stream \
    --symbol EURUSD \
    --aggregate "5min" \
    --indicators sma:20,rsi:14
```

### Configuration
#### Config File (fx-store.toml)
```toml
[server]
bind_address = "0.0.0.0:9000"
worker_threads = 8

[storage]
data_dir = "/var/lib/fx-store"
cache_size_gb = 16
compression_level = 3

[network]
af_xdp_enabled = true
numa_node = 0
rx_queue_size = 4096

[performance]
huge_pages = true
cpu_affinity = [0, 1, 2, 3]
io_uring_enabled = true
```

#### Environment Variables
```bash
# Override config file
export FX_STORE_DATA_DIR=/custom/path
export FX_STORE_CACHE_SIZE=32G
export FX_STORE_LOG_LEVEL=debug

# NUMA settings
export FX_STORE_NUMA_NODE=0
export FX_STORE_CPU_AFFINITY="0-7"
```

### API Examples
#### Rust Client
```rust
use fx_store::{Client, TimeRange};
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to server
    let client = Client::connect("localhost:9000").await?;
    
    // Query historical data
    let end = Utc::now();
    let start = end - Duration::days(7);
    
    let records = client
        .query("EURUSD")
        .range(start, end)
        .execute()
        .await?;
    
    println!("Found {} records", records.len());
    
    // Subscribe to real-time updates
    let mut stream = client.subscribe("EURUSD").await?;
    
    while let Some(ohlcv) = stream.next().await {
        println!("Real-time: {:?}", ohlcv);
    }
    
    Ok(())
}
```

#### Python Client
```python
import fx_store
from datetime import datetime, timedelta

# Connect
client = fx_store.Client("localhost:9000")

# Query
end = datetime.utcnow()
start = end - timedelta(days=30)

df = client.query(
    symbol="EURUSD",
    start=start,
    end=end,
    as_dataframe=True
)

# Calculate indicators
sma_20 = df['close'].rolling(20).mean()
print(f"Current SMA(20): {sma_20.iloc[-1]}")

# Stream real-time
for ohlcv in client.stream("EURUSD"):
    print(f"Price: {ohlcv.close}")
``` 

### Monitoring
#### Metrics Endpoint
```bash
# Prometheus metrics
curl http://localhost:9000/metrics

# Health check
curl http://localhost:9000/health

# Statistics
fx-store stats --format json
```

#### Grafana Dashboard
Import the provided dashboard from monitoring/grafana-dashboard.json to visualize:
- Ingestion rate
- Query latency
- Cache hit ratio
- Memory usage
- Network throughput

### Troubleshooting
#### Common Issues

1. AF_XDP not working
```bash
# Check kernel support
grep CONFIG_XDP_SOCKETS /boot/config-$(uname -r)

# Load required modules
sudo modprobe veth

```

2. **Performance issues**
```bash
# Check NUMA topology
numactl --hardware

# Verify huge pages
grep HugePages /proc/meminfo
```

3. **High memory usage**
```bash
# Adjust cache size
fx-store config set cache.size 8G

# Clear cache
fx-store cache clear
```

## Next Steps
- Read the Performance Guide for optimization tips
- Check API Reference for detailed documentation
- See Deployment Guide for production setup