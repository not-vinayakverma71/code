use std::path::PathBuf;

fn main() {
    // Only compile languages that actually exist in external-grammars
    let languages = vec![
        ("kotlin", "tree-sitter-kotlin"),
        ("yaml", "tree-sitter-yaml"),
        ("zig", "tree-sitter-zig"),
        ("nim", "tree-sitter-nim"),
        ("ada", "tree-sitter-ada"),
        ("fortran", "tree-sitter-fortran"),
        ("asm", "tree-sitter-asm"),
        ("perl", "tree-sitter-perl"),
        ("tcl", "tree-sitter-tcl"),
        ("racket", "tree-sitter-racket"),
        ("scheme", "tree-sitter-scheme"),
        ("matlab", "tree-sitter-matlab"),
        ("vhdl", "tree-sitter-vhdl"),
        ("vue", "tree-sitter-vue"),
        ("svelte", "tree-sitter-svelte"),
        ("dart", "tree-sitter-dart"),
        ("haskell", "tree-sitter-haskell"),
        ("clojure", "tree-sitter-clojure"),
        ("gleam", "tree-sitter-gleam"),
        ("wgsl", "tree-sitter-wgsl"),
        ("glsl", "tree-sitter-glsl"),
        ("r", "tree-sitter-r"),
        ("julia", "tree-sitter-julia"),
    ];

    for (name, dir) in languages {
        let dir_path = PathBuf::from("external-grammars").join(dir);
        let src_dir = dir_path.join("src");
        
        if src_dir.exists() {
            println!("cargo:rerun-if-changed={}/parser.c", src_dir.display());
            
            // Compile the parser
            let mut build = cc::Build::new();
            build.include(&src_dir);
            build.file(src_dir.join("parser.c"));
            
            // Some parsers have a scanner.c file
            let scanner_c = src_dir.join("scanner.c");
            if scanner_c.exists() {
                build.file(scanner_c);
            }
            
            // Some parsers have a scanner.cc file
            let scanner_cc = src_dir.join("scanner.cc");
            if scanner_cc.exists() {
                build.file(scanner_cc);
                build.cpp(true);
            }
            
            build.compile(&format!("tree-sitter-{}", name));
        }
    }
}
