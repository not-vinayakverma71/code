#!/bin/bash

# Fix external grammars that have compilation issues

echo "Fixing external grammars with proper type conversions..."

# Fix JavaScript
if [ -f "external-grammars/tree-sitter-javascript/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-javascript..."
  cat > external-grammars/tree-sitter-javascript/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_javascript() -> Language;
}

/// The tree-sitter Language for JavaScript
pub const LANGUAGE: Language = unsafe { tree_sitter_javascript() };

/// Returns the tree-sitter Language for JavaScript
pub unsafe fn language() -> Language {
    LANGUAGE
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");
pub const HIGHLIGHT_QUERY: &str = include_str!("../../queries/highlights.scm");
pub const INJECTIONS_QUERY: &str = include_str!("../../queries/injections.scm");
pub const JSX_HIGHLIGHT_QUERY: &str = include_str!("../../queries/highlights-jsx.scm");
pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading JavaScript parser");
    }
}
EOF
fi

# Fix TypeScript  
if [ -f "external-grammars/tree-sitter-typescript/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-typescript..."
  cat > external-grammars/tree-sitter-typescript/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_typescript() -> Language;
    fn tree_sitter_tsx() -> Language;
}

pub const LANGUAGE_TYPESCRIPT: Language = unsafe { tree_sitter_typescript() };
pub const LANGUAGE_TSX: Language = unsafe { tree_sitter_tsx() };

pub unsafe fn language_typescript() -> Language {
    LANGUAGE_TYPESCRIPT
}

pub unsafe fn language_tsx() -> Language {
    LANGUAGE_TSX
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");
pub const LOCALS_QUERY: &str = include_str!("../../queries/locals.scm");
pub const TAGS_QUERY: &str = include_str!("../../queries/tags.scm");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_typescript() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE_TYPESCRIPT)
            .expect("Error loading TypeScript parser");
    }
    
    #[test]
    fn test_can_load_tsx() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE_TSX)
            .expect("Error loading TSX parser");
    }
}
EOF
fi

# Fix Solidity
if [ -f "external-grammars/tree-sitter-solidity/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-solidity..."
  cat > external-grammars/tree-sitter-solidity/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_solidity() -> Language;
}

pub const LANGUAGE: Language = unsafe { tree_sitter_solidity() };

pub unsafe fn language() -> Language {
    LANGUAGE
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading Solidity parser");
    }
}
EOF
fi

# Fix HCL
if [ -f "external-grammars/tree-sitter-hcl/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-hcl..."
  cat > external-grammars/tree-sitter-hcl/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_hcl() -> Language;
}

pub const LANGUAGE: Language = unsafe { tree_sitter_hcl() };

pub unsafe fn language() -> Language {
    LANGUAGE
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading HCL parser");
    }
}
EOF
fi

# Fix Scheme
if [ -f "external-grammars/tree-sitter-scheme/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-scheme..."
  cat > external-grammars/tree-sitter-scheme/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_scheme() -> Language;
}

pub const LANGUAGE: Language = unsafe { tree_sitter_scheme() };

pub unsafe fn language() -> Language {
    LANGUAGE
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading Scheme parser");
    }
}
EOF
fi

# Fix F#
if [ -f "external-grammars/tree-sitter-fsharp/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-fsharp..."
  cat > external-grammars/tree-sitter-fsharp/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_fsharp() -> Language;
    fn tree_sitter_fsharp_signature() -> Language;
}

pub const LANGUAGE_FSHARP: Language = unsafe { tree_sitter_fsharp() };
pub const LANGUAGE_FSHARP_SIGNATURE: Language = unsafe { tree_sitter_fsharp_signature() };

pub unsafe fn language() -> Language {
    LANGUAGE_FSHARP
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE_FSHARP)
            .expect("Error loading F# parser");
    }
}
EOF
fi

# Fix all other problematic grammars with simple pattern
for grammar in tree-sitter-gleam tree-sitter-glsl tree-sitter-tcl tree-sitter-toml tree-sitter-powershell tree-sitter-asm tree-sitter-fennel tree-sitter-astro tree-sitter-wgsl tree-sitter-cairo; do
  if [ -d "external-grammars/$grammar" ]; then
    gram_name=${grammar#tree-sitter-}
    gram_name=${gram_name//-/_}
    
    echo "Fixing $grammar..."
    cat > "external-grammars/$grammar/bindings/rust/lib.rs" << EOF
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_${gram_name}() -> Language;
}

pub const LANGUAGE: Language = unsafe { tree_sitter_${gram_name}() };

pub unsafe fn language() -> Language {
    LANGUAGE
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading ${gram_name} parser");
    }
}
EOF
  fi
done

# Fix markdown (special case with tree-sitter-md)
if [ -f "external-grammars/tree-sitter-markdown/bindings/rust/lib.rs" ]; then
  echo "Fixing tree-sitter-markdown..."
  cat > external-grammars/tree-sitter-markdown/bindings/rust/lib.rs << 'EOF'
use tree_sitter::Language;

extern "C" {
    fn tree_sitter_md() -> Language;
    fn tree_sitter_markdown_inline() -> Language;
}

pub const LANGUAGE: Language = unsafe { tree_sitter_md() };
pub const LANGUAGE_INLINE: Language = unsafe { tree_sitter_markdown_inline() };

pub unsafe fn language() -> Language {
    LANGUAGE
}

pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE)
            .expect("Error loading Markdown parser");
    }
}
EOF
fi

echo "All external grammars fixed!"
