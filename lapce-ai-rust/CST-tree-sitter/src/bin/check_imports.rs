fn main() {
    println!("Checking imports only...");
    
    // Check if we can import parsers
    println!("Importing parsers...");
    
    use tree_sitter::Parser;
    
    // Try to create parsers without using them
    let _js = tree_sitter_javascript::LANGUAGE;
    println!("✅ JavaScript");
    
    let _ts = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
    println!("✅ TypeScript");
    
    let _tsx = tree_sitter_typescript::LANGUAGE_TSX;
    println!("✅ TSX");
    
    let _py = tree_sitter_python::LANGUAGE;
    println!("✅ Python");
    
    let _rs = tree_sitter_rust::LANGUAGE;
    println!("✅ Rust");
    
    let _go = tree_sitter_go::LANGUAGE;
    println!("✅ Go");
    
    // Check new parsers
    let _kt = tree_sitter_kotlin::language();
    println!("✅ Kotlin");
    
    let _sol = tree_sitter_solidity::LANGUAGE;
    println!("✅ Solidity");
    
    println!("\nAll imports successful!");
}
