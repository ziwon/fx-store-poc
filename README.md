# FX-Store: High-Performance Time-Series Storage for Financial Data

A production-grade, single-server solution for storing and querying foreign exchange market data, inspired by [Databento's architecture](https://databento.com/blog/real-time-tick-data) but optimized for [FX/HISTDATA](http://www.histdata.com/) workloads.

## 📋 Table of Contents

- [Architecture Overview](./docs/architecture.md)
- [Getting Started](./docs/getting-started.md)
- [Performance Guide](./docs/performance.md)
  - [Cache Optimization](./docs/cache-optimization.md)
  - [Hardware Specifications (WSL)](./docs/hardware-wsl.md)
  - [Hardware Specifications (Linux)](./docs/hardware-native-linux.md)
- [API Reference](./docs/api-reference.md)
- [Data Format Specification](./docs/data-format.md)
- [Deployment Guide](./docs/deployment.md)
- [Benchmarks](./docs/benchmarks.md)

## 🚀 Key Features

- **Single-server 37 Gbps** processing capability
- **10:1 compression** with zstd
- **Nanosecond precision** timestamps
- **Zero-copy** query operations
- **NUMA-aware** memory management
- **Lock-free** concurrent access
- **SIMD-accelerated** filtering

## 🏗️ Technology Stack

- **Language**: Rust
- **Network**: AF_XDP (kernel bypass)
- **Storage**: Memory-mapped files + zstd compression
- **Concurrency**: DashMap (lock-free hashmap)
- **SIMD**: AVX2 for filtering operations
- **Time Sync**: TSC + NTP calibration

## 📊 Performance Highlights (Not Proven: Goal)

| Metric            | Value            |
| ----------------- | ---------------- |
| Import Speed      | 5M records/sec   |
| Query Speed       | 100M records/sec |
| Compression Ratio | 10:1             |
| P99 Latency       | < 100μs          |
| Memory Usage      | 1/10 of raw data |

## 🔧 Quick Start

```bash
# Build
cargo build --release

# Import data
./fx-store import --symbol EURUSD --file data/EURUSD_2024.csv

# Query
./fx-store query --symbol EURUSD --start "2024-01-01" --end "2024-12-31"
```
