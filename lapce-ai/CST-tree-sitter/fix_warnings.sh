#!/bin/bash
# Systematic fix for all compiler warnings

cd "$(dirname "$0")"

echo "Fixing all warnings systematically..."

# 1. Fix unused imports in test files
echo "1. Fixing test file imports..."
sed -i 's/use lapce_tree_sitter::compact::bytecode::{TreeSitterBytecodeEncoder, SegmentedBytecodeStream};/use lapce_tree_sitter::compact::bytecode::TreeSitterBytecodeEncoder;/' tests/cross_platform_determinism.rs
sed -i 's/use tree_sitter::{Parser, InputEdit, Point};/use tree_sitter::Parser;/' tests/stable_id_incremental.rs
sed -i 's/    intern, resolve, INTERN_POOL, intern_stats,/    intern, intern_stats,/' tests/phase1_optimization_tests.rs

# 2. Fix unused macros in test files
echo "2. Fixing unused test macros..."
cat > /tmp/lang_smoke_fix.patch << 'EOF'
@@ -25,6 +25,7 @@
 
     // Generate test functions for each core language
     // For each language, create a smoke test function
+    #[allow(unused_macros)]
     macro_rules! smoke_test_fn {
         ($name:ident, $lang_name:expr, $ext:expr) => {
             #[test]
@@ -43,6 +44,7 @@
     }
     
     // Macro for feature-gated languages
+    #[allow(unused_macros)]
     macro_rules! smoke_test_fn_gated {
         ($name:ident, $lang_name:expr, $ext:expr, $feature:expr) => {
             #[cfg(feature = $feature)]
EOF

# 3. Fix bin files with unused imports
echo "3. Fixing bin file imports..."
sed -i 's/use std::time::{Duration, Instant};/use std::time::Duration;/' src/bin/test_multi_tier.rs
sed -i 's/use tree_sitter::{Parser, Language};/use tree_sitter::Parser;/' src/bin/test_multi_tier.rs
sed -i 's/use lapce_tree_sitter::phase4_cache_fixed::{Phase4Cache, Phase4Config};//' src/bin/test_tier_transitions_debug.rs
sed -i 's/use std::io::{Read, Write};/use std::io::Read;/' src/bin/migrate_cache.rs
sed -i 's/use lapce_tree_sitter::compact::bytecode::{SegmentedBytecodeStream, TreeSitterBytecodeEncoder};//' src/bin/migrate_cache.rs
sed -i 's/use parking_lot::RwLock;//' src/bin/benchmark_codex_extensive.rs
sed -i 's/use std::collections::HashMap;//' src/bin/benchmark_codex_complete.rs
sed -i 's/    SegmentedBytecodeStream,//' src/bin/benchmark_codex_complete.rs
sed -i 's/use bytes::Bytes;//' src/bin/benchmark_codex_complete.rs
sed -i 's/use std::time::Instant;//' src/bin/benchmark_all_phases.rs
sed -i 's/use std::collections::HashMap;//' src/bin/benchmark_all_phases.rs
sed -i 's/    StorageLocation,//' src/bin/benchmark_all_phases.rs
sed -i 's/use bytes::Bytes;//' src/bin/test_promotions.rs
sed -i 's/use std::sync::Arc;//' src/bin/test_promotions.rs
sed -i 's/use std::time::{Duration, Instant};/use std::time::Duration;/' src/bin/test_promotions.rs
sed -i 's/use std::path::{Path, PathBuf};/use std::path::PathBuf;/' src/bin/test_crash_recovery.rs
sed -i 's/use std::process::{Command, Stdio};//' src/bin/test_crash_recovery.rs
sed -i 's/use tree_sitter::{Parser, Language};/use tree_sitter::Parser;/' src/bin/test_all_phases.rs

# 4. Prefix unused variables with underscore
echo "4. Fixing unused variables..."

# Create a sed script for all unused variable fixes
cat > /tmp/fix_vars.sed << 'EOF'
# test_crash_recovery.rs
s/Ok(Some((tree, source)))/Ok(Some((_tree, source)))/

# benchmark_performance.rs
s/let mut system = System::new_all();/let mut _system = System::new_all();/

# benchmark_codex_extensive.rs
s/let file_start = Instant::now();/let _file_start = Instant::now();/
s/let parse_time = parse_start.elapsed();/let _parse_time = parse_start.elapsed();/
s/for iteration in 0..stress_iterations {/for _iteration in 0..stress_iterations {/
s/if let Some((language, lang_name))/if let Some((_language, lang_name))/
s/let criteria_memory =/let _criteria_memory =/

# benchmark_codex_complete.rs
s/let round_start = Instant::now();/let _round_start = Instant::now();/

# test_promotions.rs
s/let segmented = SegmentedBytecodeStream/let _segmented = SegmentedBytecodeStream/

# phase1_optimization_tests.rs
s/let code = r#"/let _code = r#"/
s/let positions_per_symbol = 5;/let _positions_per_symbol = 5;/
EOF

find src/bin tests -name "*.rs" -exec sed -i -f /tmp/fix_vars.sed {} \;

# 5. Fix dead code in structs
echo "5. Adding allow attributes for intentional dead code..."

# For struct fields that are never read but part of API
find src -name "*.rs" -exec sed -i 's/^pub struct EntryMetadata {/#[allow(dead_code)]\npub struct EntryMetadata {/' {} \;
find src -name "*.rs" -exec sed -i 's/^struct MemorySnapshot {/#[allow(dead_code)]\nstruct MemorySnapshot {/' {} \;
find src -name "*.rs" -exec sed -i 's/^struct SuccessCriteria {/#[allow(dead_code)]\nstruct SuccessCriteria {/' {} \;
find src -name "*.rs" -exec sed -i 's/^struct BenchmarkResults {/#[allow(dead_code)]\nstruct BenchmarkResults {/' {} \;
find src -name "*.rs" -exec sed -i 's/^struct TestResult {/#[allow(dead_code)]\nstruct TestResult {/' {} \;
find src -name "*.rs" -exec sed -i 's/^struct AccessInfo {/#[allow(dead_code)]\nstruct AccessInfo {/' {} \;
find src -name "*.rs" -exec sed -i 's/^enum CacheTier {/#[allow(dead_code)]\nenum CacheTier {/' {} \;

echo "Done! Now run: cargo build --release -q to verify fixes"
