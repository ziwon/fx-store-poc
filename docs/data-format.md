# Data Format Specification

## Overview

FX-Store uses a custom binary format optimized for:
- **Fast sequential reads**: Memory-mapped files
- **Efficient compression**: 10:1 ratio with zstd
- **Quick random access**: B+Tree indexing
- **Cache efficiency**: 64-byte aligned structures

## File Structure

### Main Data File (.fxd)
```plaintext
┌─────────────────────────────────┐ 0x0000
│         File Header             │
├─────────────────────────────────┤ 0x0040
│        Symbol Table             │
├─────────────────────────────────┤ Variable
│      Compressed Blocks          │
├─────────────────────────────────┤ Variable
│        B+Tree Index             │
├─────────────────────────────────┤ Variable
│         Metadata                │
└─────────────────────────────────┘ EOF
```

### File Header (64 bytes)

```c
struct FileHeader {
    uint64_t magic;           // 0x00: "FXSTORE1" (0x4658535430524531)
    uint32_t version;         // 0x08: Format version (currently 1)
    uint32_t flags;           // 0x0C: Feature flags
    uint64_t created_ts;      // 0x10: Creation timestamp (nanos)
    uint64_t modified_ts;     // 0x18: Last modified (nanos)
    uint32_t symbol_count;    // 0x20: Number of symbols
    uint32_t block_count;     // 0x24: Total blocks
    uint64_t index_offset;    // 0x28: Offset to B+Tree index
    uint64_t symbol_offset;   // 0x30: Offset to symbol table
    uint64_t data_offset;     // 0x38: Offset to first block
}
```

### Symbol Table Entry (64 bytes)
```c
struct SymbolEntry {
    uint16_t id;              // 0x00: Symbol ID
    uint16_t flags;           // 0x02: Symbol flags
    uint32_t block_count;     // 0x04: Blocks for this symbol
    char name[32];            // 0x08: Symbol name (null-terminated)
    uint64_t first_ts;        // 0x28: First timestamp
    uint64_t last_ts;         // 0x30: Last timestamp
    uint64_t record_count;    // 0x38: Total records
}
```

### Compressed Block Header (32 bytes)
```c
struct BlockHeader {
    uint32_t magic;           // 0x00: Block magic (0x424C4F43)
    uint16_t symbol_id;       // 0x04: Symbol ID
    uint16_t compression;     // 0x06: Compression type
    uint32_t date;            // 0x08: Date (YYYYMMDD)
    uint32_t compressed_size; // 0x0C: Compressed data size
    uint32_t original_size;   // 0x10: Uncompressed size
    uint64_t first_ts;        // 0x14: First record timestamp
    uint64_t last_ts;         // 0x1C: Last record timestamp
}
```

### OHLCV Record (40 bytes)
```c
struct OHLCV {
    uint64_t ts;              // 0x00: Timestamp (epoch nanos)
    uint32_t open;            // 0x08: Open price * 100000
    uint32_t high;            // 0x0C: High price * 100000
    uint32_t low;             // 0x10: Low price * 100000
    uint32_t close;           // 0x14: Close price * 100000
    uint32_t volume;          // 0x18: Volume
    uint16_t symbol_id;       // 0x1C: Symbol ID
    uint8_t flags;            // 0x1E: Record flags
    uint8_t _pad;             // 0x1F: Padding
}
``` 

### B+Tree Index Node (64KB)
```c
struct IndexNode {
    uint8_t is_leaf;          // 0x00: Node type (0=internal, 1=leaf)
    uint8_t level;            // 0x01: Tree level
    uint16_t num_keys;        // 0x02: Number of keys
    uint32_t _pad;            // 0x04: Padding
    uint64_t keys[255];       // 0x08: Timestamps
    uint64_t values[256];     // 0x808: Offsets or child pointers
}
``` 

## Compression Scheme

### Block Compression
1. **Collection**: Gather 1440 OHLCV records (1 day @ 1min intervals)
2. **Serialization**: Convert to binary using bincode
3. **Compression**: Apply zstd level 3
4. **Storage**: Write compressed block with header

### Compression Dictionary
For better compression ratio, a shared dictionary is trained:

```rust
// Dictionary training
let samples: Vec<Vec<u8>> = historical_blocks
    .iter()
    .map(|b| bincode::serialize(b).unwrap())
    .collect();

let dict = zstd::dict::from_samples(&samples, 64 * 1024)?;
```

### Binary Encoding
#### Price Encoding
Prices are stored as fixed-point integers:
- Multiplier: 100,000 (5 decimal places)
- Range: 0.00001 to 42,949.67295
- Example: 1.23456 → 123456

#### Timestamp Encoding
- Format: Nanoseconds since Unix epoch
- Range: 1970-01-01 to 2262-04-11
- Precision: 1 nanosecond

#### Flags Bitmap
```plaintext
Bit 7 6 5 4 3 2 1 0
    │ │ │ │ │ │ │ └─ Valid
    │ │ │ │ │ │ └─── Gap (missing data)
    │ │ │ │ │ └───── Interpolated
    │ │ │ │ └─────── Weekend
    │ │ │ └───────── Holiday
    │ │ └─────────── Adjusted
    │ └───────────── Error
    └─────────────── Reserved
```

### Index Structure

#### Primary Index (Time-based)
B+Tree with 255-way branching:
- Leaf nodes: Contain (timestamp, file_offset) pairs
- Internal nodes: Contain (min_timestamp, child_offset) pairs
- Height: Typically 3-4 levels for billions of records

#### Secondary Index (Symbol-based)
Hash table mapping symbol_id to block offsets:

```
symbol_id → [block_offset_1, block_offset_2, ...]
```

### Query Process
#### Symbol Resolution
```  
symbol_name → symbol_id (via symbol table)
```

#### Block Location
```
symbol_id + date_range → block_offsets (via index)
```

#### Block Loading
``` 
block_offset → mmap → decompress → OHLCV[]
```

#### Filtering
```
OHLCV[] → SIMD filter → result[]
```

### File Operations
Append Operation
```rust
// 1. Serialize new records
let serialized = bincode::serialize(&records)?;

// 2. Compress
let compressed = zstd::encode_all(&serialized[..], 3)?;

// 3. Write block header
file.write_all(&block_header.to_bytes())?;

// 4. Write compressed data
file.write_all(&compressed)?;

// 5. Update index
index.insert(first_timestamp, file_offset);
Read Operation
rust// 1. Query index
let offset = index.search(timestamp)?;

// 2. Memory map region
let mmap = MmapOptions::new()
    .offset(offset)
    .len(block_size)
    .map(&file)?;

// 3. Read header
let header = BlockHeader::from_bytes(&mmap[..32]);

// 4. Decompress
let data = zstd::decode_all(&mmap[32..32+header.compressed_size])?;

// 5. Deserialize
let records: Vec<OHLCV> = bincode::deserialize(&data)?;
```

### Version Compatibility
#### Version 1 Features
- Basic OHLCV storage
- zstd compression
- B+Tree indexing

#### Future Versions (Reserved)
- Version 2: Delta encoding
- Version 3: Columnar storage
- Version 4: Multi-resolution storage

#### Backward Compatibility
Files include version in header. Reader must support:
```rust
match header.version {
    1 => read_v1(file),
    2 => read_v2(file), // With delta encoding
    _ => Err(Error::UnsupportedVersion),
}
``` 

### Best Practices
- Alignment: Keep structures aligned to cache lines (64 bytes)
- Compression: Use level 3 for speed/ratio balance
- Block Size: 1 day (1440 minutes) optimal for FX data
- Indexing: Rebuild index periodically for optimal performance
- Validation: Verify checksums on critical operations

### Tools
#### Format Inspector
```bash
# Dump file header
fx-store inspect --file data.fxd --header

# Show index structure
fx-store inspect --file data.fxd --index

# Verify integrity
fx-store verify --file data.fxd --checksums
```

#### Format Converter
```bash
# Convert from CSV
fx-store convert --from csv --to fxd input.csv output.fxd

# Export to Parquet
fx-store convert --from fxd --to parquet data.fxd output.parquet
```