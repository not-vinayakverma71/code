/// Document Synchronization (didOpen, didChange, didClose)
/// LSP-006: Robust document lifecycle management with CST incremental parsing

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Tree, InputEdit, Point};

// Import from CST-tree-sitter if available
#[cfg(feature = "cst_integration")]
use lapce_tree_sitter::incremental::IncrementalParser;
#[cfg(feature = "cst_integration")]
use lapce_tree_sitter::language::LanguageRegistry;

/// Document state
struct DocumentState {
    uri: String,
    language_id: String,
    version: i32,
    text: String,
    #[cfg(feature = "cst_integration")]
    parser: IncrementalParser,
    #[cfg(feature = "cst_integration")]
    tree: Option<Tree>,
}

/// Document state tracker
pub struct DocumentSync {
    documents: HashMap<String, DocumentState>,
}

impl DocumentSync {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    /// Handle didOpen notification
    pub async fn did_open(&mut self, uri: &str, language_id: &str, text: &str) -> Result<()> {
        tracing::debug!("didOpen: uri={}, language_id={}", uri, language_id);
        
        // Normalize text (handle CRLF -> LF)
        let normalized_text = normalize_line_endings(text);
        
        #[cfg(feature = "cst_integration")]
        {
            // Detect language via LanguageRegistry if language_id not recognized
            let lang = detect_language(uri, language_id)?;
            
            // Create parser for this language
            let mut parser = IncrementalParser::new(&lang)
                .map_err(|e| anyhow!("Failed to create parser: {}", e))?;
            
            // Parse initial document
            let tree = parser.parse_full(normalized_text.as_bytes());
            
            // Store document state
            let state = DocumentState {
                uri: uri.to_string(),
                language_id: language_id.to_string(),
                version: 1,
                text: normalized_text,
                parser,
                tree,
            };
            
            self.documents.insert(uri.to_string(), state);
            tracing::info!("Document opened: {} (language: {})", uri, lang);
        }
        
        #[cfg(not(feature = "cst_integration"))]
        {
            // Fallback: just store text without parsing
            let state = DocumentState {
                uri: uri.to_string(),
                language_id: language_id.to_string(),
                version: 1,
                text: normalized_text,
            };
            self.documents.insert(uri.to_string(), state);
            tracing::info!("Document opened (no parser): {}", uri);
        }
        
        Ok(())
    }

    /// Handle didChange notification with incremental updates
    pub async fn did_change(&mut self, uri: &str, changes_json: &str) -> Result<()> {
        tracing::debug!("didChange: uri={}", uri);
        
        // Parse changes JSON (LSP TextDocumentContentChangeEvent)
        let changes: Vec<TextChange> = serde_json::from_str(changes_json)
            .map_err(|e| anyhow!("Failed to parse changes: {}", e))?;
        
        let doc = self.documents.get_mut(uri)
            .ok_or_else(|| anyhow!("Document not found: {}", uri))?;
        
        // Apply changes
        for change in changes {
            if let Some(range) = change.range {
                // Incremental change
                let start_byte = position_to_byte(&doc.text, range.start.line, range.start.character);
                let old_end_byte = position_to_byte(&doc.text, range.end.line, range.end.character);
                
                // Apply text change
                let new_text = apply_text_change(&doc.text, start_byte, old_end_byte, &change.text);
                let new_end_byte = start_byte + change.text.len();
                
                #[cfg(feature = "cst_integration")]
                {
                    // Create InputEdit for incremental parsing
                    let edit = create_input_edit(
                        doc.text.as_bytes(),
                        new_text.as_bytes(),
                        start_byte,
                        old_end_byte,
                        new_end_byte,
                    );
                    
                    // Parse incrementally (target: <10ms for micro-edits)
                    if let Ok(tree) = doc.parser.parse_incremental(new_text.as_bytes(), edit) {
                        doc.tree = Some(tree);
                    }
                }
                
                doc.text = new_text;
            } else {
                // Full document sync
                doc.text = normalize_line_endings(&change.text);
                
                #[cfg(feature = "cst_integration")]
                {
                    doc.tree = doc.parser.parse_full(doc.text.as_bytes());
                }
            }
        }
        
        doc.version += 1;
        tracing::debug!("Document updated: {} (version: {})", uri, doc.version);
        
        Ok(())
    }

    /// Handle didClose notification
    pub async fn did_close(&mut self, uri: &str) -> Result<()> {
        tracing::debug!("didClose: uri={}", uri);
        
        if self.documents.remove(uri).is_some() {
            tracing::info!("Document closed: {}", uri);
        } else {
            tracing::warn!("Document not found for close: {}", uri);
        }
        
        Ok(())
    }
    
    /// Get document text
    pub fn get_text(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|doc| doc.text.as_str())
    }
    
    /// Get document tree (if CST integration enabled)
    #[cfg(feature = "cst_integration")]
    pub fn get_tree(&self, uri: &str) -> Option<&Tree> {
        self.documents.get(uri).and_then(|doc| doc.tree.as_ref())
    }
}

impl Default for DocumentSync {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Types and Functions
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct TextChange {
    range: Option<TextRange>,
    text: String,
}

#[derive(Debug, serde::Deserialize)]
struct TextRange {
    start: Position,
    end: Position,
}

#[derive(Debug, serde::Deserialize)]
struct Position {
    line: u32,
    character: u32,
}

/// Normalize line endings (CRLF -> LF)
fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// Detect language from URI and language_id
fn detect_language(uri: &str, language_id: &str) -> Result<String> {
    // Try language_id first
    let lang = match language_id {
        "rust" | "rs" => "rust",
        "python" | "py" => "python",
        "javascript" | "js" => "javascript",
        "typescript" | "ts" => "typescript",
        _ => {
            // Fall back to file extension
            if let Ok(path) = url::Url::parse(uri) {
                let path_str = path.path();
                let path = Path::new(path_str);
                
                #[cfg(feature = "cst_integration")]
                {
                    if let Ok(info) = LanguageRegistry::instance().for_path(path) {
                        return Ok(info.name.to_string());
                    }
                }
                
                // Manual fallback
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext {
                        "rs" => "rust",
                        "py" => "python",
                        "js" | "jsx" => "javascript",
                        "ts" | "tsx" => "typescript",
                        _ => language_id,
                    }
                } else {
                    language_id
                }
            } else {
                language_id
            }
        }
    };
    
    Ok(lang.to_string())
}

/// Convert line/character position to byte offset
fn position_to_byte(text: &str, line: u32, character: u32) -> usize {
    let mut byte_offset = 0;
    let mut current_line = 0;
    
    for (i, ch) in text.char_indices() {
        if current_line == line {
            // Count characters in current line
            let line_start = byte_offset;
            let mut char_count = 0;
            
            for (j, _) in text[line_start..].char_indices() {
                if char_count == character {
                    return line_start + j;
                }
                char_count += 1;
            }
            
            // If we reach here, character is beyond line length
            return text.len();
        }
        
        if ch == '\n' {
            current_line += 1;
        }
        
        byte_offset = i + ch.len_utf8();
    }
    
    byte_offset
}

/// Apply text change to string
fn apply_text_change(text: &str, start_byte: usize, old_end_byte: usize, new_text: &str) -> String {
    let mut result = String::with_capacity(text.len() + new_text.len());
    result.push_str(&text[..start_byte]);
    result.push_str(new_text);
    result.push_str(&text[old_end_byte..]);
    result
}

/// Create InputEdit for tree-sitter incremental parsing
#[cfg(feature = "cst_integration")]
fn create_input_edit(
    old_source: &[u8],
    new_source: &[u8],
    start_byte: usize,
    old_end_byte: usize,
    new_end_byte: usize,
) -> InputEdit {
    InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position: byte_to_point(old_source, start_byte),
        old_end_position: byte_to_point(old_source, old_end_byte),
        new_end_position: byte_to_point(new_source, new_end_byte),
    }
}

/// Convert byte offset to Point (row, column)
#[cfg(feature = "cst_integration")]
fn byte_to_point(source: &[u8], byte_offset: usize) -> Point {
    let mut row = 0;
    let mut column = 0;
    
    for (i, &byte) in source.iter().enumerate() {
        if i >= byte_offset {
            break;
        }
        if byte == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    
    Point { row, column }
}
