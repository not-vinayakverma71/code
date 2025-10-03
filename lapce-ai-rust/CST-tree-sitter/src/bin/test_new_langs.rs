use tree_sitter::{Parser, Language};

fn main() {
    println!("\n📊 Testing New Languages Added in Phase 2");
    println!("==========================================\n");
    
    let languages = vec![
        ("Kotlin", tree_sitter_kotlin::language()),
        ("YAML", tree_sitter_yaml::language()),
        ("SQL", tree_sitter_sql::language()),
        ("GraphQL", tree_sitter_graphql::language()),
        ("Dart", tree_sitter_dart::language()),
        ("Haskell", tree_sitter_haskell::language()),
        ("R", tree_sitter_r::language()),
        ("Julia", tree_sitter_julia::language()),
        ("Clojure", tree_sitter_clojure::language()),
        ("Zig", tree_sitter_zig::language()),
        ("Nix", tree_sitter_nix::LANGUAGE),
        ("LaTeX", tree_sitter_latex::LANGUAGE),
        ("Make", tree_sitter_make::LANGUAGE),
        ("CMake", tree_sitter_cmake::LANGUAGE),
        ("Verilog", tree_sitter_verilog::LANGUAGE),
        ("Erlang", tree_sitter_erlang::LANGUAGE),
        ("D", tree_sitter_d::LANGUAGE),
    ];
    
    let mut success = 0;
    let mut fail = 0;
    
    for (name, lang) in languages {
        let mut parser = Parser::new();
        match parser.set_language(&lang.into()) {
            Ok(_) => {
                println!("✅ {:<10} - Language loaded successfully", name);
                success += 1;
            }
            Err(e) => {
                println!("❌ {:<10} - Error: {:?}", name, e);
                fail += 1;
            }
        }
    }
    
    println!("\n==========================================");
    println!("Results: {}/{} successful", success, success + fail);
}
