use tree_sitter::Parser;

fn main() {
    println!("\n🔍 VERIFYING THE 6 LANGUAGES FIXED IN THIS SESSION");
    println!("==================================================\n");
    
    let test_cases = vec![
        ("JavaScript", tree_sitter_javascript::language(), "const test = () => { return 42; }"),
        ("JSX", tree_sitter_javascript::language(), "<div className='test'>{value}</div>"),
        ("SystemVerilog", tree_sitter_systemverilog::LANGUAGE(), "module test; logic [7:0] data; endmodule"),
        ("Elm", tree_sitter_elm::LANGUAGE(), "module Main exposing (main)\n\nadd x y = x + y"),
        ("XML", tree_sitter_xml::language(), "<?xml version=\"1.0\"?>\n<root><child>test</child></root>"),
        ("COBOL", tree_sitter_cobol::language(), "IDENTIFICATION DIVISION.\nPROGRAM-ID. TEST."),
    ];
    
    let mut success_count = 0;
    let mut fail_count = 0;
    
    for (name, language, code) in test_cases.iter() {
        let mut parser = Parser::new();
        
        match parser.set_language(&language) {
            Ok(_) => {
                match parser.parse(code, None) {
                    Some(tree) => {
                        let root = tree.root_node();
                        if !root.has_error() {
                            println!("✅ {}: Successfully parsed ({} nodes)", name, root.descendant_count());
                            success_count += 1;
                        } else {
                            println!("⚠️ {}: Parse tree has errors", name);
                            fail_count += 1;
                        }
                    }
                    None => {
                        println!("❌ {}: Failed to parse", name);
                        fail_count += 1;
                    }
                }
            }
            Err(e) => {
                println!("❌ {}: Failed to set language: {:?}", name, e);
                fail_count += 1;
            }
        }
    }
    
    println!("\n==================================================");
    println!("RESULTS:");
    println!("✅ Successful: {}/6", success_count);
    println!("❌ Failed: {}/6", fail_count);
    
    if fail_count == 0 {
        println!("\n🎉 ALL 6 FIXED LANGUAGES ARE WORKING PERFECTLY!");
    } else {
        println!("\n⚠️ Some languages still have issues");
    }
}
