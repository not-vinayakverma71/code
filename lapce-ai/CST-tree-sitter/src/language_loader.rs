// Unified language loader that handles version mismatches between tree-sitter crates
// This allows us to use parsers from different tree-sitter versions

use tree_sitter::Language;

// These functions are placeholders for now - the actual implementations 
// are blocked by version conflicts. When we fix the 4 remaining languages,
// we'll implement proper loaders for them.

pub fn load_toml_language() -> Language {
    panic!("TOML parser not available - version conflict with tree-sitter 0.20 vs 0.23")
}

pub fn load_dockerfile_language() -> Language {
    panic!("Dockerfile parser not available - version conflict with tree-sitter 0.20 vs 0.23")
}

pub fn load_prisma_language() -> Language {
    panic!("Prisma parser not available - version conflict with tree-sitter 0.20 vs 0.23")
}

pub fn load_hlsl_language() -> Language {
    panic!("HLSL parser not available - version conflict with tree-sitter 0.20 vs 0.23")
}
