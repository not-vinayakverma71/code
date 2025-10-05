//! Bytecode-based tree representation for ultimate memory efficiency
//! Phase 3 optimization with 0% quality loss guarantee

pub mod opcodes;
pub mod encoder;
pub mod decoder;
pub mod navigator;
pub mod validator;

pub use opcodes::{Opcode, BytecodeStream};
pub use encoder::BytecodeEncoder;
pub use decoder::BytecodeDecoder;
pub use navigator::BytecodeNavigator;
pub use validator::BytecodeValidator;
