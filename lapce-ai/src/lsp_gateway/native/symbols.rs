/// Document Symbols (textDocument/documentSymbol)
/// LSP-008: Extract symbols using CST-tree-sitter with EXACT Codex schema

use anyhow::{Result, anyhow};
use std::sync::Arc;

#[cfg(feature = "cst_integration")]
use lapce_tree_sitter::symbols::SymbolExtractor as CstSymbolExtractor;

/// Symbol extraction handler
pub struct SymbolExtractor {
    // Shared reference to document sync for accessing trees
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
}

impl SymbolExtractor {
    pub fn new(doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>) -> Self {
        Self { doc_sync }
    }

    /// Extract document symbols
    pub async fn extract_symbols(&self, uri: &str, language_id: &str) -> Result<String> {
        #[cfg(feature = "cst_integration")]
        {
            // Get document tree from DocumentSync
            let doc_sync = self.doc_sync.lock();
            
            let tree = doc_sync
                .get_tree(uri)
                .ok_or_else(|| anyhow!("Document not found or not parsed: {}", uri))?;
            
            let text = doc_sync
                .get_text(uri)
                .ok_or_else(|| anyhow!("Document text not found: {}", uri))?;
            
            // Create symbol extractor for this language
            let mut extractor = CstSymbolExtractor::new(language_id);
            
            // Extract symbols with Codex schema
            let symbols = extractor.extract(tree, text.as_bytes());
            
            // Convert to LSP DocumentSymbol format
            let lsp_symbols = symbols
                .iter()
                .map(|sym| convert_to_lsp_symbol(sym))
                .collect::<Vec<_>>();
            
            // Serialize to JSON
            serde_json::to_string(&lsp_symbols)
                .map_err(|e| anyhow!("Failed to serialize symbols: {}", e))
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            // Without CST integration, return empty symbols
            tracing::warn!("Symbol extraction not available without cst_integration feature");
            Ok("[]".to_string())
        }
    }
}

// ============================================================================
// LSP Type Conversions
// ============================================================================

#[cfg(feature = "cst_integration")]
fn convert_to_lsp_symbol(symbol: &lapce_tree_sitter::symbols::Symbol) -> LspDocumentSymbol {
    use lapce_tree_sitter::ast::kinds::CanonicalKind;
    
    LspDocumentSymbol {
        name: symbol.name.clone(),
        detail: symbol.doc_comment.clone(),
        kind: kind_to_lsp_symbol_kind(&symbol.kind),
        range: LspRange {
            start: LspPosition {
                line: symbol.range.start.line as u32,
                character: symbol.range.start.column as u32,
            },
            end: LspPosition {
                line: symbol.range.end.line as u32,
                character: symbol.range.end.column as u32,
            },
        },
        selection_range: LspRange {
            start: LspPosition {
                line: symbol.range.start.line as u32,
                character: symbol.range.start.column as u32,
            },
            end: LspPosition {
                line: symbol.range.end.line as u32,
                character: symbol.range.end.column as u32,
            },
        },
        children: if symbol.children.is_empty() {
            None
        } else {
            Some(
                symbol
                    .children
                    .iter()
                    .map(|child| convert_to_lsp_symbol(child))
                    .collect(),
            )
        },
    }
}

#[cfg(feature = "cst_integration")]
fn kind_to_lsp_symbol_kind(kind: &lapce_tree_sitter::ast::kinds::CanonicalKind) -> u32 {
    use lapce_tree_sitter::ast::kinds::CanonicalKind;
    
    // LSP SymbolKind enum values
    match kind {
        CanonicalKind::FunctionDeclaration => 12, // Function
        CanonicalKind::ClassDeclaration => 5,     // Class
        CanonicalKind::StructDeclaration => 23,   // Struct
        CanonicalKind::EnumDeclaration => 10,     // Enum
        CanonicalKind::InterfaceDeclaration => 11, // Interface
        CanonicalKind::ConstantDeclaration => 14, // Constant
        CanonicalKind::VariableDeclaration => 13, // Variable
        _ => 1, // File (default)
    }
}

// ============================================================================
// LSP Types (subset needed for document symbols)
// ============================================================================

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LspDocumentSymbol {
    name: String,
    detail: Option<String>,
    kind: u32,
    range: LspRange,
    selection_range: LspRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<LspDocumentSymbol>>,
}

#[derive(Debug, serde::Serialize)]
struct LspRange {
    start: LspPosition,
    end: LspPosition,
}

#[derive(Debug, serde::Serialize)]
struct LspPosition {
    line: u32,
    character: u32,
}
