//! Bytecode-based tree representation for ultimate memory efficiency

pub mod encoder;
pub mod decoder;
pub mod navigator;
pub mod validator;
pub mod segmented_fixed;
pub mod tree_sitter_encoder;
pub mod opcodes;

pub use encoder::BytecodeEncoder;
pub use decoder::BytecodeDecoder;
pub use navigator::BytecodeNavigator;
pub use validator::BytecodeValidator;
pub use segmented_fixed::{SegmentedBytecodeStream, SegmentedNavigator, SegmentStatsSnapshot};
pub use tree_sitter_encoder::{TreeSitterBytecodeEncoder, TreeSitterBytecodeDecoder};
pub use opcodes::{Opcode, NodeFlags, BytecodeStream, BytecodeReader};
