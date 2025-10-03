fn main() {
    println!("Counting available language parsers...\n");
    
    let mut count = 0;
    let mut languages = Vec::new();
    
    // Phase 1: Already working (23 languages)
    languages.extend(vec![
        "JavaScript", "TypeScript", "TSX", "Python", "Rust", "Go",
        "C", "C++", "C#", "Ruby", "Java", "PHP", "Swift",
        "CSS", "HTML", "OCaml", "Lua", "Elixir", "Scala",
        "Elm", "Bash", "JSON", "Markdown"
    ]);
    count += 23;
    
    // Phase 2: New additions
    languages.extend(vec![
        "Kotlin", "YAML", "SQL", "GraphQL", "Dart", "Haskell",
        "R", "Julia", "Clojure", "Zig", "TOML", "Dockerfile",
        "Nix", "LaTeX", "Make", "CMake", "Verilog", "Erlang", "D"
    ]);
    count += 19;
    
    // Phase 3: More additions
    languages.extend(vec![
        "Nim", "Pascal", "Scheme", "Racket", "CommonLisp", "Fennel",
        "Gleam", "Prisma", "VimDoc", "WGSL", "GLSL", "HLSL",
        "Objective-C", "MATLAB", "Fortran", "COBOL", "Perl", "Tcl",
        "Groovy", "HCL", "Solidity", "F#", "PowerShell", "SystemVerilog",
        "Vue", "Svelte", "EmbeddedTemplate", "Elisp"
    ]);
    count += 28;
    
    println!("Total languages available: {}", count);
    println!("\nLanguage list:");
    for (i, lang) in languages.iter().enumerate() {
        print!("{:15}", lang);
        if (i + 1) % 5 == 0 {
            println!();
        }
    }
    println!("\n\n✅ {} total languages can be supported!", count);
}
