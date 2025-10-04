// FFI bindings for languages not available on crates.io or with version conflicts
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_kotlin() -> Language;
    fn tree_sitter_sql() -> Language;
    fn tree_sitter_r() -> Language;
    fn tree_sitter_julia() -> Language;
    fn tree_sitter_yaml() -> Language;
    fn tree_sitter_zig() -> Language;
    fn tree_sitter_nim() -> Language;
    fn tree_sitter_ada() -> Language;
    fn tree_sitter_fortran() -> Language;
    fn tree_sitter_asm() -> Language;
    fn tree_sitter_perl() -> Language;
    fn tree_sitter_tcl() -> Language;
    fn tree_sitter_racket() -> Language;
    fn tree_sitter_scheme() -> Language;
    fn tree_sitter_matlab() -> Language;
    fn tree_sitter_vhdl() -> Language;
    fn tree_sitter_vue() -> Language;
    fn tree_sitter_svelte() -> Language;
    fn tree_sitter_dart() -> Language;
    fn tree_sitter_haskell() -> Language;
    fn tree_sitter_clojure() -> Language;
    fn tree_sitter_gleam() -> Language;
    fn tree_sitter_wgsl() -> Language;
    fn tree_sitter_glsl() -> Language;
}

pub fn get_language(name: &str) -> Option<Language> {
    unsafe {
        match name {
            "kotlin" => Some(tree_sitter_kotlin()),
            "sql" => Some(tree_sitter_sql()),
            "r" => Some(tree_sitter_r()),
            "julia" => Some(tree_sitter_julia()),
            "yaml" => Some(tree_sitter_yaml()),
            "zig" => Some(tree_sitter_zig()),
            "nim" => Some(tree_sitter_nim()),
            "ada" => Some(tree_sitter_ada()),
            "fortran" => Some(tree_sitter_fortran()),
            "assembly" | "asm" => Some(tree_sitter_asm()),
            "perl" => Some(tree_sitter_perl()),
            "tcl" => Some(tree_sitter_tcl()),
            "racket" => Some(tree_sitter_racket()),
            "scheme" => Some(tree_sitter_scheme()),
            "matlab" => Some(tree_sitter_matlab()),
            "vhdl" => Some(tree_sitter_vhdl()),
            "vue" => Some(tree_sitter_vue()),
            "svelte" => Some(tree_sitter_svelte()),
            "dart" => Some(tree_sitter_dart()),
            "haskell" => Some(tree_sitter_haskell()),
            "clojure" => Some(tree_sitter_clojure()),
            "gleam" => Some(tree_sitter_gleam()),
            "wgsl" => Some(tree_sitter_wgsl()),
            "glsl" => Some(tree_sitter_glsl()),
            _ => None,
        }
    }
}
