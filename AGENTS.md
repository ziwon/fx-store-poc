# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

FX-Store is a high-performance time-series storage system specifically designed for foreign exchange market data. It targets sub-100μs latency and 37 Gbps throughput on a single server using Rust with specialized optimizations for financial data workloads.

## Build Commands

```bash
# Development build
cargo build

# Optimized release build (use for performance testing)
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Run tests
cargo test

# Run benchmarks (when implemented)
cargo bench

# Check for compilation errors without building
cargo check
```

## Architecture Overview

### Core Data Flow
The system implements a three-layer architecture optimized for cache efficiency and compression:

1. **OHLCV Records** (`src/types.rs`): 40-byte fixed-width structures with nanosecond timestamps, designed to fit within CPU cache lines
2. **Block Compression** (`src/block.rs`): Daily blocks of 1440 records (1-minute intervals) compressed with zstd level 3
3. **Lock-free Storage** (`src/store.rs`): DashMap-based concurrent storage with background compression workers

### Key Components

**FxStore** (`src/store.rs`): Main storage engine with nested hash maps:
- Outer: `symbol_id -> DashMap<date, CompressedBlock>`
- Inner: `date -> CompressedBlock` mapping for each symbol
- Background compression via crossbeam channels to avoid blocking writes

**OHLCV Structure** (`src/types.rs`): Cache-optimized 40-byte records:
- Fixed-point prices (5 decimal precision, multiplied by 100,000)
- Compile-time size assertion ensures exactly 40 bytes
- Packed representation with explicit padding for alignment

**CompressedBlock** (`src/block.rs`): Daily compression units:
- Stores 1440 minute-bars per trading day
- Lazy decompression with RwLock-protected cache
- Block-level metadata (date, symbol_id)

**SIMD Filtering** (`src/query.rs`): AVX2-accelerated price filtering:
- Processes 8 records simultaneously using 256-bit vectors
- Falls back to scalar processing for remainder records

### Data Format Details

**CSV Import Format**: HISTDATA-compatible format expected:
```
YYYYMMDD HHMMSS,Open,High,Low,Close,Volume
```

**Binary Storage**: Memory-mapped files with custom .fxd format:
- File header with magic number "FXSTORE1"
- Symbol table for string-to-ID mapping  
- Compressed daily blocks with zstd
- B+Tree index for timestamp-based queries (planned)

**Price Encoding**: Fixed-point integers for deterministic calculations:
- Multiply by 100,000 for 5 decimal places (standard FX precision)
- Example: 1.23456 EUR/USD → 123456 as u32

### Concurrency Model

**Lock-free Reads**: DashMap enables concurrent queries without blocking
**Background Compression**: Crossbeam channels decouple compression from ingestion
**NUMA Awareness**: Designed for binding to specific CPU cores/memory nodes
**Async I/O**: Memory-mapped files avoid kernel transitions for zero-copy access

## Development Environment Notes

Currently optimized for WSL2 development on Intel Core Ultra 9 285K with 32GB RAM. Production deployment requires native Linux for AF_XDP network optimizations.

**WSL2 Limitations**:
- No AF_XDP support (kernel bypass networking)
- Limited huge pages and NUMA control
- Higher latency due to virtualization overhead

**Performance Targets** (native Linux):
- Import: 5M records/sec
- Query: 100M records/sec  
- P99 Latency: <100μs
- Compression: 10:1 ratio

## Key Dependencies

- **bincode 1.3**: Serialization (note: version pinned for API stability)
- **zstd 0.13**: Block compression with level 3 speed/ratio balance
- **dashmap 6.1**: Lock-free concurrent hash maps
- **rayon 1.8**: Data-parallel CSV processing
- **crossbeam 0.8**: Channel-based background compression
- **memmap2 0.9**: Zero-copy file access
- **parking_lot 0.12**: RwLocks for cache management

## Critical Size Constraints

The OHLCV struct MUST remain exactly 40 bytes (enforced by compile-time assertion). When modifying:
- Check padding calculation: 8+4+4+4+4+4+2+10 = 40 bytes
- Regenerate padding if fields change
- Consider cache line alignment (64 bytes) for block arrays

## Testing and Data

Place test CSV files in `data/` directory with HISTDATA format. The main.rs example expects:
- `data/EURUSD_2023.csv`
- `data/GBPUSD_2023.csv`

For CSV parsing, the system expects space-separated date/time in first column:
```
20230101 000000,1.05432,1.05478,1.05401,1.05456,12500
```