#!/bin/bash
# Fix all remaining Rust warnings systematically

cd "$(dirname "$0")"

echo "Fixing remaining warnings..."

# Fix unused variables by prefixing with underscore
find src -name "*.rs" -type f -exec sed -i \
  -e 's/let tree_data =/let _tree_data =/' \
  -e 's/let parse_time =/let _parse_time =/' \
  -e 's/let tree =/let _tree =/' \
  -e 's/let node_idx =/let _node_idx =/' \
  -e 's/let kind_set =/let _kind_set =/' \
  -e 's/let field_set =/let _field_set =/' \
  -e 's/let start_pos =/let _start_pos =/' \
  -e 's/let flat_nodes =/let _flat_nodes =/' \
  -e 's/let encoder2 =/let _encoder2 =/' \
  -e 's/let temp_path =/let _temp_path =/' \
  -e 's/let storage_dir =/let _storage_dir =/' \
  -e 's/let offset =/let _offset =/' \
  -e 's/let kind =/let _kind =/' \
  -e 's/let flags =/let _flags =/' \
  -e 's/let length =/let _length =/' \
  -e 's/for i in/for _i in/' \
  -e 's/let count =/let _count =/' \
  -e 's/let source =/let _source =/' \
  -e 's/let segmented =/let _segmented =/' \
  {} +

# Fix function parameters
find src -name "*.rs" -type f -exec sed -i \
  -e 's/fn get_id(&self, s: /fn get_id(\&self, _s: /' \
  -e 's/fn resolve(&self, id: /fn resolve(\&self, _id: /' \
  -e 's/(tree: \&CompactTree, node_idx: /(_tree: \&CompactTree, _node_idx: /' \
  {} +

# Remove unused imports
sed -i '/use std::io::Read;/d' src/bin/migrate_cache.rs
sed -i '/use std::fs::File;/s/File, //' src/phase4_cache.rs 2>/dev/null || true

echo "Done! Verifying..."
cargo build --lib -q 2>&1 | grep -c "warning:" || echo "0 warnings!"
