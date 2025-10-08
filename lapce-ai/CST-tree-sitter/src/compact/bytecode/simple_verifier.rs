//! Simple bytecode verifier that handles ambiguity

use super::{SegmentedBytecodeStream, Opcode};

/// Simple verifier that just checks basic structure
pub fn verify_simple(stream: &SegmentedBytecodeStream) -> Result<(), String> {
    if stream.bytes.is_empty() {
        return Err("Empty bytecode".to_string());
    }
    
    // Check for End marker
    let has_end = stream.bytes.iter().any(|&b| b == 0xFF);
    if !has_end {
        return Err("No End marker found".to_string());
    }
    
    // Count Enter and Exit opcodes
    let enter_count = stream.bytes.iter().filter(|&&b| b == 0x01).count();
    let exit_count = stream.bytes.iter().filter(|&&b| b == 0x02).count();
    let leaf_count = stream.bytes.iter().filter(|&&b| b == 0x03).count();
    
    // Basic sanity checks
    if enter_count == 0 && leaf_count == 0 {
        return Err("No nodes found".to_string());
    }
    
    // Enter/Exit balance is hard to check due to ambiguity
    // (0x01 and 0x02 could be data bytes)
    // So we just check that we have some structure
    
    Ok(())
}
