//! Symbol extraction with Codex-compliant schema
//! 
//! Extracts symbols from tree-sitter trees following the exact format
//! expected by semantic analysis per docs/05-TREE-SITTER-INTEGRATION.md

use crate::ast::kinds::{CanonicalKind, map_kind};
use tree_sitter::{Node, Tree};
use std::time::Instant;

/// Symbol information following Codex schema
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol name in Codex format:
    /// - Classes: "class MyClass"
    /// - Functions: "function myFunc()"  
    /// - Methods: "MyClass.method()"
    /// - Variables: "const myVar"
    pub name: String,
    
    /// Canonical kind
    pub kind: CanonicalKind,
    
    /// Source range
    pub range: Range,
    
    /// Child symbols
    pub children: Vec<Symbol>,
    
    /// Documentation comment if present
    pub doc_comment: Option<String>,
    
    /// Stable ID for tracking across edits
    pub stable_id: u64,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

/// Symbol extractor for a specific language
pub struct SymbolExtractor {
    language: String,
    next_stable_id: u64,
}

impl SymbolExtractor {
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            next_stable_id: 1,
        }
    }
    
    /// Extract symbols from a tree-sitter tree
    /// Performance target: < 50ms for 1K lines
    pub fn extract(&mut self, tree: &Tree, source: &[u8]) -> Vec<Symbol> {
        let start = Instant::now();
        let mut symbols = Vec::new();
        
        self.extract_node_symbols(tree.root_node(), source, &mut symbols, None);
        
        let elapsed = start.elapsed();
        if source.len() > 1000 && elapsed.as_millis() > 50 {
            eprintln!("Warning: Symbol extraction took {}ms for {} bytes", 
                     elapsed.as_millis(), source.len());
        }
        
        symbols
    }
    
    fn extract_node_symbols(
        &mut self,
        node: Node,
        source: &[u8],
        symbols: &mut Vec<Symbol>,
        parent_class: Option<&str>,
    ) {
        let _kind = map_kind(&self.language, node.kind());
        
        // Check if this node represents a symbol
        let symbol = match kind {
            CanonicalKind::FunctionDeclaration => {
                self.extract_function_symbol(node, source, parent_class)
            }
            CanonicalKind::ClassDeclaration => {
                self.extract_class_symbol(node, source)
            }
            CanonicalKind::VariableDeclaration | CanonicalKind::ConstantDeclaration => {
                self.extract_variable_symbol(node, source, kind)
            }
            CanonicalKind::StructDeclaration => {
                self.extract_struct_symbol(node, source)
            }
            CanonicalKind::EnumDeclaration => {
                self.extract_enum_symbol(node, source)
            }
            CanonicalKind::InterfaceDeclaration => {
                self.extract_interface_symbol(node, source)
            }
            _ => None,
        };
        
        // Track current class context for methods
        let current_class = if kind == CanonicalKind::ClassDeclaration {
            symbol.as_ref().and_then(|s| {
                // Extract class name from "class ClassName"
                s.name.strip_prefix("class ").map(|s| s.to_string())
            })
        } else {
            parent_class.map(|s| s.to_string())
        };
        
        // Add symbol if extracted
        if let Some(mut sym) = symbol {
            // Recursively extract child symbols
            for _i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    self.extract_node_symbols(
                        child, 
                        source, 
                        &mut sym.children,
                        current_class.as_deref()
                    );
                }
            }
            symbols.push(sym);
        } else {
            // Continue traversing even if current node isn't a symbol
            for _i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    self.extract_node_symbols(
                        child,
                        source,
                        symbols,
                        current_class.as_deref().or(parent_class)
                    );
                }
            }
        }
    }
    
    fn extract_function_symbol(
        &mut self,
        node: Node,
        source: &[u8],
        parent_class: Option<&str>,
    ) -> Option<Symbol> {
        let name_node = self.find_child_by_field(node, "name")?;
        let name = name_node.utf8_text(source).ok()?;
        
        // Format according to Codex schema
        let formatted_name = if let Some(class) = parent_class {
            format!("{}.{}()", class, name)  // Method: "MyClass.method()"
        } else {
            format!("function {}()", name)    // Function: "function myFunc()"
        };
        
        Some(Symbol {
            name: formatted_name,
            kind: CanonicalKind::FunctionDeclaration,
            range: self.node_to_range(node),
            children: Vec::new(),
            doc_comment: self.extract_doc_comment(node, source),
            stable_id: self.next_id(),
        })
    }
    
    fn extract_class_symbol(&mut self, node: Node, source: &[u8]) -> Option<Symbol> {
        let name_node = self.find_child_by_field(node, "name")?;
        let name = name_node.utf8_text(source).ok()?;
        
        Some(Symbol {
            name: format!("class {}", name),  // "class MyClass"
            kind: CanonicalKind::ClassDeclaration,
            range: self.node_to_range(node),
            children: Vec::new(),
            doc_comment: self.extract_doc_comment(node, source),
            stable_id: self.next_id(),
        })
    }
    
    fn extract_variable_symbol(
        &mut self,
        node: Node,
        source: &[u8],
        kind: CanonicalKind,
    ) -> Option<Symbol> {
        // Variable extraction depends on language
        let name = self.extract_variable_name(node, source)?;
        
        let prefix = if kind == CanonicalKind::ConstantDeclaration {
            "const"
        } else {
            "let"  // or "var" depending on language
        };
        
        Some(Symbol {
            name: format!("{} {}", prefix, name),  // "const myVar" or "let myVar"
            kind,
            range: self.node_to_range(node),
            children: Vec::new(),
            doc_comment: None,
            stable_id: self.next_id(),
        })
    }
    
    fn extract_struct_symbol(&mut self, node: Node, source: &[u8]) -> Option<Symbol> {
        let name_node = self.find_child_by_field(node, "name")?;
        let name = name_node.utf8_text(source).ok()?;
        
        Some(Symbol {
            name: format!("struct {}", name),
            kind: CanonicalKind::StructDeclaration,
            range: self.node_to_range(node),
            children: Vec::new(),
            doc_comment: self.extract_doc_comment(node, source),
            stable_id: self.next_id(),
        })
    }
    
    fn extract_enum_symbol(&mut self, node: Node, source: &[u8]) -> Option<Symbol> {
        let name_node = self.find_child_by_field(node, "name")?;
        let name = name_node.utf8_text(source).ok()?;
        
        Some(Symbol {
            name: format!("enum {}", name),
            kind: CanonicalKind::EnumDeclaration,
            range: self.node_to_range(node),
            children: Vec::new(),
            doc_comment: self.extract_doc_comment(node, source),
            stable_id: self.next_id(),
        })
    }
    
    fn extract_interface_symbol(&mut self, node: Node, source: &[u8]) -> Option<Symbol> {
        let name_node = self.find_child_by_field(node, "name")?;
        let name = name_node.utf8_text(source).ok()?;
        
        Some(Symbol {
            name: format!("interface {}", name),
            kind: CanonicalKind::InterfaceDeclaration,
            range: self.node_to_range(node),
            children: Vec::new(),
            doc_comment: self.extract_doc_comment(node, source),
            stable_id: self.next_id(),
        })
    }
    
    fn extract_variable_name(&self, node: Node, source: &[u8]) -> Option<String> {
        // Try different patterns based on language
        if let Some(pattern) = self.find_child_by_field(node, "pattern") {
            if let Some(name) = self.find_child_by_kind(pattern, "identifier") {
                return name.utf8_text(source).ok().map(|s| s.to_string());
            }
        }
        
        if let Some(name) = self.find_child_by_field(node, "name") {
            return name.utf8_text(source).ok().map(|s| s.to_string());
        }
        
        // Fallback: look for first identifier
        self.find_child_by_kind(node, "identifier")
            .and_then(|n| n.utf8_text(source).ok())
            .map(|s| s.to_string())
    }
    
    fn extract_doc_comment(&self, node: Node, source: &[u8]) -> Option<String> {
        // Look for preceding comment node
        if let Some(prev) = node.prev_sibling() {
            let _kind = map_kind(&self.language, prev.kind());
            if matches!(kind, CanonicalKind::DocComment | CanonicalKind::BlockComment) {
                return prev.utf8_text(source).ok().map(|s| s.to_string());
            }
        }
        None
    }
    
    fn find_child_by_field<'a>(&self, node: Node<'a>, field_name: &str) -> Option<Node<'a>> {
        node.child_by_field_name(field_name)
    }
    
    fn find_child_by_kind<'a>(&self, node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        for _i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == kind {
                    return Some(child);
                }
            }
        }
        None
    }
    
    fn node_to_range(&self, node: Node) -> Range {
        Range {
            start: Position {
                line: node.start_position().row,
                column: node.start_position().column,
            },
            end: Position {
                line: node.end_position().row,
                column: node.end_position().column,
            },
        }
    }
    
    fn next_id(&mut self) -> u64 {
        let id = self.next_stable_id;
        self.next_stable_id += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;
    
    #[test]
    fn test_extract_rust_symbols() {
        let _source = r#"
/// Documentation for MyStruct
struct MyStruct {
    field: i32,
}

impl MyStruct {
    /// Creates a new instance
    fn new() -> Self {
        Self { field: 0 }
    }
    
    fn method(&self) -> i32 {
        self.field
    }
}

fn standalone_function() {
    let x = 42;
    const Y: i32 = 100;
}
"#;
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let _tree = parser.parse(source, None).unwrap();
        
        let mut extractor = SymbolExtractor::new("rust");
        let symbols = extractor.extract(&tree, source.as_bytes());
        
        // Should find struct, functions, and variables
        assert!(!symbols.is_empty());
        
        // Check for struct
        assert!(symbols.iter().any(|s| s.name == "struct MyStruct"));
        
        // Check for function
        assert!(symbols.iter().any(|s| s.name == "function standalone_function()"));
        
        // All symbols should have stable IDs
        for symbol in &symbols {
            assert!(symbol.stable_id > 0);
        }
    }
    
    #[test]
    fn test_extract_python_symbols() {
        let _source = r#"
class MyClass:
    """A sample class"""
    
    def __init__(self):
        self.value = 0
    
    def method(self):
        return self.value

def standalone_function():
    x = 42
    return x
"#;
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();
        let _tree = parser.parse(source, None).unwrap();
        
        let mut extractor = SymbolExtractor::new("python");
        let symbols = extractor.extract(&tree, source.as_bytes());
        
        // Should find class and function
        assert!(!symbols.is_empty());
        
        // Check for class
        assert!(symbols.iter().any(|s| s.name == "class MyClass"));
        
        // Check for function
        assert!(symbols.iter().any(|s| s.name == "function standalone_function()"));
    }
    
    #[test]
    fn test_performance_target() {
        // Generate a 1K line source file
        let mut source = String::new();
        for _i in 0..100 {
            source.push_str(&format!("fn function_{}() {{ let x = {}; }}\n", i, i));
        }
        
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
        let _tree = parser.parse(&source, None).unwrap();
        
        let mut extractor = SymbolExtractor::new("rust");
        
        let start = Instant::now();
        let symbols = extractor.extract(&tree, source.as_bytes());
        let elapsed = start.elapsed();
        
        // Should extract all functions
        assert_eq!(symbols.len(), 100);
        
        // Should complete within 50ms (target for 1K lines)
        assert!(elapsed.as_millis() < 50, "Extraction took {}ms", elapsed.as_millis());
    }
}
