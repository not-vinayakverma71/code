// Complete language registry mapping CST-tree-sitter languages to file extensions
// Based on CST-tree-sitter/Cargo.toml language definitions

use std::collections::HashMap;
use lazy_static::lazy_static;

pub struct LanguageInfo {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub tree_sitter_name: &'static str,
    pub is_core: bool, // true if from crates.io, false if external grammar
}

// Complete list of all 67 languages from CST-tree-sitter
static LANGUAGES: &[LanguageInfo] = &[
    // Core languages (31) - from crates.io
    LanguageInfo { name: "rust", extensions: &["rs"], tree_sitter_name: "rust", is_core: true },
    LanguageInfo { name: "python", extensions: &["py", "pyi"], tree_sitter_name: "python", is_core: true },
    LanguageInfo { name: "go", extensions: &["go"], tree_sitter_name: "go", is_core: true },
    LanguageInfo { name: "java", extensions: &["java"], tree_sitter_name: "java", is_core: true },
    LanguageInfo { name: "c", extensions: &["c", "h"], tree_sitter_name: "c", is_core: true },
    LanguageInfo { name: "cpp", extensions: &["cpp", "cc", "cxx", "hpp", "hxx", "h++"], tree_sitter_name: "cpp", is_core: true },
    LanguageInfo { name: "c_sharp", extensions: &["cs", "csx"], tree_sitter_name: "c-sharp", is_core: true },
    LanguageInfo { name: "ruby", extensions: &["rb", "gemspec", "podspec"], tree_sitter_name: "ruby", is_core: true },
    LanguageInfo { name: "php", extensions: &["php", "php3", "php4", "php5", "phps"], tree_sitter_name: "php", is_core: true },
    LanguageInfo { name: "lua", extensions: &["lua"], tree_sitter_name: "lua", is_core: true },
    LanguageInfo { name: "bash", extensions: &["sh", "bash", "zsh"], tree_sitter_name: "bash", is_core: true },
    LanguageInfo { name: "css", extensions: &["css"], tree_sitter_name: "css", is_core: true },
    LanguageInfo { name: "json", extensions: &["json"], tree_sitter_name: "json", is_core: true },
    LanguageInfo { name: "swift", extensions: &["swift"], tree_sitter_name: "swift", is_core: true },
    LanguageInfo { name: "scala", extensions: &["scala", "sc"], tree_sitter_name: "scala", is_core: true },
    LanguageInfo { name: "elixir", extensions: &["ex", "exs"], tree_sitter_name: "elixir", is_core: true },
    LanguageInfo { name: "html", extensions: &["html", "htm"], tree_sitter_name: "html", is_core: true },
    LanguageInfo { name: "ocaml", extensions: &["ml", "mli"], tree_sitter_name: "ocaml", is_core: true },
    LanguageInfo { name: "nix", extensions: &["nix"], tree_sitter_name: "nix", is_core: true },
    LanguageInfo { name: "make", extensions: &["make", "mk", "makefile", "Makefile"], tree_sitter_name: "make", is_core: true },
    LanguageInfo { name: "cmake", extensions: &["cmake"], tree_sitter_name: "cmake", is_core: true },
    LanguageInfo { name: "verilog", extensions: &["v", "vh", "sv", "svh"], tree_sitter_name: "verilog", is_core: true },
    LanguageInfo { name: "erlang", extensions: &["erl", "hrl"], tree_sitter_name: "erlang", is_core: true },
    LanguageInfo { name: "d", extensions: &["d", "di"], tree_sitter_name: "d", is_core: true },
    LanguageInfo { name: "pascal", extensions: &["pas", "pp"], tree_sitter_name: "pascal", is_core: true },
    LanguageInfo { name: "commonlisp", extensions: &["lisp", "cl", "l"], tree_sitter_name: "commonlisp", is_core: true },
    LanguageInfo { name: "objc", extensions: &["m", "mm"], tree_sitter_name: "objc", is_core: true },
    LanguageInfo { name: "groovy", extensions: &["groovy", "gradle"], tree_sitter_name: "groovy", is_core: true },
    LanguageInfo { name: "embedded_template", extensions: &["ejs", "erb"], tree_sitter_name: "embedded-template", is_core: true },
    
    // JavaScript and TypeScript are core (from crates.io)
    LanguageInfo { name: "javascript", extensions: &["js", "jsx", "mjs", "cjs"], tree_sitter_name: "javascript", is_core: true },
    LanguageInfo { name: "typescript", extensions: &["ts", "tsx", "mts", "cts"], tree_sitter_name: "typescript", is_core: true },
    
    // External grammar languages (35 remaining)
    LanguageInfo { name: "toml", extensions: &["toml"], tree_sitter_name: "toml", is_core: false },
    LanguageInfo { name: "dockerfile", extensions: &["dockerfile", "Dockerfile"], tree_sitter_name: "dockerfile", is_core: false },
    LanguageInfo { name: "elm", extensions: &["elm"], tree_sitter_name: "elm", is_core: false },
    LanguageInfo { name: "kotlin", extensions: &["kt", "kts"], tree_sitter_name: "kotlin", is_core: false },
    LanguageInfo { name: "yaml", extensions: &["yaml", "yml"], tree_sitter_name: "yaml", is_core: false },
    LanguageInfo { name: "r", extensions: &["r", "R"], tree_sitter_name: "r", is_core: false },
    LanguageInfo { name: "matlab", extensions: &["m", "mat"], tree_sitter_name: "matlab", is_core: false },
    LanguageInfo { name: "perl", extensions: &["pl", "pm", "pod"], tree_sitter_name: "perl", is_core: false },
    LanguageInfo { name: "dart", extensions: &["dart"], tree_sitter_name: "dart", is_core: false },
    LanguageInfo { name: "julia", extensions: &["jl"], tree_sitter_name: "julia", is_core: false },
    LanguageInfo { name: "haskell", extensions: &["hs", "lhs"], tree_sitter_name: "haskell", is_core: false },
    LanguageInfo { name: "graphql", extensions: &["graphql", "gql"], tree_sitter_name: "graphql", is_core: false },
    LanguageInfo { name: "sql", extensions: &["sql"], tree_sitter_name: "sql", is_core: false },
    LanguageInfo { name: "zig", extensions: &["zig"], tree_sitter_name: "zig", is_core: false },
    LanguageInfo { name: "vim", extensions: &["vim", "vimrc"], tree_sitter_name: "vim", is_core: false },
    LanguageInfo { name: "abap", extensions: &["abap"], tree_sitter_name: "abap", is_core: false },
    LanguageInfo { name: "nim", extensions: &["nim", "nims"], tree_sitter_name: "nim", is_core: false },
    LanguageInfo { name: "clojure", extensions: &["clj", "cljs", "cljc", "edn"], tree_sitter_name: "clojure", is_core: false },
    LanguageInfo { name: "crystal", extensions: &["cr"], tree_sitter_name: "crystal", is_core: false },
    LanguageInfo { name: "fortran", extensions: &["f", "for", "f90", "f95", "f03"], tree_sitter_name: "fortran", is_core: false },
    LanguageInfo { name: "vhdl", extensions: &["vhd", "vhdl"], tree_sitter_name: "vhdl", is_core: false },
    LanguageInfo { name: "racket", extensions: &["rkt"], tree_sitter_name: "racket", is_core: false },
    LanguageInfo { name: "ada", extensions: &["ada", "adb", "ads"], tree_sitter_name: "ada", is_core: false },
    LanguageInfo { name: "prolog", extensions: &["pl", "pro", "P"], tree_sitter_name: "prolog", is_core: false },
    LanguageInfo { name: "gradle", extensions: &["gradle"], tree_sitter_name: "gradle", is_core: false },
    LanguageInfo { name: "xml", extensions: &["xml", "xsd", "xsl", "xslt", "svg"], tree_sitter_name: "xml", is_core: false },
    LanguageInfo { name: "markdown", extensions: &["md", "markdown"], tree_sitter_name: "md", is_core: false },
    LanguageInfo { name: "svelte", extensions: &["svelte"], tree_sitter_name: "svelte", is_core: false },
    LanguageInfo { name: "scheme", extensions: &["scm", "ss"], tree_sitter_name: "scheme", is_core: false },
    LanguageInfo { name: "fennel", extensions: &["fnl"], tree_sitter_name: "fennel", is_core: false },
    LanguageInfo { name: "gleam", extensions: &["gleam"], tree_sitter_name: "gleam", is_core: false },
    LanguageInfo { name: "hcl", extensions: &["hcl", "tf", "tfvars"], tree_sitter_name: "hcl", is_core: false },
    LanguageInfo { name: "solidity", extensions: &["sol"], tree_sitter_name: "solidity", is_core: false },
    LanguageInfo { name: "fsharp", extensions: &["fs", "fsi", "fsx"], tree_sitter_name: "fsharp", is_core: false },
    LanguageInfo { name: "cobol", extensions: &["cob", "cbl", "cobol"], tree_sitter_name: "cobol", is_core: false },
    LanguageInfo { name: "systemverilog", extensions: &["sv", "svh"], tree_sitter_name: "systemverilog", is_core: false },
];

lazy_static! {
    pub static ref EXTENSION_TO_LANGUAGE: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        for lang in LANGUAGES {
            for ext in lang.extensions {
                map.insert(*ext, lang.name);
            }
        }
        map
    };
    
    pub static ref LANGUAGE_TO_TREE_SITTER: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        for lang in LANGUAGES {
            map.insert(lang.name, lang.tree_sitter_name);
        }
        map
    };
}

pub fn get_language_by_extension(ext: &str) -> Option<&'static str> {
    EXTENSION_TO_LANGUAGE.get(ext).copied()
}

pub fn get_tree_sitter_name(language: &str) -> Option<&'static str> {
    LANGUAGE_TO_TREE_SITTER.get(language).copied()
}

pub fn get_all_languages() -> &'static [LanguageInfo] {
    LANGUAGES
}

pub fn get_language_count() -> (usize, usize) {
    let core = LANGUAGES.iter().filter(|l| l.is_core).count();
    let external = LANGUAGES.iter().filter(|l| !l.is_core).count();
    (core, external)
}
