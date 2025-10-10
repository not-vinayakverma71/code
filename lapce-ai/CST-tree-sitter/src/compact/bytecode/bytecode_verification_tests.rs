//! Bytecode verification tests for edge cases

#[cfg(test)]
mod tests {
    use crate::compact::bytecode::{
        TreeSitterBytecodeEncoder, BytecodeStream, Opcode, BytecodeReader,
        jump_table_builder::build_jump_table,
    };
    use tree_sitter::Parser;
    
    /// Test that leaf nodes (child_count == 0) are encoded correctly
    #[test]
    fn test_leaf_node_encoding() {
        // Simple source with leaf nodes
        let source = b"x = 1";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let stream = encoder.encode_tree(&tree, source);
        
        // Verify bytecode structure
        let mut reader = BytecodeReader::new(&stream.bytes);
        let mut leaf_count = 0;
        let mut enter_count = 0;
        let mut exit_count = 0;
        
        while !reader.is_at_end() {
            if let Some(op) = reader.read_op() {
                match op {
                    Opcode::Leaf => {
                        leaf_count += 1;
                        // Leaf nodes should have: kind_idx, flags, then position/length opcodes
                        let kind_idx = reader.read_varint().expect("Leaf should have kind_idx");
                        let flags = reader.read_byte().expect("Leaf should have flags");
                        
                        // Skip position/length opcodes
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
                    }
                    Opcode::Enter => {
                        enter_count += 1;
                        reader.read_varint(); // kind_idx
                        reader.read_byte(); // flags
                    }
                    Opcode::Exit => {
                        exit_count += 1;
                    }
                    Opcode::End => break,
                    _ => {}
                }
            }
        }
        
        // Verify we have leaf nodes
        assert!(leaf_count > 0, "Should have leaf nodes");
        
        // Verify Enter/Exit balance
        assert_eq!(enter_count, exit_count, "Enter/Exit should be balanced");
    }
    
    /// Test that varint encoding/decoding is aligned correctly
    #[test]
    fn test_varint_alignment() {
        let mut stream = BytecodeStream::new();
        
        // Write various sized varints
        stream.write_varint(0);      // 1 byte
        stream.write_varint(127);    // 1 byte
        stream.write_varint(128);    // 2 bytes
        stream.write_varint(16383);  // 2 bytes
        stream.write_varint(16384);  // 3 bytes
        
        // Read them back
        let mut reader = BytecodeReader::new(&stream.bytes);
        assert_eq!(reader.read_varint(), Some(0));
        assert_eq!(reader.read_varint(), Some(127));
        assert_eq!(reader.read_varint(), Some(128));
        assert_eq!(reader.read_varint(), Some(16383));
        assert_eq!(reader.read_varint(), Some(16384));
        
        // Should be at end
        assert!(reader.is_at_end());
    }
    
    /// Test that nodes with no children still write Exit for Enter nodes
    #[test]
    fn test_empty_enter_node_has_exit() {
        // Create a simple tree with an empty block
        let source = b"fn foo() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let stream = encoder.encode_tree(&tree, source);
        
        // Track Enter/Exit depth
        let mut reader = BytecodeReader::new(&stream.bytes);
        let mut depth = 0;
        let mut max_depth = 0;
        
        while !reader.is_at_end() {
            if let Some(op) = reader.read_op() {
                match op {
                    Opcode::Enter => {
                        depth += 1;
                        max_depth = max_depth.max(depth);
                        reader.read_varint(); // kind_idx
                        reader.read_byte(); // flags
                        
                        // Skip position/length opcodes
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
                    }
                    Opcode::Exit => {
                        depth -= 1;
                        assert!(depth >= 0, "Exit without matching Enter");
                    }
                    Opcode::Leaf => {
                        reader.read_varint(); // kind_idx
                        reader.read_byte(); // flags
                        
                        // Skip position/length opcodes
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
                    }
                    Opcode::End => break,
                    _ => {}
                }
            }
        }
        
        // Depth should return to 0
        assert_eq!(depth, 0, "All Enter nodes should have matching Exit");
        assert!(max_depth > 0, "Should have some nesting");
    }
    
    /// Test that we never encounter 0x00 where an opcode is expected
    #[test]
    fn test_no_null_opcodes() {
        let source = b"fn main() { let x = 1; let y = 2; }";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let stream = encoder.encode_tree(&tree, source);
        
        // Scan for valid opcodes
        let mut reader = BytecodeReader::new(&stream.bytes);
        let mut last_was_opcode = true;
        
        while !reader.is_at_end() {
            let byte = stream.bytes[reader.pos];
            
            if last_was_opcode {
                // This should be an opcode
                assert_ne!(byte, 0x00, "Found 0x00 where opcode expected at position {}", reader.pos);
                
                if let Some(op) = Opcode::from_byte(byte) {
                    reader.pos += 1;
                    
                    // Handle opcode data
                    match op {
                        Opcode::Enter | Opcode::Leaf => {
                            reader.read_varint(); // kind_idx
                            reader.read_byte(); // flags
                            last_was_opcode = false; // Next might be position opcode
                        }
                        Opcode::Exit | Opcode::End => {
                            last_was_opcode = true;
                        }
                        Opcode::SetPos | Opcode::DeltaPos | Opcode::Length => {
                            reader.read_varint();
                            last_was_opcode = false; // More position opcodes might follow
                        }
                        _ => {}
                    }
                } else {
                    panic!("Invalid opcode {} at position {}", byte, reader.pos);
                }
            } else {
                // Check if this is another position opcode
                if let Some(op) = Opcode::from_byte(byte) {
                    match op {
                        Opcode::SetPos | Opcode::DeltaPos | Opcode::Length => {
                            reader.pos += 1;
                            reader.read_varint();
                        }
                        _ => {
                            // Back to normal opcodes
                            last_was_opcode = true;
                        }
                    }
                } else {
                    // Not an opcode, skip
                    reader.pos += 1;
                }
            }
        }
    }
    
    /// Test jump table building
    #[test]
    fn test_jump_table_correctness() {
        let source = b"fn a() {} fn b() {} fn c() {}";
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        
        let mut encoder = TreeSitterBytecodeEncoder::new();
        let mut stream = encoder.encode_tree(&tree, source);
        
        // Build jump table
        build_jump_table(&mut stream);
        
        // Verify jump table
        assert_eq!(stream.jump_table.len(), stream.node_count,
                   "Jump table should have entry for each node");
        
        // Each jump table entry should point to Enter or Leaf opcode
        for (i, &offset) in stream.jump_table.iter().enumerate() {
            let op = Opcode::from_byte(stream.bytes[offset as usize])
                .expect(&format!("Jump table entry {} points to invalid byte", i));
            assert!(
                matches!(op, Opcode::Enter | Opcode::Leaf),
                "Jump table entry {} should point to Enter or Leaf, got {:?}", i, op
            );
        }
    }
}
