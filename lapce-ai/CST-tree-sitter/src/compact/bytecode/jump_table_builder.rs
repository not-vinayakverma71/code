//! Jump table builder for efficient node navigation

use super::opcodes::{Opcode, BytecodeStream, BytecodeReader};

/// Build jump table for a bytecode stream
pub fn build_jump_table(stream: &mut BytecodeStream) {
    let mut reader = BytecodeReader::new(&stream.bytes);
    let mut node_offsets = Vec::new();
    let mut node_index = 0;
    
    // Scan through bytecode and record node start positions
    while !reader.is_at_end() {
        let pos = reader.pos;
        if let Some(op) = reader.read_op() {
            match op {
                Opcode::Enter | Opcode::Leaf => {
                    // Record this node's position
                    node_offsets.push(pos as u32);
                    node_index += 1;
                    
                    // Skip node data
                    reader.read_varint(); // kind_idx
                    reader.read_byte(); // flags
                    
                    // Skip position and length opcodes
                    while !reader.is_at_end() {
                        let peek_pos = reader.pos;
                        if let Some(next_op) = reader.read_op() {
                            match next_op {
                                Opcode::SetPos | Opcode::DeltaPos | Opcode::Length => {
                                    reader.read_varint();
                                }
                                _ => {
                                    reader.seek(peek_pos);
                                    break;
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    
                    // For Enter nodes, skip to matching Exit
                    if op == Opcode::Enter {
                        let mut depth = 1;
                        while depth > 0 && !reader.is_at_end() {
                            if let Some(inner_op) = reader.read_op() {
                                match inner_op {
                                    Opcode::Enter => {
                                        depth += 1;
                                        // Record nested node
                                        node_offsets.push((reader.pos - 1) as u32);
                                        node_index += 1;
                                        
                                        // Skip node data
                                        reader.read_varint(); // kind
                                        reader.read_byte(); // flags
                                    }
                                    Opcode::Leaf => {
                                        // Record leaf node
                                        node_offsets.push((reader.pos - 1) as u32);
                                        node_index += 1;
                                        
                                        // Skip node data
                                        reader.read_varint(); // kind
                                        reader.read_byte(); // flags
                                    }
                                    Opcode::Exit => {
                                        depth -= 1;
                                    }
                                    Opcode::Length | Opcode::DeltaPos | Opcode::SetPos => {
                                        reader.read_varint();
                                    }
                                    Opcode::Checkpoint => {
                                        reader.read_varint();
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Opcode::Checkpoint => {
                    reader.read_varint();
                }
                Opcode::End => break,
                _ => {}
            }
        }
    }
    
    // Update stream with jump table
    stream.jump_table = node_offsets;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compact::bytecode::TreeSitterBytecodeEncoder;
    use tree_sitter::Parser;
    
    #[test]
    fn test_jump_table_building() {
        let source = b"fn foo() {} fn bar() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let mut stream = encoder.encode_tree(&tree, source);
        
        // Build jump table
        build_jump_table(&mut stream);
        
        // Verify jump table has entries
        assert!(!stream.jump_table.is_empty());
        assert_eq!(stream.jump_table.len(), stream.node_count);
        
        // Verify each entry points to a valid node opcode
        for &offset in &stream.jump_table {
            let op = Opcode::from_byte(stream.bytes[offset as usize]).unwrap();
            assert!(matches!(op, Opcode::Enter | Opcode::Leaf));
        }
    }
}
