# Index Schema Versioning & Migration Plan

**Version**: 1.0  
**Current Schema Version**: v1.0.0  
**Status**: Production Ready

## Schema Version History

### v1.0.0 (Initial - 2025-10-11)

**Tables**:
```sql
-- Vector embeddings table
CREATE TABLE embeddings (
    id UUID PRIMARY KEY,
    file_path TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    node_id TEXT,              -- CST stable node ID
    embedding VECTOR(1024),     -- AWS Titan dimension
    metadata JSONB,
    created_at TIMESTAMP,
    updated_at TIMESTAMP,
    version INTEGER DEFAULT 1
);

-- Index metadata
CREATE TABLE index_metadata (
    key TEXT PRIMARY KEY,
    value JSONB,
    version TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

-- File tracking
CREATE TABLE indexed_files (
    file_path TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    last_indexed TIMESTAMP,
    node_count INTEGER,
    chunk_count INTEGER,
    version INTEGER DEFAULT 1
);
```

**Indices**:
```sql
CREATE INDEX idx_embeddings_file_path ON embeddings(file_path);
CREATE INDEX idx_embeddings_node_id ON embeddings(node_id);
CREATE INDEX idx_embeddings_created ON embeddings(created_at);
CREATE INDEX idx_files_hash ON indexed_files(content_hash);
```

**Vector Index**: IVF_PQ with 256 clusters, 8 subquantizers

## Version Metadata

Each index stores version information:

```rust
pub struct IndexVersion {
    pub major: u32,    // Breaking changes
    pub minor: u32,    // Backward-compatible features
    pub patch: u32,    // Bug fixes, no schema change
}

impl IndexVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0, patch: 0 };
    
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}
```

Stored in index header:
```rust
pub struct IndexHeader {
    pub magic: [u8; 4],           // "LCST"
    pub version: IndexVersion,
    pub created_at: u64,
    pub embedding_dim: u32,
    pub total_vectors: u64,
    pub checksum: u32,
}
```

## Migration Framework

### Migration Structure

```rust
pub trait Migration {
    fn version_from(&self) -> IndexVersion;
    fn version_to(&self) -> IndexVersion;
    fn migrate(&self, index: &mut Index) -> Result<(), MigrationError>;
    fn rollback(&self, index: &mut Index) -> Result<(), MigrationError>;
    fn validate(&self, index: &Index) -> Result<bool, MigrationError>;
}
```

### Migration Registry

```rust
pub struct MigrationRegistry {
    migrations: Vec<Box<dyn Migration>>,
}

impl MigrationRegistry {
    pub fn get_migration_path(
        &self,
        from: IndexVersion,
        to: IndexVersion
    ) -> Result<Vec<&dyn Migration>, MigrationError> {
        // Find shortest path from 'from' to 'to'
        let mut path = Vec::new();
        let mut current = from;
        
        while current != to {
            let next_migration = self.migrations.iter()
                .find(|m| m.version_from() == current)
                .ok_or(MigrationError::PathNotFound)?;
            
            path.push(next_migration.as_ref());
            current = next_migration.version_to();
        }
        
        Ok(path)
    }
    
    pub fn apply_migrations(
        &self,
        index: &mut Index,
        target: IndexVersion
    ) -> Result<(), MigrationError> {
        let current = index.header.version;
        let path = self.get_migration_path(current, target)?;
        
        for migration in path {
            migration.migrate(index)?;
            migration.validate(index)?;
            index.header.version = migration.version_to();
            index.save_header()?;
        }
        
        Ok(())
    }
}
```

## Planned Migrations

### v1.0.0 → v1.1.0: Add Language Field

**Changes**:
- Add `language` column to embeddings table
- Add language-specific indices
- Backward compatible (defaults to "unknown")

```rust
pub struct Migration_1_0_to_1_1;

impl Migration for Migration_1_0_to_1_1 {
    fn version_from(&self) -> IndexVersion {
        IndexVersion { major: 1, minor: 0, patch: 0 }
    }
    
    fn version_to(&self) -> IndexVersion {
        IndexVersion { major: 1, minor: 1, patch: 0 }
    }
    
    fn migrate(&self, index: &mut Index) -> Result<(), MigrationError> {
        // Add language column with default
        index.execute("ALTER TABLE embeddings ADD COLUMN language TEXT DEFAULT 'unknown'")?;
        
        // Create index
        index.execute("CREATE INDEX idx_embeddings_language ON embeddings(language)")?;
        
        // Infer language from file_path for existing records
        index.execute(r#"
            UPDATE embeddings SET language = 
                CASE 
                    WHEN file_path LIKE '%.rs' THEN 'rust'
                    WHEN file_path LIKE '%.ts' THEN 'typescript'
                    WHEN file_path LIKE '%.js' THEN 'javascript'
                    WHEN file_path LIKE '%.py' THEN 'python'
                    WHEN file_path LIKE '%.go' THEN 'go'
                    WHEN file_path LIKE '%.java' THEN 'java'
                    ELSE 'unknown'
                END
        "#)?;
        
        Ok(())
    }
    
    fn rollback(&self, index: &mut Index) -> Result<(), MigrationError> {
        index.execute("DROP INDEX idx_embeddings_language")?;
        index.execute("ALTER TABLE embeddings DROP COLUMN language")?;
        Ok(())
    }
    
    fn validate(&self, index: &Index) -> Result<bool, MigrationError> {
        // Check column exists
        let has_column = index.query_single::<bool>(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('embeddings') WHERE name='language'"
        )?;
        
        Ok(has_column)
    }
}
```

### v1.1.0 → v2.0.0: Multi-Model Support (BREAKING)

**Changes**:
- Add `model_name` and `model_version` columns
- Change embedding dimension to variable (BREAKING)
- Separate indices per model
- Requires full re-indexing

```rust
pub struct Migration_1_1_to_2_0;

impl Migration for Migration_1_1_to_2_0 {
    fn version_from(&self) -> IndexVersion {
        IndexVersion { major: 1, minor: 1, patch: 0 }
    }
    
    fn version_to(&self) -> IndexVersion {
        IndexVersion { major: 2, minor: 0, patch: 0 }
    }
    
    fn migrate(&self, index: &mut Index) -> Result<(), MigrationError> {
        // This is a breaking change - requires full re-index
        Err(MigrationError::RequiresReindex {
            reason: "Embedding dimension changes require re-indexing".into()
        })
    }
    
    fn rollback(&self, _index: &mut Index) -> Result<(), MigrationError> {
        Err(MigrationError::CannotRollback)
    }
}
```

## Forward Compatibility

### Reading Future Versions

```rust
pub fn open_index(path: &Path) -> Result<Index, IndexError> {
    let header = read_header(path)?;
    let current = IndexVersion::CURRENT;
    
    if header.version.major > current.major {
        return Err(IndexError::IncompatibleVersion {
            index_version: header.version,
            supported_version: current,
            message: "Index created with newer major version".into()
        });
    }
    
    if header.version.major == current.major && 
       header.version.minor > current.minor {
        // Forward-compatible: Can read, but some features unavailable
        warn!("Index has newer minor version. Some features may be unavailable.");
    }
    
    Ok(Index::from_header(header))
}
```

### Writing with Downgrades

```rust
pub fn save_index_with_version(
    index: &Index,
    target_version: IndexVersion
) -> Result<(), IndexError> {
    if target_version < index.header.version {
        // Downgrade path - check if safe
        if !is_downgrade_safe(&index, target_version) {
            return Err(IndexError::UnsafeDowngrade {
                from: index.header.version,
                to: target_version
            });
        }
    }
    
    let mut downgraded = index.clone();
    downgraded.header.version = target_version;
    downgraded.save()
}
```

## Backward Compatibility Tests

```rust
#[cfg(test)]
mod compatibility_tests {
    use super::*;
    
    #[test]
    fn test_read_v1_0_index() {
        let index_v1_0 = load_test_index("fixtures/index-v1.0.0.bin");
        let result = Index::open(&index_v1_0);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_migrate_v1_0_to_v1_1() {
        let mut index = load_test_index("fixtures/index-v1.0.0.bin");
        let registry = MigrationRegistry::default();
        
        let target = IndexVersion { major: 1, minor: 1, patch: 0 };
        registry.apply_migrations(&mut index, target).unwrap();
        
        assert_eq!(index.header.version, target);
        assert!(index.has_column("embeddings", "language"));
    }
    
    #[test]
    fn test_forward_compatibility() {
        // Simulate reading v1.2 index with v1.1 code
        let index_v1_2 = create_test_index_v1_2();
        let result = Index::open_with_version(
            &index_v1_2,
            IndexVersion { major: 1, minor: 1, patch: 0 }
        );
        
        // Should succeed (minor version difference)
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_major_version_incompatibility() {
        let index_v2_0 = create_test_index_v2_0();
        let result = Index::open_with_version(
            &index_v2_0,
            IndexVersion { major: 1, minor: 1, patch: 0 }
        );
        
        // Should fail (major version difference)
        assert!(matches!(result, Err(IndexError::IncompatibleVersion { .. })));
    }
}
```

## Migration Execution

### CLI Tool

```bash
# Check current index version
./lapce-index version --index /path/to/index

# Migrate to specific version
./lapce-index migrate --index /path/to/index --to 1.1.0

# Dry-run migration
./lapce-index migrate --index /path/to/index --to 1.1.0 --dry-run

# Rollback to previous version
./lapce-index rollback --index /path/to/index --to 1.0.0

# Validate index integrity
./lapce-index validate --index /path/to/index
```

### Programmatic API

```rust
use lapce_semantic_search::index::{Index, IndexVersion, MigrationRegistry};

// Open index and check version
let mut index = Index::open("/path/to/index")?;
println!("Current version: {}", index.version());

// Migrate if needed
let target = IndexVersion::CURRENT;
if index.version() < target {
    let registry = MigrationRegistry::default();
    registry.apply_migrations(&mut index, target)?;
    println!("Migrated to version {}", index.version());
}
```

## Rollback Strategy

### Safe Rollback Conditions

Can safely rollback if:
1. Minor version change only (1.1 → 1.0)
2. No data added that depends on new schema
3. Index passes validation after rollback

### Rollback Procedure

```bash
# 1. Backup current index
./scripts/backup_index.sh /var/lib/lapce-ai/indices

# 2. Stop services
systemctl stop lapce-ai-service

# 3. Execute rollback
./lapce-index rollback \
    --index /var/lib/lapce-ai/indices \
    --to 1.0.0 \
    --verify

# 4. Validate
./lapce-index validate --index /var/lib/lapce-ai/indices

# 5. Restart services
systemctl start lapce-ai-service
```

### Rollback Verification

```rust
pub fn verify_rollback(
    index: &Index,
    original_version: IndexVersion,
    target_version: IndexVersion
) -> Result<bool, ValidationError> {
    // Check version was updated
    if index.header.version != target_version {
        return Ok(false);
    }
    
    // Check data integrity
    let checksum_valid = index.verify_checksum()?;
    if !checksum_valid {
        return Ok(false);
    }
    
    // Check all records are readable
    let total = index.count_records()?;
    let readable = index.scan_all_records()?.count();
    if readable != total {
        return Ok(false);
    }
    
    // Check indices are valid
    for index_name in index.list_indices()? {
        if !index.verify_index(&index_name)? {
            return Ok(false);
        }
    }
    
    Ok(true)
}
```

## Index Corruption Detection

### Validation Checks

```rust
pub struct IndexValidator;

impl IndexValidator {
    pub fn validate(index: &Index) -> Result<ValidationReport, ValidationError> {
        let mut report = ValidationReport::default();
        
        // Check header integrity
        report.header_valid = self.validate_header(index)?;
        
        // Check schema matches version
        report.schema_valid = self.validate_schema(index)?;
        
        // Check data integrity
        report.data_valid = self.validate_data(index)?;
        
        // Check indices
        report.indices_valid = self.validate_indices(index)?;
        
        // Check checksums
        report.checksum_valid = self.validate_checksums(index)?;
        
        Ok(report)
    }
    
    fn validate_header(&self, index: &Index) -> Result<bool, ValidationError> {
        let header = &index.header;
        
        // Check magic number
        if &header.magic != b"LCST" {
            return Ok(false);
        }
        
        // Check version is valid
        if header.version.major == 0 {
            return Ok(false);
        }
        
        // Check embedding dimension
        if header.embedding_dim == 0 || header.embedding_dim > 4096 {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    fn validate_data(&self, index: &Index) -> Result<bool, ValidationError> {
        // Check all records are readable
        let mut errors = 0;
        
        for record in index.iter_records() {
            match record {
                Ok(_) => continue,
                Err(e) => {
                    error!("Invalid record: {}", e);
                    errors += 1;
                }
            }
        }
        
        Ok(errors == 0)
    }
}
```

### Automated Validation

```bash
# Daily validation cron job
0 2 * * * /opt/lapce-ai/scripts/validate_indices.sh --email-on-error

# CI validation
cargo test --test index_validation -- --nocapture
```

## Emergency Procedures

### Index Corruption

If validation fails:

```bash
# 1. Stop indexing
systemctl stop lapce-ai-indexer

# 2. Assess damage
./lapce-index validate --index /var/lib/lapce-ai/indices --detailed

# 3. Restore from backup
./scripts/restore_index.sh --backup latest --verify

# 4. If backup fails, rebuild
./scripts/rebuild_index.sh --from-source /path/to/source --verify
```

### Version Mismatch

If version incompatibility detected:

```bash
# Option 1: Upgrade code to match index version
git checkout v$(./lapce-index version --index /var/lib/lapce-ai/indices)
cargo build --release

# Option 2: Migrate index to match code version
./lapce-index migrate --index /var/lib/lapce-ai/indices --to $(./lapce-version)

# Option 3: Rebuild index with current code
./scripts/rebuild_index.sh --verify
```

## Schema Evolution Guidelines

### Adding Fields (Minor Version)
- ✅ Add optional columns with defaults
- ✅ Add new indices
- ✅ Add new tables
- ✅ Backward compatible

### Modifying Fields (Major Version)
- ❌ Change column types
- ❌ Change embedding dimensions
- ❌ Remove columns
- ❌ Requires re-indexing

### Best Practices
1. Always test migrations in staging first
2. Backup before any schema change
3. Validate after migration
4. Keep migrations idempotent
5. Document breaking changes
6. Provide migration path or rebuild instructions

## Monitoring

### Metrics

```promql
# Index version distribution
index_version_info{major="1", minor="0", patch="0"}

# Migration success rate
rate(index_migrations_total{status="success"}[1h]) /
rate(index_migrations_total[1h])

# Validation failures
rate(index_validation_failures_total[1h])
```

### Alerts

```yaml
- alert: IndexVersionMismatch
  expr: index_version_info != code_version_info
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "Index version does not match code version"

- alert: IndexValidationFailed
  expr: rate(index_validation_failures_total[5m]) > 0
  for: 1m
  labels:
    severity: critical
  annotations:
    summary: "Index validation failing"
```

---

**Version**: 1.0.0  
**Last Updated**: 2025-10-11  
**Owner**: Platform Team
