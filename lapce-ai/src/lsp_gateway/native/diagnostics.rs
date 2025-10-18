/// Diagnostics (publishDiagnostics)
/// LSP-014: Parse ERROR nodes and provide real-time diagnostics

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// LSP Types
// ============================================================================

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct Diagnostic {
    range: DiagnosticRange,
    severity: DiagnosticSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct DiagnosticRange {
    start: DiagnosticPosition,
    end: DiagnosticPosition,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct DiagnosticPosition {
    line: u32,
    character: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[repr(u32)]
pub(crate) enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

// ============================================================================
// Diagnostics Provider
// ============================================================================

/// Diagnostics provider
pub struct DiagnosticsProvider {
    doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>,
    // Track per-document diagnostics
    diagnostics: HashMap<String, Vec<Diagnostic>>,
    // Debounce: track last computation time
    last_computed: HashMap<String, Instant>,
    // Debounce duration
    debounce_ms: u64,
}

impl DiagnosticsProvider {
    pub fn new(doc_sync: Arc<parking_lot::Mutex<super::DocumentSync>>) -> Self {
        Self {
            doc_sync,
            diagnostics: HashMap::new(),
            last_computed: HashMap::new(),
            debounce_ms: 300, // 300ms debounce
        }
    }
    
    /// Set debounce duration in milliseconds
    pub fn set_debounce_ms(&mut self, ms: u64) {
        self.debounce_ms = ms;
    }

    /// Compute diagnostics for document
    pub async fn compute_diagnostics(&mut self, uri: &str, _language_id: &str) -> Result<String> {
        // Check debounce
        if let Some(last_time) = self.last_computed.get(uri) {
            let elapsed = last_time.elapsed();
            if elapsed < Duration::from_millis(self.debounce_ms) {
                // Return cached diagnostics
                return self.serialize_diagnostics(uri);
            }
        }
        
        #[cfg(feature = "cst_integration")]
        {
            let doc_sync = self.doc_sync.lock();
            
            let tree = doc_sync
                .get_tree(uri)
                .ok_or_else(|| anyhow!("Document tree not found: {}", uri))?;
            
            let text = doc_sync
                .get_text(uri)
                .ok_or_else(|| anyhow!("Document text not found: {}", uri))?;
            
            // Extract diagnostics from tree
            let mut diagnostics = Vec::new();
            let mut cursor = tree.root_node().walk();
            
            self.extract_diagnostics(&mut cursor, text.as_bytes(), &mut diagnostics);
            
            // Sort by position
            diagnostics.sort_by(|a, b| {
                a.range.start.line.cmp(&b.range.start.line)
                    .then(a.range.start.character.cmp(&b.range.start.character))
            });
            
            // Store diagnostics
            self.diagnostics.insert(uri.to_string(), diagnostics);
            self.last_computed.insert(uri.to_string(), Instant::now());
            
            self.serialize_diagnostics(uri)
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            tracing::warn!("Diagnostics not available without cst_integration feature");
            Ok("[]".to_string())
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn extract_diagnostics(
        &self,
        cursor: &mut tree_sitter::TreeCursor,
        source: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        loop {
            let node = cursor.node();
            
            // Check for ERROR or MISSING nodes
            if node.is_error() || node.is_missing() {
                let start = node.start_position();
                let end = node.end_position();
                
                let message = if node.is_missing() {
                    format!("Syntax error: expected {}", node.kind())
                } else {
                    "Syntax error: unexpected token".to_string()
                };
                
                diagnostics.push(Diagnostic {
                    range: DiagnosticRange {
                        start: DiagnosticPosition {
                            line: start.row as u32,
                            character: start.column as u32,
                        },
                        end: DiagnosticPosition {
                            line: end.row as u32,
                            character: end.column as u32,
                        },
                    },
                    severity: DiagnosticSeverity::Error,
                    code: None,
                    source: Some("tree-sitter".to_string()),
                    message,
                });
            }
            
            // Additional language-specific checks
            self.check_language_rules(&node, source, diagnostics);
            
            // Recurse into children
            if cursor.goto_first_child() {
                self.extract_diagnostics(cursor, source, diagnostics);
                cursor.goto_parent();
            }
            
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    
    #[cfg(feature = "cst_integration")]
    fn check_language_rules(
        &self,
        node: &tree_sitter::Node,
        source: &[u8],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let kind = node.kind();
        
        // Example: Check for unused variables (simplified)
        if kind == "identifier" {
            let text = &source[node.start_byte()..node.end_byte()];
            let text_str = String::from_utf8_lossy(text);
            
            // Check for common issues
            if text_str.starts_with("unused_") {
                let start = node.start_position();
                let end = node.end_position();
                
                diagnostics.push(Diagnostic {
                    range: DiagnosticRange {
                        start: DiagnosticPosition {
                            line: start.row as u32,
                            character: start.column as u32,
                        },
                        end: DiagnosticPosition {
                            line: end.row as u32,
                            character: end.column as u32,
                        },
                    },
                    severity: DiagnosticSeverity::Warning,
                    code: Some("unused-variable".to_string()),
                    source: Some("lsp-gateway".to_string()),
                    message: format!("Variable '{}' appears to be unused", text_str),
                });
            }
        }
        
        // Example: Deprecated syntax warnings
        if kind == "deprecated_syntax" {
            let start = node.start_position();
            let end = node.end_position();
            
            diagnostics.push(Diagnostic {
                range: DiagnosticRange {
                    start: DiagnosticPosition {
                        line: start.row as u32,
                        character: start.column as u32,
                    },
                    end: DiagnosticPosition {
                        line: end.row as u32,
                        character: end.column as u32,
                    },
                },
                severity: DiagnosticSeverity::Warning,
                code: Some("deprecated".to_string()),
                source: Some("lsp-gateway".to_string()),
                message: "Deprecated syntax".to_string(),
            });
        }
    }
    
    fn serialize_diagnostics(&self, uri: &str) -> Result<String> {
        let diags = self.diagnostics.get(uri).cloned().unwrap_or_default();
        serde_json::to_string(&diags)
            .map_err(|e| anyhow!("Failed to serialize diagnostics: {}", e))
    }

    /// Clear diagnostics for a document
    pub fn clear_diagnostics(&mut self, uri: &str) {
        self.diagnostics.remove(uri);
        self.last_computed.remove(uri);
    }

    /// Get current diagnostics for a document (as JSON string)
    pub fn get_diagnostics(&self, uri: &str) -> Result<String> {
        self.serialize_diagnostics(uri)
    }
}

impl Default for DiagnosticsProvider {
    fn default() -> Self {
        panic!("Use DiagnosticsProvider::new(doc_sync) instead of Default")
    }
}
