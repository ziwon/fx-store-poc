# Performance Optimization Guide

## Overview

This guide covers performance tuning for FX-Store to achieve maximum throughput and minimum latency on a single server.

## Hardware Optimization

### CPU Configuration

#### 1. Disable Hyper-Threading
```bash
# Check current state
lscpu | grep "Thread(s) per core"

# Disable in BIOS or:
echo off | sudo tee /sys/devices/system/cpu/smt/control
```

#### 2. Set Performance Governor
```bash
# Set all CPUs to performance mode
sudo cpupower frequency-set -g performance

# Disable CPU frequency scaling
for i in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    echo performance | sudo tee $i
done
```

#### 3. Isolate CPUs
```bash
# Add to kernel boot parameters
GRUB_CMDLINE_LINUX="isolcpus=8-15 nohz_full=8-15 rcu_nocbs=8-15"

# Update grub
sudo update-grub
sudo reboot
```

### Memory Configuration

#### 1. Enable Huge Pages
```bash
# Persistent configuration
echo "vm.nr_hugepages = 8192" | sudo tee -a /etc/sysctl.conf

# Immediate effect
echo 8192 | sudo tee /sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages

# Verify
grep HugePages /proc/meminfo
```

#### 2. NUMA Optimization
```bash
# Check NUMA topology
numactl --hardware

# Run with NUMA binding
numactl --cpunodebind=0 --membind=0 fx-store server

# Set NUMA balancing
echo 0 | sudo tee /proc/sys/kernel/numa_balancing
```

### Network Configuration

#### 1. NIC Optimization
```bash
# Increase ring buffer
sudo ethtool -G eth0 rx 4096 tx 4096

# Enable RSS
sudo ethtool -L eth0 combined 8

# Disable interrupt coalescing
sudo ethtool -C eth0 rx-usecs 0 tx-usecs 0

# Set IRQ affinity
sudo set_irq_affinity.sh 0-7 eth0
```

#### 2. AF_XDP Setup
```bash
# Load AF_XDP program
sudo ip link set dev eth0 xdpgeneric obj xdp_prog.o sec xdp

# Create AF_XDP socket
fx-store network --mode af_xdp --queues 8
```

## Software Optimization

### Compilation Flags
```toml
# Cargo.toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[profile.release.package."*"]
opt-level = 3
codegen-units = 1
```

```bash
# Build with native CPU optimizations
RUSTFLAGS="-C target-cpu=native -C link-arg=-fuse-ld=lld" \
    cargo build --release
```

### Code-Level Optimizations

#### 1. SIMD Usage
```rust
// Ensure AVX2 is used
#[target_feature(enable = "avx2")]
unsafe fn process_batch(data: &[OHLCV]) {
    // Implementation
}

// Check CPU features at runtime
if is_x86_feature_detected!("avx2") {
    unsafe { process_batch_avx2(data) }
} else {
    process_batch_scalar(data)
}
```

#### 2. Memory Prefetching
```rust
use std::arch::x86_64::_mm_prefetch;

// Prefetch next cache line
unsafe {
    _mm_prefetch(
        ptr.add(64) as *const i8,
        _MM_HINT_T0
    );
}
```

#### 3. Branch Prediction
```rust
// Use likely/unlikely hints
#[cold]
fn handle_error() { /* ... */ }

#[inline(always)]
fn hot_path() {
    // Performance critical code
}
```

## Benchmarking

### Micro-benchmarks
```bash
# Run built-in benchmarks
cargo bench

# Profile with perf
perf record --call-graph=dwarf cargo bench
perf report

# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bench import_bench
```

### Load Testing
```bash
# Generate test data
fx-store generate \
    --symbols 100 \
    --days 365 \
    --ticks-per-day 1000000

# Run load test
fx-store bench \
    --duration 60s \
    --threads 16 \
    --operation mixed
```

### Latency Testing
```rust
// Custom latency measurement
use quanta::Clock;

let clock = Clock::new();
let start = clock.now();

// Operation to measure
store.query(symbol, range);

let elapsed = clock.now() - start;
let nanos = clock.duration(elapsed).as_nanos();
```

## Monitoring & Profiling

### System Metrics
```bash
# CPU usage by core
mpstat -P ALL 1

# Memory bandwidth
sudo pcm-memory.x 1

# Network statistics
sar -n DEV 1

# Disk I/O
iostat -x 1
```

### Application Metrics
```rust
// Add custom metrics
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref QUERY_COUNTER: Counter = 
        register_counter!("fx_store_queries_total", "Total queries").unwrap();
    
    static ref QUERY_DURATION: Histogram = 
        register_histogram!(
            "fx_store_query_duration_seconds",
            "Query duration",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
        ).unwrap();
}
```

## Performance Targets

### Single-Core Performance

| Operation | Target       | Measurement      |
|-----------|--------------|------------------|
| Import    | 1M msg/sec   | Per thread       |
| Query     | 10M msg/sec  | Sequential scan  |
| Filter    | 100M msg/sec | SIMD operation   |
| Compress  | 500 MB/sec   | zstd level 3     |
### System-Wide Performance

| Metric        | Target   | Configuration     |
|---------------|----------|-------------------|
| Throughput    | 37 Gbps  | 8 cores, AF_XDP   |
| P50 Latency   | < 10μs   | Query operation   |
| P99 Latency   | < 100μs  | Under load        |
| P99.9 Latency | < 1ms    | Worst case        |

## Optimization Checklist

- ☐ CPU frequency scaling disabled
- ☐ Huge pages enabled (8GB+)
- ☐ NUMA properly configured
- ☐ IRQ affinity set
- ☐ Kernel bypass (AF_XDP) enabled
- ☐ Compiled with native CPU features
- ☐ Memory prefetching implemented
- ☐ SIMD operations verified
- ☐ Lock contention minimized
- ☐ False sharing eliminated

## Common Bottlenecks
### 1. Memory Bandwidth
**Symptom:** High memory controller utilization  
**Solution:**
- Improve cache locality
- Use NUMA-local allocations
- Implement prefetching

### 2. Lock Contention
**Symptom:** High spin time in perf  
**Solution:**
- Use lock-free data structures
- Increase sharding in DashMap
- Reduce critical section size

### 3. Context Switches
**Symptom:** High cs rate in vmstat  
**Solution:**
- Pin threads to CPUs
- Use busy-wait instead of blocking
- Increase batch sizes

### 4. TLB Misses
**Symptom:** High dTLB-miss rate  
**Solution:**
- Enable huge pages
- Reduce memory footprint
- Improve memory access patterns