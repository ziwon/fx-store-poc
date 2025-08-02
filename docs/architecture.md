# Architecture Overview

## System Design Philosophy

FX-Store follows the "boring technology" principle popularized by Databento, prioritizing:

- **Simplicity over complexity**
- **Hardware efficiency over distributed systems**
- **Binary formats over text protocols**
- **Flat files over databases**

## High-Level Architecture
```plaintext
┌─────────────────────────────────────────────────────────────┐
│                        Application Layer                    │ 
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ Time Range  │  │ SIMD Filter  │  │ Technical        │    │
│  │   Query     │  │   (AVX2)     │  │ Indicators       │    │
│  │             │  │              │  │ (SMA, RSI)       │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │ Compressed  │  │  DashMap     │  │   Memory Map     │    │
│  │   Blocks    │  │  (Lock-free) │  │     Files        │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
├─────────────────────────────────────────────────────────────┤
│                      Network Layer                          │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐    │
│  │   AF_XDP    │  │ Zero-Copy    │  │  NUMA-aware      │    │
│  │   Socket    │  │   Buffer     │  │   Allocation     │    │
│  └─────────────┘  └──────────────┘  └──────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Data Structure (40-byte OHLCV)

```rust
#[repr(C, packed)]
pub struct OHLCV {
    pub ts: u64,        // 8 bytes - epoch nanoseconds
    pub open: u32,      // 4 bytes - price * 100000
    pub high: u32,      // 4 bytes
    pub low: u32,       // 4 bytes
    pub close: u32,     // 4 bytes
    pub volume: u32,    // 4 bytes
    pub symbol_id: u16, // 2 bytes
    pub _pad: [u8; 2],  // 2 bytes - alignment
}
```

#### Design Rationale:
- 40 bytes: Fits within single cache line (64 bytes)
- Fixed precision: 5 decimal places for FX rates
- Symbol ID: Avoids string comparisons in hot path
- Padding: Ensures alignment for SIMD operations

### 2. Block Compression Strategy
Each trading day is stored as a compressed block:

```plaintext
┌─────────────────┐
│   Block Header  │
├─────────────────┤
│  Date: u32      │
│  Symbol: u16    │
│  Size: u32      │
├─────────────────┤
│                 │
│  Compressed     │
│  1440 records   │
│  (1 per minute) │
│                 │
└─────────────────┘
```
#### Compression Pipeline:
1. Collect 1 day of data (1440 minutes)
2. Serialize with bincode
3. Compress with zstd level 3
4. Cache decompressed blocks on access

### 3. Memory Layout
```plaintext
NUMA Node 0 (Ingest)          NUMA Node 1 (Query)
┌────────────────────┐        ┌────────────────────┐
│  Receive Buffers   │        │   Query Cache      │
│  Index Building    │        │   SIMD Filters     │
│  Compression       │        │   Result Buffers   │
└────────────────────┘        └────────────────────┘
         │                              │
         └──────── DashMap ─────────────┘
              (Shared State)
```

### 4. Zero-Copy Data Flow
```plaintext
NIC → DMA → AF_XDP → User Memory → Compressed Block → mmap File
                ↑                          ↓
                └── No kernel transition ──┘
```

## Concurrency Model

### Lock-Free Operations
- DashMap: Sharded HashMap with per-shard RwLocks
- Atomic Counters: For statistics
- Channel-based: Background compression

### Thread Allocation
```
Main Thread:      API handling
Compression (4):  Background block compression
Query (N):        Parallel query execution
Network (2):      AF_XDP RX/TX per NUMA node
```


## File Format Specification

#### Main Data File (.fxd)
```plaintext  
Offset  Size    Description
0       8       Magic: "FXSTORE1"
8       4       Version: 1
12      4       Symbol count
16      8       Block count
24      8       Index offset
32      8       Data offset
40      -       Symbol table
...     -       Compressed blocks
...     -       B+Tree index
```

#### Index Structure
B+Tree with 64KB nodes for cache efficiency:
```rust
#[repr(C, packed)]
struct BPlusNode {
    is_leaf: u8,
    num_keys: u16,
    keys: [u64; 255],     // timestamps
    children: [u64; 256], // offsets or child pointers
}
```

#### Performance Optimizations

1. CPU Optimizations
- CPU Affinity: Pin threads to specific cores
- NUMA Binding: Allocate memory on local node
- Huge Pages: 2MB pages for reduced TLB misses

2. Memory Optimizations
- Prefetching: Explicit cache line prefetch
- False Sharing: 64-byte alignment
- Memory Pools: Pre-allocated buffers

3. Network Optimizations
- Kernel Bypass: AF_XDP sockets
- Batch Processing: Process multiple packets
- RSS: Distribute load across cores

4. Storage Optimizations
- Compression: 10:1 ratio with zstd
- Memory Mapping: Avoid file I/O
- Append-Only: Sequential writes

## Comparison with Original Approach

| Feature      | Databento Style      | FX-Store Optimized    |
|--------------|----------------------|-----------------------|
| Data Size    | 64-byte messages     | 40-byte OHLCV         |
| Compression  | None mentioned       | zstd 10:1             |
| Storage      | Flat binary          | Compressed blocks     |
| Index        | Linear               | B+Tree                |
| Concurrency  | Single-threaded hints| Lock-free DashMap     |
| SIMD         | Not mentioned        | AVX2 filters          |


## Future Enhancements
- GPU Acceleration: CUDA for technical indicators
- RDMA Support: For distributed deployments
- Columnar Format: Better compression for specific queries
- Delta Encoding: Further compression improvements