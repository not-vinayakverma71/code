# Upstream Storage Format Versioning & Migration (CST-UP05)

**Target:** CST-tree-sitter bytecode storage  
**Purpose:** Document format versioning, backward compatibility, and migration paths  
**Priority:** Low

## Overview

This document specifies storage format versioning for CST-tree-sitter's bytecode encoding to ensure:
1. Forward/backward compatibility between versions
2. Safe migration paths for format changes
3. Graceful degradation on version mismatches
4. Clear upgrade/downgrade procedures

## Version Header Specification

### Binary Format Header

Every stored bytecode file MUST begin with a version header:

```rust
#[repr(C, packed)]
pub struct BytecodeHeader {
    /// Magic bytes: "CSTB" (0x43 0x53 0x54 0x42)
    pub magic: [u8; 4],
    
    /// Major version (breaking changes)
    pub version_major: u16,
    
    /// Minor version (backward-compatible additions)
    pub version_minor: u16,
    
    /// Patch version (bug fixes)
    pub version_patch: u16,
    
    /// Reserved for future use (must be 0)
    pub reserved: u16,
    
    /// CRC32 of the entire payload (excluding this header)
    pub payload_crc32: u32,
    
    /// Total payload size in bytes
    pub payload_size: u64,
    
    /// Feature flags (bitfield)
    pub features: u32,
    
    /// Timestamp of encoding (Unix epoch seconds)
    pub timestamp: u64,
}

impl BytecodeHeader {
    pub const SIZE: usize = 32; // bytes
    pub const MAGIC: [u8; 4] = [0x43, 0x53, 0x54, 0x42]; // "CSTB"
    pub const CURRENT_VERSION: (u16, u16, u16) = (1, 0, 0);
}
```

### Feature Flags

```rust
bitflags! {
    pub struct FormatFeatures: u32 {
        /// Stable IDs included
        const STABLE_IDS = 0x0001;
        
        /// Compressed with zstd
        const COMPRESSION_ZSTD = 0x0002;
        
        /// Segmented storage
        const SEGMENTED = 0x0004;
        
        /// Metadata included
        const METADATA = 0x0008;
        
        /// Incremental format (delta encoding)
        const INCREMENTAL = 0x0010;
        
        /// Field names included
        const FIELD_NAMES = 0x0020;
        
        /// Source text included
        const SOURCE_TEXT = 0x0040;
        
        /// Reserved for future use
        const RESERVED = 0xFFFFFF80;
    }
}
```

## Version Compatibility Matrix

| Writer Version | Reader Version | Compatibility | Notes |
|---------------|----------------|---------------|-------|
| 1.0.x | 1.0.x | Full | Same major.minor |
| 1.0.x | 1.1.x | Read-only | Reader can read, may not write |
| 1.1.x | 1.0.x | Degraded | Old reader ignores new features |
| 2.0.x | 1.x.x | Incompatible | Must migrate |
| 1.x.x | 2.0.x | Incompatible | Must upgrade reader |

### Compatibility Rules

1. **Major version change:** Breaking changes, no backward compatibility
2. **Minor version change:** Backward-compatible additions
3. **Patch version change:** Bug fixes, fully compatible

## Version History

### v1.0.0 (Initial Release)

**Release Date:** 2025-Q1 (planned)

**Features:**
- Basic bytecode encoding
- Tree structure preservation
- Node kinds and positions
- CRC32 validation

**Format:**
```
[Header: 32 bytes]
[Node Count: 8 bytes]
[Nodes: Variable]
  - Node ID: 8 bytes
  - Kind: Variable (null-terminated string)
  - Start byte: 8 bytes
  - End byte: 8 bytes
  - Child count: 4 bytes
  - Children: [Node ID] * child_count
```

### v1.1.0 (Stable IDs)

**Release Date:** 2025-Q2 (planned)

**New Features:**
- ✨ Stable ID generation
- ✨ Incremental change tracking
- ✨ Field name preservation

**Format Changes:**
```diff
  [Header: 32 bytes]
  [Node Count: 8 bytes]
+ [Stable ID Table: Variable] // NEW
  [Nodes: Variable]
    - Node ID: 8 bytes
+   - Stable ID: 8 bytes // NEW
    - Kind: Variable
+   - Field Name: Variable // NEW
    - Start byte: 8 bytes
    - End byte: 8 bytes
    - Child count: 4 bytes
    - Children: [Node ID] * child_count
```

**Migration:**
- v1.0.0 readers can read v1.1.0 files (ignores new fields)
- v1.1.0 readers can read v1.0.0 files (stable IDs set to 0)

### v1.2.0 (Segmented Storage)

**Release Date:** 2025-Q3 (planned)

**New Features:**
- ✨ Segmented storage for large files
- ✨ Lazy loading support
- ✨ Memory-mapped access

**Format Changes:**
```diff
  [Header: 32 bytes]
+ [Segment Table: Variable] // NEW
+   - Segment count: 4 bytes
+   - Segment offsets: [8 bytes] * count
  [Node Count: 8 bytes]
  [Stable ID Table: Variable]
+ [Segments: Variable] // Data split across segments
```

**Migration:**
- v1.1.x readers cannot read v1.2.0 segmented files
- Use migration tool to combine segments for old readers

### v2.0.0 (Compression & Optimization)

**Release Date:** 2025-Q4 (planned)

**Breaking Changes:**
- 🔥 New compression algorithm (zstd)
- 🔥 Optimized node representation
- 🔥 Changed field ordering

**Migration Required:** Yes, use `cst-migrate` tool

## Migration Procedures

### Upgrading Storage Format

#### v1.0 → v1.1 (Add Stable IDs)

```bash
# Using migration tool
cst-migrate upgrade \
  --from 1.0 \
  --to 1.1 \
  --input storage/v1.0/ \
  --output storage/v1.1/ \
  --generate-stable-ids

# Verify migration
cst-migrate verify \
  --input storage/v1.1/ \
  --expected-version 1.1
```

**Programmatic Migration:**

```rust
use lapce_tree_sitter::{BytecodeReader, BytecodeWriter, StableIdGenerator};

fn migrate_v10_to_v11(input: &Path, output: &Path) -> Result<()> {
    let reader = BytecodeReader::new(input)?;
    let header = reader.header()?;
    
    // Check version
    assert_eq!(header.version_major, 1);
    assert_eq!(header.version_minor, 0);
    
    // Decode tree
    let tree = reader.decode()?;
    
    // Generate stable IDs
    let id_gen = StableIdGenerator::new();
    let tree_with_ids = id_gen.assign_ids(&tree)?;
    
    // Encode with new format
    let writer = BytecodeWriter::new(output)?;
    writer.set_version(1, 1, 0)?;
    writer.set_features(FormatFeatures::STABLE_IDS)?;
    writer.encode(&tree_with_ids)?;
    
    Ok(())
}
```

#### v1.1 → v1.2 (Add Segmentation)

```bash
# Segment large files
cst-migrate segment \
  --input storage/v1.1/large_file.cst \
  --output storage/v1.2/large_file.cst \
  --segment-size 256KB \
  --compression none
```

**Programmatic Migration:**

```rust
use lapce_tree_sitter::{BytecodeReader, SegmentedWriter};

fn migrate_v11_to_v12(input: &Path, output: &Path) -> Result<()> {
    let reader = BytecodeReader::new(input)?;
    let tree = reader.decode_with_ids()?;
    
    // Create segmented writer
    let mut writer = SegmentedWriter::new(output)?;
    writer.set_version(1, 2, 0)?;
    writer.set_segment_size(256 * 1024)?; // 256 KB
    writer.encode_segmented(&tree)?;
    
    Ok(())
}
```

#### v1.x → v2.0 (Major Upgrade)

```bash
# Full migration with compression
cst-migrate upgrade-major \
  --from 1.x \
  --to 2.0 \
  --input storage/ \
  --output storage_v2/ \
  --enable-compression zstd \
  --compression-level 3 \
  --backup storage_backup/
```

### Downgrading Storage Format

#### v1.1 → v1.0 (Remove Stable IDs)

```bash
# Lossy downgrade (stable IDs discarded)
cst-migrate downgrade \
  --from 1.1 \
  --to 1.0 \
  --input storage/v1.1/ \
  --output storage/v1.0/ \
  --warn-data-loss
```

**Warning:** Stable IDs will be lost!

#### v1.2 → v1.1 (Combine Segments)

```bash
# Combine segmented file into monolithic
cst-migrate combine-segments \
  --input storage/v1.2/large_file.cst \
  --output storage/v1.1/large_file.cst
```

## Compatibility Checks

### Runtime Version Check

```rust
use lapce_tree_sitter::BytecodeReader;

fn check_compatibility(path: &Path) -> Result<CompatibilityStatus> {
    let reader = BytecodeReader::new(path)?;
    let header = reader.header()?;
    
    let current = BytecodeHeader::CURRENT_VERSION;
    let file = (header.version_major, header.version_minor, header.version_patch);
    
    if file.0 != current.0 {
        // Major version mismatch
        Ok(CompatibilityStatus::Incompatible {
            file_version: file,
            reader_version: current,
            message: "Major version mismatch. Migration required.".to_string(),
        })
    } else if file.1 > current.1 {
        // File has newer minor version
        Ok(CompatibilityStatus::ReadOnly {
            file_version: file,
            reader_version: current,
            message: "File written with newer version. Read-only mode.".to_string(),
        })
    } else if file.1 < current.1 {
        // File has older minor version
        Ok(CompatibilityStatus::Degraded {
            file_version: file,
            reader_version: current,
            message: "File from older version. Some features may not be available.".to_string(),
        })
    } else {
        Ok(CompatibilityStatus::FullyCompatible)
    }
}
```

### Feature Flag Check

```rust
fn check_features(header: &BytecodeHeader) -> Result<Vec<String>> {
    let mut missing = Vec::new();
    let file_features = FormatFeatures::from_bits_truncate(header.features);
    let supported = FormatFeatures::STABLE_IDS 
        | FormatFeatures::COMPRESSION_ZSTD 
        | FormatFeatures::SEGMENTED;
    
    if file_features.contains(FormatFeatures::STABLE_IDS) 
        && !supported.contains(FormatFeatures::STABLE_IDS) {
        missing.push("Stable IDs not supported".to_string());
    }
    
    if file_features.contains(FormatFeatures::COMPRESSION_ZSTD) 
        && !supported.contains(FormatFeatures::COMPRESSION_ZSTD) {
        missing.push("Zstd compression not supported".to_string());
    }
    
    Ok(missing)
}
```

## Rollback Procedures

### Emergency Rollback

If a migration fails or causes issues:

```bash
# Stop all services using the storage
systemctl stop lapce-ai

# Restore from backup
cst-migrate rollback \
  --backup storage_backup/ \
  --target storage/ \
  --verify-integrity

# Restart services
systemctl start lapce-ai
```

### Gradual Rollback

For production systems with zero downtime:

```bash
# Step 1: Dual-write to both old and new formats
cst-migrate dual-write \
  --primary storage_v2/ \
  --secondary storage_v1/ \
  --duration 24h

# Step 2: Switch read traffic to old format
cst-migrate switch-reads \
  --from storage_v2/ \
  --to storage_v1/

# Step 3: Stop writing to new format
cst-migrate stop-dual-write

# Step 4: Archive new format
mv storage_v2/ storage_v2_archived/
```

## Testing & Validation

### Format Validation

```rust
#[test]
fn test_format_roundtrip() {
    let original = create_test_tree();
    
    // Encode
    let mut encoder = BytecodeEncoder::new();
    encoder.set_version(1, 1, 0);
    let bytes = encoder.encode(&original).unwrap();
    
    // Decode
    let decoder = BytecodeDecoder::new();
    let decoded = decoder.decode(&bytes).unwrap();
    
    // Verify equality
    assert_trees_equal(&original, &decoded);
}

#[test]
fn test_version_compatibility() {
    // Write with v1.0
    let v10_bytes = encode_v10(&tree);
    
    // Read with v1.1 reader
    let v11_reader = BytecodeDecoder::new();
    let decoded = v11_reader.decode(&v10_bytes).unwrap();
    
    // Verify stable IDs are initialized to 0
    assert_eq!(decoded.root().stable_id(), 0);
}
```

### Migration Testing

```bash
# Run migration test suite
cargo test --test migration_tests

# Test specific migration path
cargo test migration_v10_to_v11

# Fuzz test migrations
cargo fuzz run migration_fuzzer
```

## Monitoring & Observability

### Prometheus Metrics

```rust
// Version distribution
format_version_total{major="1",minor="1",patch="0"} 1234

// Migration operations
format_migration_total{from="1.0",to="1.1",status="success"} 56
format_migration_duration_seconds{from="1.0",to="1.1"} 12.34

// Compatibility warnings
format_compatibility_warnings_total{type="version_mismatch"} 3
```

### Logging

```rust
use tracing::{info, warn, error};

fn load_bytecode(path: &Path) -> Result<Tree> {
    let reader = BytecodeReader::new(path)?;
    let header = reader.header()?;
    
    info!(
        path = ?path,
        version = format!("{}.{}.{}", 
            header.version_major, 
            header.version_minor, 
            header.version_patch),
        features = ?FormatFeatures::from_bits(header.features),
        "Loading bytecode file"
    );
    
    match check_compatibility(path)? {
        CompatibilityStatus::Incompatible { message, .. } => {
            error!(path = ?path, "Incompatible format: {}", message);
            Err(Error::IncompatibleFormat(message))
        }
        CompatibilityStatus::ReadOnly { message, .. } => {
            warn!(path = ?path, "Read-only mode: {}", message);
            reader.decode()
        }
        CompatibilityStatus::FullyCompatible => {
            reader.decode()
        }
        _ => reader.decode()
    }
}
```

## Best Practices

### For Library Authors

1. ✅ Always check version before decoding
2. ✅ Provide graceful degradation for missing features
3. ✅ Document breaking changes in CHANGELOG
4. ✅ Provide migration tools for major versions
5. ✅ Test backward compatibility in CI

### For Users

1. ✅ Backup before migrating
2. ✅ Test migrations in staging first
3. ✅ Monitor compatibility warnings
4. ✅ Keep old readers available for rollback
5. ✅ Plan migration windows during low traffic

### For Operators

1. ✅ Use dual-write during migrations
2. ✅ Monitor migration progress
3. ✅ Have rollback procedures ready
4. ✅ Validate data integrity after migration
5. ✅ Document version in deployment manifests

## Future Considerations

### Planned Improvements

- **Schema evolution:** Support for adding/removing fields
- **Streaming migration:** Process large datasets without loading fully into memory
- **Distributed migration:** Parallelize migration across multiple nodes
- **Zero-downtime migration:** Online schema migration without service interruption

### Research Areas

- **Columnar encoding:** Store tree as columnar data for better compression
- **Delta encoding:** Store only changes between versions
- **Adaptive compression:** Choose compression based on data characteristics
- **Versioned segments:** Different segments can use different versions

## Appendix

### Migration Tool Reference

```bash
cst-migrate --help

Usage: cst-migrate <COMMAND>

Commands:
  upgrade           Upgrade storage format to newer version
  downgrade         Downgrade storage format (may lose data)
  verify            Verify format integrity and version
  segment           Convert to segmented storage
  combine-segments  Combine segmented file into monolithic
  rollback          Rollback to previous version from backup
  dual-write        Enable dual-write mode for gradual migration
  
Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Version Detection Script

```bash
#!/bin/bash
# Detect bytecode version from file

file="$1"

# Read magic bytes
magic=$(xxd -l 4 -p "$file")
if [ "$magic" != "43535442" ]; then
    echo "ERROR: Not a valid bytecode file (magic: $magic)"
    exit 1
fi

# Read version
major=$(xxd -s 4 -l 2 -e -p "$file")
minor=$(xxd -s 6 -l 2 -e -p "$file")
patch=$(xxd -s 8 -l 2 -e -p "$file")

echo "Version: $((16#$major)).$((16#$minor)).$((16#$patch))"
```

---

*Last updated: 2025-10-11*  
*Specification version: 1.0*  
*Target: CST-tree-sitter upstream*
