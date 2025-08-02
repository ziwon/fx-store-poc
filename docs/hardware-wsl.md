# Hardware Specifications - WSL2 Development Environment

## Overview

This document describes the current hardware and software specifications for the FX-Store development environment running on Windows Subsystem for Linux 2 (WSL2).

## Hardware Specifications

### CPU
- **Model**: Intel Core Ultra 9 285K
- **Architecture**: x86_64
- **Cores**: 24 cores (single sdfsocket)
- **Threads per Core**: 1 (no hyperthreading)
- **Base Frequency**: ~3.7 GHz (BogoMIPS: 7372.79)
- **Address Space**: 46 bits physical, 48 bits virtual

### Cache Hierarchy
- **L1d Cache**: 1.1 MiB (24 instances)
- **L1i Cache**: 1.5 MiB (24 instances)
- **L2 Cache**: 72 MiB (24 instances)
- **L3 Cache**: 36 MiB (1 instance)

### SIMD Instruction Support
- **SSE**: SSE, SSE2, SSE4.1, SSE4.2
- **AVX**: AVX, AVX2, AVX-VNNI
- **Other**: FMA, AES-NI, SHA-NI, BMI1, BMI2
- **Advanced Features**: VAES, VPCLMULQDQ, GFNI

### Memory
- **Total RAM**: 32 GB (31 GiB available)
- **Available**: ~26 GiB free
- **Swap**: 8 GB
- **Memory Type**: DDR4/DDR5 (virtualized through Hyper-V)

### Storage
- **Primary Disk**: 1007 GB total
- **Available Space**: 732 GB
- **File System**: WSL2 virtual file system
- **Host Storage**: Windows NTFS (C: 512GB, D: 1.4TB)

### Network
- **Primary Interface**: eth0 (Hyper-V Virtual Ethernet)
- **MTU**: 1280 bytes
- **MAC Address**: <redacted>
- **Additional**: Docker bridge networks present

## Software Environment

### Operating System
- **Distribution**: Ubuntu 24.04.2 LTS (Noble Numbat)
- **Kernel**: Linux 5.15.167.4-microsoft-standard-WSL2
- **Platform**: WSL2 (Windows Subsystem for Linux)
- **Hypervisor**: Microsoft Hyper-V
- **Virtualization**: Full virtualization with VT-x support

### Development Toolchain
- **Rust**: 1.88.0 (6b00bc388 2025-06-23)
- **Cargo**: 1.88.0 (873a06493 2025-05-10)
- **GCC**: 11.2.0
- **GNU Binutils**: 2.37

### System Libraries
- Standard Ubuntu 24.04 package repository
- Docker runtime environment available
- WSL2-specific kernel modules and drivers

## Performance Characteristics

### Advantages for FX-Store Development

✅ **SIMD Optimization**
- Full AVX2 support for vectorized operations
- AES-NI for cryptographic operations
- Modern instruction set for high-performance computing

✅ **Multi-Core Processing**
- 24 cores available for parallel processing
- Excellent for concurrent query processing
- Good for multi-threaded compression/decompression

✅ **Memory Capacity**
- 32 GB sufficient for large datasets in memory
- Good for caching compressed blocks
- Adequate for development and testing workloads

✅ **Development Environment**
- Modern Rust toolchain
- Full Linux compatibility for development
- Easy integration with Windows development tools

### Limitations in WSL2

❌ **Network Performance**
- No AF_XDP support (requires native Linux kernel)
- Virtualized network stack adds latency
- Limited to standard UDP/TCP sockets
- No direct hardware access for network optimization

❌ **Memory Management**
- Limited huge pages support
- No direct NUMA control
- Virtualized memory management
- Cannot use memory-mapped files optimally

❌ **Real-time Performance**
- WSL2 adds virtualization overhead
- No real-time kernel scheduling
- Interrupt handling through hypervisor
- Higher and less predictable latency

❌ **Hardware Access**
- No direct access to network hardware
- Cannot configure IRQ affinity
- No access to performance counters
- Limited system tuning capabilities

## Benchmarking Expectations

### Expected Performance in WSL2

| Component | WSL2 Performance | Native Linux | Impact |
|-----------|------------------|--------------|---------|
| CPU Compute | ~95% | 100% | Minimal |
| Memory Bandwidth | ~90% | 100% | Low |
| Network Throughput | ~60% | 100% | High |
| Storage I/O | ~80% | 100% | Medium |
| Latency | +50-100μs | Baseline | High |

### Realistic Targets for Development

| Metric | WSL2 Target | Production Target | Notes |
|--------|-------------|-------------------|-------|
| Import Speed | 800K rec/s | 1M+ rec/s | Good for testing |
| Query Latency | 200-500μs | <100μs | Acceptable for dev |
| Compression | 400 MB/s | 500+ MB/s | CPU-bound, good |
| Memory Usage | Same | Same | No virtualization impact |

## Development vs Production

### Suitable for Development
- ✅ Algorithm development and testing
- ✅ Data structure optimization
- ✅ Compression/decompression logic
- ✅ Query engine development
- ✅ Unit and integration testing
- ✅ Functional correctness validation

### Not Suitable for Production
- ❌ High-frequency trading applications
- ❌ Ultra-low latency requirements (<100μs)
- ❌ High-throughput network processing
- ❌ Real-time market data ingestion
- ❌ Performance benchmarking for production

## Recommendations

### For Current Development
```bash
# Optimize for WSL2 environment
export RUSTFLAGS="-C target-cpu=native"
export FX_STORE_CACHE_SIZE=8G
export FX_STORE_WORKER_THREADS=16

# Use available cores efficiently
cargo build --release --jobs 20
```

### For Production Migration
When moving to production, consider:

1. **Hardware Upgrade**
   - Native Linux server (Ubuntu 22.04 LTS recommended)
   - Multi-socket NUMA system
   - High-speed NIC with AF_XDP support
   - NVMe storage array

2. **System Configuration**
   - Kernel 5.15+ for AF_XDP support
   - Huge pages configuration
   - CPU isolation and affinity
   - Network interface optimization

3. **Performance Validation**
   - Run full benchmark suite on target hardware
   - Validate latency requirements
   - Test under production load patterns
   - Measure actual throughput capabilities

## Testing Strategy

### Development Testing (WSL2)
- Focus on correctness over performance
- Test data integrity and compression
- Validate query results and API functionality
- Develop unit tests and integration tests

### Performance Testing (Production Hardware)
- Benchmark on target deployment environment
- Measure end-to-end latency under load
- Test network throughput with real data feeds
- Validate memory usage patterns

## Conclusion

The current WSL2 environment with Intel Core Ultra 9 285K provides an excellent platform for FX-Store development, offering modern CPU features, sufficient memory, and full Linux compatibility. While it cannot achieve the ultra-low latency and high-throughput targets required for production financial applications, it is ideal for:

- Core algorithm development
- Data structure optimization
- Feature implementation and testing
- Code correctness validation

For production deployment targeting sub-100μs latency and 37 Gbps throughput, migration to native Linux hardware with specialized networking equipment will be necessary.