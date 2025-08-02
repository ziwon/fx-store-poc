# CPU Cache Optimization for FX-Store

## Overview

This document explains how FX-Store can leverage the Intel Core Ultra 9 285K cache hierarchy for maximum performance. Understanding and optimizing for the cache is crucial for achieving high-throughput financial data processing.

## Current Cache Hierarchy

### Cache Specifications
```
L1d Cache: 1.1 MiB (24 instances) - 48 KB per core
L1i Cache: 1.5 MiB (24 instances) - 64 KB per core  
L2 Cache:  72 MiB (24 instances)  - 3 MB per core
L3 Cache:  36 MiB (1 instance)    - Shared across all cores
```

### Cache Line Size
- **Cache Line**: 64 bytes (typical for x86_64)
- **Pages**: 4 KB (standard), 2 MB (huge pages)
- **Memory Access**: ~300 cycles for main memory vs ~4 cycles for L1

## Cache-Friendly Data Structures

### 1. OHLCV Record Optimization

```rust
// Current structure (40 bytes) - fits in single cache line
#[repr(C, packed)]
pub struct OHLCV {
    pub ts: u64,        // 8 bytes - timestamp
    pub open: u32,      // 4 bytes - price * 100000
    pub high: u32,      // 4 bytes
    pub low: u32,       // 4 bytes  
    pub close: u32,     // 4 bytes
    pub volume: u32,    // 4 bytes
    pub symbol_id: u16, // 2 bytes
    pub _pad: [u8; 14], // 14 bytes - pad to 64 bytes (cache line)
}

// Benefits:
// - Single cache line access
// - No false sharing between records
// - SIMD-friendly alignment
```

### 2. Block Structure for Cache Efficiency

```rust
// Organize data in cache-friendly blocks
const RECORDS_PER_BLOCK: usize = 1440; // 1 day of minute data
const BLOCK_SIZE: usize = RECORDS_PER_BLOCK * 64; // 92,160 bytes

#[repr(C, align(64))] // Cache line aligned
pub struct DataBlock {
    header: BlockHeader,                    // 64 bytes
    records: [OHLCV; RECORDS_PER_BLOCK],  // 1440 * 64 bytes
}

// Benefits:
// - Sequential access pattern
// - Prefetcher-friendly
// - Efficient compression unit
```

### 3. Index Structure Optimization

```rust
// B+Tree nodes sized for cache efficiency
const NODE_SIZE: usize = 4096; // 4KB - fits in L1d cache
const KEYS_PER_NODE: usize = (NODE_SIZE - 64) / 16; // ~250 keys

#[repr(C, align(64))]
pub struct BTreeNode {
    header: NodeHeader,           // 64 bytes
    keys: [u64; KEYS_PER_NODE],  // Timestamps
    values: [u64; KEYS_PER_NODE + 1], // Child pointers or data offsets
}

// Benefits:
// - Entire node fits in L1 cache
// - High branching factor
// - Cache-friendly traversal
```

## Memory Layout Strategies

### 1. Spatial Locality Optimization

```rust
// Group frequently accessed data together
#[repr(C)]
pub struct SymbolData {
    // Hot data (frequently accessed) - first cache line
    symbol_id: u16,
    block_count: u32,
    last_timestamp: u64,
    stats: QueryStats,           // 48 bytes total
    _pad1: [u8; 16],            // Pad to 64 bytes
    
    // Warm data - second cache line  
    metadata: SymbolMetadata,    // 64 bytes
    
    // Cold data - separate cache lines
    block_offsets: Vec<u64>,     // Heap allocated
}
```

### 2. False Sharing Prevention

```rust
// Align per-thread data to cache line boundaries
#[repr(C, align(64))]
pub struct ThreadLocalStats {
    queries_processed: AtomicU64,
    bytes_processed: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    _pad: [u8; 32], // Ensure 64-byte alignment
}

// Benefits:
// - No false sharing between threads
// - Each thread owns full cache line
// - Better scalability on multi-core
```

### 3. Array of Structures vs Structure of Arrays

```rust
// AoS - Good for accessing all fields of one record
pub struct RecordsAoS {
    records: Vec<OHLCV>, // All fields together
}

// SoA - Good for accessing one field across many records
pub struct RecordsSoA {
    timestamps: Vec<u64>,
    opens: Vec<u32>,
    highs: Vec<u32>,
    lows: Vec<u32>,
    closes: Vec<u32>,
    volumes: Vec<u32>,
}

// Use AoS for: Complete record queries
// Use SoA for: Aggregations, filtering, SIMD operations
```

## Cache Access Patterns

### 1. Sequential Access Optimization

```rust
// Optimize for sequential processing
impl DataBlock {
    // Good: Sequential iteration
    pub fn process_sequential(&self) -> Statistics {
        let mut stats = Statistics::default();
        
        // Prefetch next cache line while processing current
        for i in 0..self.records.len() {
            if i + 16 < self.records.len() {
                // Prefetch 16 records ahead (1 cache line)
                unsafe {
                    std::arch::x86_64::_mm_prefetch(
                        (&self.records[i + 16] as *const OHLCV) as *const i8,
                        std::arch::x86_64::_MM_HINT_T0
                    );
                }
            }
            
            stats.update(&self.records[i]);
        }
        stats
    }
}
```

### 2. Random Access Optimization

```rust
// For random access, use cache-friendly data structures
pub struct TimestampIndex {
    // Hot path: Keep frequently accessed data in L1
    root_node: *const BTreeNode,
    cache: LruCache<u64, *const BTreeNode>, // 32 entries, fits in L2
}

impl TimestampIndex {
    pub fn search(&self, timestamp: u64) -> Option<u64> {
        // Check L2 cache first
        if let Some(&node_ptr) = self.cache.get(&timestamp) {
            return self.search_node(unsafe { &*node_ptr }, timestamp);
        }
        
        // Traverse from root with prefetching
        self.search_with_prefetch(timestamp)
    }
}
```

## SIMD and Cache Optimization

### 1. Vectorized Operations with Cache Awareness

```rust
use std::arch::x86_64::*;

impl DataBlock {
    // Process 8 records at once (AVX2)
    pub unsafe fn filter_prices_avx2(&self, min_price: f32, max_price: f32) -> Vec<usize> {
        let mut results = Vec::new();
        let min_vec = _mm256_set1_ps(min_price);
        let max_vec = _mm256_set1_ps(max_price);
        
        // Process in chunks that fit in L1 cache
        const CHUNK_SIZE: usize = 64; // ~4KB per chunk
        
        for chunk_start in (0..self.records.len()).step_by(CHUNK_SIZE) {
            let chunk_end = (chunk_start + CHUNK_SIZE).min(self.records.len());
            
            // Prefetch next chunk
            if chunk_end < self.records.len() {
                _mm_prefetch(
                    (&self.records[chunk_end] as *const OHLCV) as *const i8,
                    _MM_HINT_T0
                );
            }
            
            // Process current chunk
            for i in (chunk_start..chunk_end).step_by(8) {
                // Load 8 close prices
                let prices = _mm256_loadu_ps(
                    &self.records[i].close as *const u32 as *const f32
                );
                
                // Compare with bounds
                let gt_min = _mm256_cmp_ps(prices, min_vec, _CMP_GE_OQ);
                let lt_max = _mm256_cmp_ps(prices, max_vec, _CMP_LE_OQ);
                let mask = _mm256_and_ps(gt_min, lt_max);
                
                // Extract results
                let result_mask = _mm256_movemask_ps(mask);
                for bit in 0..8 {
                    if (result_mask & (1 << bit)) != 0 {
                        results.push(i + bit);
                    }
                }
            }
        }
        results
    }
}
```

### 2. Cache-Aware Batch Processing

```rust
pub struct QueryProcessor {
    // Size batch to fit in L2 cache (3MB per core)
    batch_size: usize, // ~400KB worth of data
}

impl QueryProcessor {
    pub fn process_range_query(&self, start: u64, end: u64) -> Vec<OHLCV> {
        let mut results = Vec::new();
        
        // Process in cache-sized batches
        for batch in self.get_batches_in_range(start, end) {
            // Entire batch fits in L2 cache
            let filtered = self.process_batch_in_cache(batch);
            results.extend(filtered);
        }
        
        results
    }
    
    fn process_batch_in_cache(&self, batch: &[OHLCV]) -> Vec<OHLCV> {
        // All operations on this batch will hit L2 cache
        batch.iter()
            .filter(|record| self.matches_criteria(record))
            .cloned()
            .collect()
    }
}
```

## Memory Prefetching Strategies

### 1. Hardware Prefetcher Optimization

```rust
// Stride patterns that work well with Intel prefetcher
impl DataIterator {
    // Good: Regular stride (prefetcher detects pattern)
    pub fn iterate_every_nth(&self, n: usize) -> impl Iterator<Item = &OHLCV> {
        self.records.iter().step_by(n)
    }
    
    // Good: Sequential access (prefetcher friendly)
    pub fn iterate_range(&self, start: usize, end: usize) -> &[OHLCV] {
        &self.records[start..end]
    }
}
```

### 2. Software Prefetching

```rust
use std::arch::x86_64::_mm_prefetch;

impl TimeSeries {
    pub fn calculate_moving_average(&self, window: usize) -> Vec<f64> {
        let mut averages = Vec::with_capacity(self.data.len() - window + 1);
        
        for i in 0..=(self.data.len() - window) {
            // Prefetch data we'll need in next iteration
            if i + window + 64 < self.data.len() {
                unsafe {
                    _mm_prefetch(
                        (&self.data[i + window + 64] as *const OHLCV) as *const i8,
                        _MM_HINT_T1 // Load into L2 cache
                    );
                }
            }
            
            // Calculate average for current window
            let sum: u64 = self.data[i..i + window]
                .iter()
                .map(|r| r.close as u64)
                .sum();
                
            averages.push(sum as f64 / window as f64 / 100000.0);
        }
        
        averages
    }
}
```

## Cache Performance Monitoring

### 1. Cache Miss Profiling

```rust
use std::time::Instant;

pub struct CacheProfiler {
    pub l1_misses: u64,
    pub l2_misses: u64,
    pub l3_misses: u64,
}

impl CacheProfiler {
    pub fn profile_operation<F, R>(&mut self, name: &str, op: F) -> R 
    where F: FnOnce() -> R 
    {
        // Use perf counters if available
        let start = Instant::now();
        let result = op();
        let duration = start.elapsed();
        
        println!("Operation {}: {:?}", name, duration);
        result
    }
}

// Usage
let mut profiler = CacheProfiler::new();
let results = profiler.profile_operation("range_query", || {
    query_processor.execute_range_query(start, end)
});
```

### 2. Cache-Friendly Testing

```bash
# Use perf to measure cache performance
perf stat -e cache-references,cache-misses,L1-dcache-loads,L1-dcache-load-misses \
    ./target/release/fx-store bench --operation query

# Look for:
# - L1 miss rate < 5%
# - L2 miss rate < 20%  
# - Cache references/instructions ratio
```

## Best Practices Summary

### Data Structure Design
1. **Align to cache lines** (64 bytes)
2. **Size structures** to fit cache levels
3. **Group hot data** together
4. **Separate read/write data** to avoid false sharing

### Access Patterns
1. **Prefer sequential access** over random
2. **Use blocking/tiling** for large datasets
3. **Batch operations** to stay in cache
4. **Prefetch predictable patterns**

### SIMD Integration
1. **Align data for SIMD** (16/32 byte boundaries)
2. **Process in SIMD-width chunks**
3. **Combine with cache blocking**
4. **Use appropriate SIMD instructions** (AVX2 available)

### Memory Management
1. **Use huge pages** for large allocations
2. **Pool allocations** to reduce fragmentation
3. **NUMA-aware allocation** (when available)
4. **Monitor working set size**

## Implementation Priorities

### Phase 1: Core Structures
- Optimize OHLCV record layout
- Implement cache-aligned blocks
- Add software prefetching

### Phase 2: Query Engine  
- Cache-aware B+Tree implementation
- Batch processing optimization
- SIMD filtering operations

### Phase 3: Advanced Optimizations
- Working set analysis
- Cache miss profiling
- Adaptive prefetching strategies

The Intel Core Ultra 9 285K's cache hierarchy provides excellent opportunities for optimization. With proper data structure design and access patterns, FX-Store can achieve significant performance improvements by keeping hot data in the faster cache levels.