# AWS Embedder Test Fixes Required

## Issue
Tests are using incorrect `AwsTitanProduction` constructor signatures.

## Actual API (from `src/embeddings/aws_titan_production.rs`)

```rust
impl AwsTitanProduction {
    // NOT async, requires BedrockClient
    pub fn new(
        client: BedrockClient,
        tier: AwsTier,
        max_batch_size: usize,
        requests_per_second: f64,
    ) -> Self { ... }
    
    // Async, loads from environment
    pub async fn new_from_config() -> Result<Self> { ... }
}
```

## Test Files with Incorrect Usage

### Pattern 1: `.await` on non-async `new()`
**Affected files:**
- `tests/aws_config_hardening_tests.rs` (multiple occurrences)
- `tests/aws_security_rate_limit_tests.rs` (multiple occurrences)

**Incorrect:**
```rust
let embedder = AwsTitanProduction::new().await;
```

**Fix:**
```rust
let embedder = AwsTitanProduction::new_from_config().await;
```

### Pattern 2: Passing region string and tier (wrong signature)
**Affected files:**
- `tests/real_aws_no_mocks.rs`
- `tests/final_optimized_test.rs`
- `tests/full_system_production_test.rs`
- `tests/optimized_aws_test.rs`
- `tests/aws_titan_full_performance_test.rs`

**Incorrect:**
```rust
let embedder = AwsTitanProduction::new("us-east-1", AwsTier::Standard).await
```

**Fix:**
```rust
// Use new_from_config which loads region from env
let embedder = AwsTitanProduction::new_from_config().await
```

## Migration Steps

1. **For all test files**: Replace incorrect constructors with `new_from_config().await`
2. **Ensure AWS env vars are set** before running tests:
   ```bash
   export AWS_ACCESS_KEY_ID=your_key
   export AWS_SECRET_ACCESS_KEY=your_secret
   export AWS_DEFAULT_REGION=us-east-1
   ```
3. **Update test documentation** to mention AWS credentials requirement

## Files to Update (Priority Order)

### High Priority (Real AWS Integration Tests)
1. `tests/real_aws_no_mocks.rs`
2. `tests/final_optimized_test.rs`
3. `tests/aws_titan_full_performance_test.rs`
4. `tests/optimized_aws_test.rs`
5. `tests/full_system_production_test.rs`

### Medium Priority (Config/Security Tests)
6. `tests/aws_config_hardening_tests.rs` (15+ occurrences)
7. `tests/aws_security_rate_limit_tests.rs` (15+ occurrences)

## Automated Fix Command

```bash
# Replace pattern 1: new().await -> new_from_config().await
find tests -name "*.rs" -type f -exec sed -i 's/AwsTitanProduction::new().await/AwsTitanProduction::new_from_config().await/g' {} +

# Pattern 2 requires manual review due to .expect() and other variations
```

## Testing After Fixes

```bash
# Ensure AWS creds are set
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_DEFAULT_REGION=us-east-1

# Test individual files
cargo test --test real_aws_no_mocks -- --nocapture
cargo test --test aws_config_hardening_tests -- --nocapture

# Full test suite
cargo test --features aws_integration
```

## Notes

- Tests requiring AWS will fail without valid credentials (expected behavior)
- Some tests intentionally test invalid configs - these should handle errors gracefully
- The `new_from_config()` method already handles region loading from environment
