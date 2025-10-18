use tree_sitter::Parser;
use lapce_tree_sitter::compact::bytecode::{TreeSitterBytecodeEncoder, BytecodeNavigator};
use lapce_tree_sitter::cst_api::CstApiBuilder;

fn main() {
    let source = b"fn foo() {} fn bar() {} fn baz() {}";
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    
    println!("Tree root node: {:?}", tree.root_node().kind());
    println!("Child count: {}", tree.root_node().child_count());
    
    // Manually encode
    let mut encoder = TreeSitterBytecodeEncoder::new();
    let stream = encoder.encode_tree(&tree, source);
    
    println!("Bytecode size: {}", stream.bytes.len());
    println!("Node count: {}", stream.node_count);
    println!("Kind names: {:?}", stream.kind_names);
    
    // Try navigator
    let nav = BytecodeNavigator::new(&stream);
    println!("Navigator node count: {}", nav.node_count());
    
    // Try to get first few nodes
    for i in 0..5.min(nav.node_count()) {
        if let Some(node) = nav.get_node(i) {
            println!("Node {}: kind={}, name={}", i, node.kind_id, node.kind_name);
        }
    }
    
    // Now test CstApi
    let api = CstApiBuilder::new()
        .build_from_tree(&tree, source)
        .unwrap();
    
    let functions = api.find_nodes_by_kind("function_item");
    println!("Found {} function_item nodes", functions.len());
    
    // Try other kinds
    for kind in &["source_file", "fn", "identifier", "block"] {
        let nodes = api.find_nodes_by_kind(kind);
        println!("Found {} {} nodes", nodes.len(), kind);
    }
}
