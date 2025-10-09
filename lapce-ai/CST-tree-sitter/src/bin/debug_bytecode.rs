use lapce_tree_sitter::compact::bytecode::{
    tree_sitter_encoder::TreeSitterBytecodeEncoder,
    opcodes::Opcode,
};
use tree_sitter::Parser;

fn main() {
    let _source = "fn main() { println!(\"hello\"); }";
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let _tree = parser.parse(source, None).unwrap();
    
    // Encode
    let mut encoder = TreeSitterBytecodeEncoder::new();
    let bytecode = encoder.encode_tree(&tree, source.as_bytes());
    
    println!("Source: {}", source);
    println!("Bytecode size: {} bytes", bytecode.bytes.len());
    println!("Node count: {}", bytecode.node_count);
    println!("\nBytecode stream:");
    
    let mut i = 0;
    while i < bytecode.bytes.len() {
        let byte = bytecode.bytes[i];
        if let Some(op) = Opcode::from_byte(byte) {
            println!("{:04}: {:?}", i, op);
        } else {
            println!("{:04}: data byte 0x{:02x}", i, byte);
        }
        i += 1;
    }
}
