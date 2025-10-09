#!/bin/bash
# Final targeted fixes for remaining warnings

cd "$(dirname "$0")"

# Fix dynamic_compressed_cache.rs line 399
sed -i '399s/let tree/let _tree/' src/dynamic_compressed_cache.rs
sed -i '399s/let parse_time/let _parse_time/' src/dynamic_compressed_cache.rs

# Fix compact/bytecode/encoder.rs lines 79-82, 99
sed -i -e '79,82s/let tree =/let _tree =/' \
       -e '79,82s/let node_idx =/let _node_idx =/' \
       -e '79,82s/let kind_set =/let _kind_set =/' \
       -e '79,82s/let field_set =/let _field_set =/' \
       -e '99s/tree: &CompactTree/_tree: \&CompactTree/' \
       -e '99s/node_idx: usize/_node_idx: usize/' \
       src/compact/bytecode/encoder.rs

# Fix compact/bytecode/validator.rs line 103
sed -i '103s/tree: &CompactTree/_tree: \&CompactTree/' src/compact/bytecode/validator.rs
sed -i '103s/node_idx: usize/_node_idx: usize/' src/compact/bytecode/validator.rs  
sed -i '103s/flat_nodes: /_flat_nodes: /' src/compact/bytecode/validator.rs

# Fix compact/bytecode/segmented_fixed.rs
sed -i '346s/storage_dir: PathBuf/_storage_dir: PathBuf/' src/compact/bytecode/segmented_fixed.rs
sed -i '398s/let (segment_id, offset)/let (segment_id, _offset)/' src/compact/bytecode/segmented_fixed.rs

# Fix multi_tier_cache.rs line 323
sed -i '323s/if let Ok((source,/if let Ok((_source,/' src/multi_tier_cache.rs

# Fix phase4_cache.rs
sed -i '211s/if let Some(segmented)/if let Some(_segmented)/' src/phase4_cache.rs
sed -i '268s/hash: u64/_hash: u64/' src/phase4_cache.rs

# Fix phase4_cache_fixed.rs
sed -i '188s/if let Some((bytecode_stream,/if let Some((_bytecode_stream,/' src/phase4_cache_fixed.rs
sed -i '178s/hash: u64/_hash: u64/' src/phase4_cache_fixed.rs

# Fix performance_config.rs line 321
sed -i '321s/let cpu_count =/let _cpu_count =/' src/performance_config.rs

echo "All fixes applied! Running final check..."
cargo build --lib 2>&1 | grep -c "^warning:" | xargs -I {} echo "Remaining warnings: {}"
