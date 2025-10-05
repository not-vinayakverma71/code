//! Succinct CST implementation for 10-20x memory reduction
//! Lossless compression while maintaining O(1) operations

pub mod bp;
pub mod rank_select;
pub mod bitvec;
pub mod optimized_tree;
pub mod bytecode;
pub mod varint;
pub mod tree_builder;
pub mod tree;
pub mod node;
pub mod incremental;
pub mod query_engine;
pub mod production;
pub mod interning;

pub use bitvec::BitVec;
pub use rank_select::RankSelect;
pub use bp::BP;
pub use varint::{VarInt, DeltaEncoder, DeltaDecoder, PrefixSumIndex};
pub use tree::CompactTree;
pub use node::CompactNode;
pub use tree_builder::CompactTreeBuilder;
pub use incremental::{IncrementalCompactTree, Edit, ParseMetrics, MicrotreeCache};
pub use query_engine::{CompactQueryEngine, SuccinctQueryOps, SymbolIndex};
pub use production::{ProductionTreeBuilder, CompactMetrics, HealthMonitor, Profiler, METRICS, PROFILER};
pub use interning::{GlobalInternPool, SymbolId, InternResult, InternConfig, InternStats, INTERN_POOL, intern, resolve, intern_stats};
pub use optimized_tree::{OptimizedCompactTree, OptimizedTreeBuilder};
