//! Bytecode-based tree representation for ultimate memory efficiency

pub mod opcodes;
pub mod encoder;
pub mod decoder;
pub mod navigator;
mod validator;
pub mod tree_sitter_encoder;
mod tree_sitter_decoder_v2;
pub mod segmented_fixed;
mod simple_verifier;
pub mod jump_table_builder;

#[cfg(test)]
mod bytecode_verification_tests;

pub use encoder::BytecodeEncoder;
pub use decoder::{BytecodeDecoder, DecodedNode};
pub use navigator::BytecodeNavigator;
pub use validator::BytecodeValidator;
pub use segmented_fixed::{SegmentedBytecodeStream, SegmentedNavigator, SegmentStatsSnapshot};
pub use tree_sitter_encoder::{TreeSitterBytecodeEncoder, TreeSitterBytecodeDecoder};
pub use opcodes::{Opcode, NodeFlags, BytecodeStream, BytecodeReader};
