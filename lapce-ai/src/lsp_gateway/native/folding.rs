/// Folding Ranges (textDocument/foldingRange)
/// LSP-012: Provide code folding regions via tree-sitter queries

use anyhow::{Result, anyhow};
use std::sync::Arc;

/// Folding range provider
pub struct FoldingProvider {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
    min_lines: u32,
}

impl FoldingProvider {
    pub fn new(doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>) -> Self {
        Self {
            doc_sync,
            min_lines: 2, // Minimum 2 lines to be foldable
        }
    }
    
    /// Set minimum line count for folding
    pub fn set_min_lines(&mut self, min: u32) {
        self.min_lines = min;
    }

    /// Get folding ranges for document
    pub async fn get_folding_ranges(&self, uri: &str) -> Result<String> {
        #[cfg(feature = "cst_integration")]
        {
            let doc_sync = self.doc_sync.lock();
            
            let tree = doc_sync
                .get_tree(uri)
                .ok_or_else(|| anyhow!("Document tree not found: {}", uri))?;
            
            let text = doc_sync
                .get_text(uri)
                .ok_or_else(|| anyhow!("Document text not found: {}", uri))?;
            
            let mut ranges = Vec::new();
            
            // Walk tree and find foldable nodes
            let mut cursor = tree.root_node().walk();
            self.extract_folding_ranges(&mut cursor, text.as_bytes(), &mut ranges);
            
            // Filter by minimum line count
            ranges.retain(|r: &FoldingRange| {
                r.end_line - r.start_line >= self.min_lines
            });
            
            // Sort by start line
            ranges.sort_by_key(|r| r.start_line);
            
            serde_json::to_string(&ranges)
                .map_err(|e| anyhow!("Failed to serialize folding ranges: {}", e))
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            tracing::warn!("Folding ranges not available without cst_integration feature");
            Ok("[]".to_string())
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn extract_folding_ranges(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &[u8],
        ranges: &mut Vec<FoldingRange>,
    ) {
        loop {
            let node = cursor.node();
            let kind = node.kind();
            
            // Check if this node should be foldable
            if self.is_foldable_kind(kind) {
                let start = node.start_position();
                let end = node.end_position();
                
                // Determine folding kind
                let fold_kind = if kind.contains("comment") {
                    Some("comment".to_string())
                } else if kind.contains("import") || kind == "use_declaration" || kind == "using_directive" {
                    Some("imports".to_string())
                } else {
                    Some("region".to_string())
                };
                
                ranges.push(FoldingRange {
                    start_line: start.row as u32,
                    start_character: Some(start.column as u32),
                    end_line: end.row as u32,
                    end_character: Some(end.column as u32),
                    kind: fold_kind,
                });
            }
            
            // Recurse into children
            if cursor.goto_first_child() {
                self.extract_folding_ranges(cursor, source, ranges);
                cursor.goto_parent();
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn is_foldable_kind(&self, kind: &str) -> bool {
        matches!(kind,
            // Blocks
            "block" | "statement_block" | "compound_statement" |
            "declaration_list" | "field_declaration_list" |
            // Functions/Classes
            "function_declaration" | "function_definition" |
            "method_declaration" | "method_definition" |
            "class_declaration" | "class_definition" |
            "interface_declaration" | "struct_declaration" |
            "enum_declaration" | "trait_declaration" |
            // Control flow
            "if_statement" | "for_statement" | "while_statement" |
            "switch_statement" | "match_expression" |
            // Arrays/Objects
            "array" | "object" | "array_expression" | "object_expression" |
            // Comments
            "comment" | "block_comment" | "line_comment" |
            // Imports
            "import_statement" | "use_declaration" | "using_directive" |
            "import_declaration" | "from_import_statement" |
            // Others
            "module" | "namespace_declaration" | "try_statement"
        )
    }
}

impl Default for FoldingProvider {
    fn default() -> Self {
        // Can't use Default for Arc, so this won't work
        // Users must call new() with doc_sync
        panic!("Use FoldingProvider::new(doc_sync) instead of Default")
    }
}

// ============================================================================
// LSP Types
// ============================================================================

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FoldingRange {
    start_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_character: Option<u32>,
    end_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_character: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
}
