//! Compact representations for CST nodes
//! Phase 1 optimizations: Varint, Packing, Interning
//! Phase 3 optimizations: Bytecode representation

pub mod bytecode;
pub mod varint;
pub mod tree_builder;
pub mod tree;
pub mod node;
pub mod interning;
pub mod packed_array;

pub use varint::{VarInt, DeltaEncoder, DeltaDecoder, PrefixSumIndex};
pub use tree::CompactTree;
pub use node::CompactNode;
pub use tree_builder::CompactTreeBuilder;
pub use interning::{GlobalInternPool, SymbolId, InternResult, InternConfig, InternStats, INTERN_POOL, intern, resolve, intern_stats};
pub use packed_array::PackedArray;
