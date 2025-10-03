//! CODE INTELLIGENCE - GO TO DEFINITION, REFERENCES, ETC FOR 32 LANGUAGES

use crate::native_parser_manager::{NativeParserManager, FileType};
use tree_sitter::{Node, Point};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub path: PathBuf,
    pub position: Position,
}

pub struct SymbolIndex {
    symbols: DashMap<String, Vec<Location>>,
}

pub struct CodeIntelligence {
    parser_manager: Arc<NativeParserManager>,
    symbol_index: Arc<SymbolIndex>,
}

impl CodeIntelligence {
    pub fn new(parser_manager: Arc<NativeParserManager>) -> Self {
        Self {
            parser_manager,
            symbol_index: Arc::new(SymbolIndex::new()),
        }
    }
    
    pub async fn goto_definition(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Option<Location>, Box<dyn std::error::Error>> {
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        // Find node at position
        let node = self.find_node_at_position(
            parse_result.tree.root_node(),
            position.clone(),
        )?;
        
        // Get symbol name
        let symbol_name = node.utf8_text(parse_result.source.as_ref())?;
        
        // Search for definition in symbol index
        let definition = self.symbol_index
            .find_definition(symbol_name)
            .await?;
            
        Ok(definition)
    }
    
    pub async fn find_references(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<Location>, Box<dyn std::error::Error>> {
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        let node = self.find_node_at_position(
            parse_result.tree.root_node(),
            position,
        )?;
        
        let symbol_name = node.utf8_text(parse_result.source.as_ref())?;
        
        // Find all references
        let references = self.symbol_index
            .find_references(symbol_name)
            .await?;
            
        Ok(references)
    }
    
    pub async fn get_hover_info(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let parse_result = self.parser_manager.parse_file(path).await?;
        
        let node = self.find_node_at_position(
            parse_result.tree.root_node(),
            position,
        )?;
        
        // Generate hover info based on node type
        let info = self.generate_hover_info(&node, &parse_result.source)?;
        
        Ok(info)
    }
    
    fn find_node_at_position<'a>(&self, root: Node<'a>, position: Position) -> Result<Node<'a>, Box<dyn std::error::Error>> {
        let mut cursor = root.walk();
        let point = Point::new(position.line, position.column);
        
        loop {
            let node = cursor.node();
            let start = node.start_position();
            let end = node.end_position();
            
            if point.row >= start.row && point.row <= end.row {
                if point.column >= start.column && point.column <= end.column {
                    // Found node containing position
                    if cursor.goto_first_child() {
                        continue; // Try to find more specific node
                    } else {
                        return Ok(node);
                    }
                }
            }
            
            if !cursor.goto_next_sibling() {
                if cursor.goto_parent() {
                    cursor.goto_next_sibling();
                } else {
                    break;
                }
            }
        }
        
        Err("No node at position".into())
    }
    
    fn generate_hover_info(&self, node: &Node, source: &[u8]) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let node_text = node.utf8_text(source)?;
        let node_kind = node.kind();
        
        let info = match node_kind {
            "function_declaration" | "function_item" | "method_definition" => {
                format!("Function: {}\nType: {}", node_text, node_kind)
            }
            "variable_declaration" | "let_declaration" | "const_declaration" => {
                format!("Variable: {}\nType: {}", node_text, node_kind)
            }
            "class_declaration" | "struct_item" | "interface_declaration" => {
                format!("Type: {}\nKind: {}", node_text, node_kind)
            }
            _ => {
                format!("Symbol: {}\nKind: {}", node_text, node_kind)
            }
        };
        
        Ok(Some(info))
    }
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: DashMap::new(),
        }
    }
    
    pub async fn find_definition(&self, symbol: &str) -> Result<Option<Location>, Box<dyn std::error::Error>> {
        if let Some(locations) = self.symbols.get(symbol) {
            Ok(locations.first().cloned())
        } else {
            Ok(None)
        }
    }
    
    pub async fn find_references(&self, symbol: &str) -> Result<Vec<Location>, Box<dyn std::error::Error>> {
        if let Some(locations) = self.symbols.get(symbol) {
            Ok(locations.clone())
        } else {
            Ok(Vec::new())
        }
    }
    
    pub fn index_symbol(&self, symbol: String, location: Location) {
        self.symbols.entry(symbol)
            .or_insert_with(Vec::new)
            .push(location);
    }
}
