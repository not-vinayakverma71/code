/// Symbol Index (workspace-wide symbol database)
/// LSP-010, LSP-011, LSP-015: Per-workspace index keyed by EXACT Codex names

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "cst_integration")]
use lapce_tree_sitter::symbols::SymbolExtractor as CstSymbolExtractor;

/// Symbol location
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolLocation {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

/// Symbol definition info
#[derive(Debug, Clone)]
struct SymbolDefinition {
    location: SymbolLocation,
    kind: String,
    doc_comment: Option<String>,
}

/// Workspace symbol index
pub struct SymbolIndex {
    // Map symbol name -> definition location
    symbols: HashMap<String, SymbolDefinition>,
    // Reverse map for references: symbol name -> list of reference locations
    references: HashMap<String, Vec<SymbolLocation>>,
    // Track which symbols belong to which file for cleanup
    file_symbols: HashMap<String, Vec<String>>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            references: HashMap::new(),
            file_symbols: HashMap::new(),
        }
    }

    /// Index a file from parsed tree
    #[cfg(feature = "cst_integration")]
    pub async fn index_file(
        &mut self,
        uri: &str,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &[u8],
    ) -> Result<()> {
        // Remove old symbols for this file
        self.remove_file(uri);
        
        // Extract symbols from tree
        let mut extractor = CstSymbolExtractor::new(language_id);
        let symbols = extractor.extract(tree, source);
        
        let mut indexed_symbols = Vec::new();
        
        // Index symbols recursively
        for symbol in &symbols {
            self.index_symbol_recursive(uri, symbol, &mut indexed_symbols)?;
        }
        
        // Track symbols for this file
        self.file_symbols.insert(uri.to_string(), indexed_symbols);
        
        tracing::debug!("Indexed {} symbols from {}", symbols.len(), uri);
        Ok(())
    }
    
    #[cfg(feature = "cst_integration")]
    fn index_symbol_recursive(
        &mut self,
        uri: &str,
        symbol: &lapce_tree_sitter::symbols::Symbol,
        indexed: &mut Vec<String>,
    ) -> Result<()> {
        // Create location
        let location = SymbolLocation {
            uri: uri.to_string(),
            line: symbol.range.start.line as u32,
            character: symbol.range.start.column as u32,
            end_line: symbol.range.end.line as u32,
            end_character: symbol.range.end.column as u32,
        };
        
        // Store definition
        let def = SymbolDefinition {
            location: location.clone(),
            kind: format!("{:?}", symbol.kind),
            doc_comment: symbol.doc_comment.clone(),
        };
        
        self.symbols.insert(symbol.name.clone(), def);
        indexed.push(symbol.name.clone());
        
        // Recursively index children
        for child in &symbol.children {
            self.index_symbol_recursive(uri, child, indexed)?;
        }
        
        Ok(())
    }
    
    /// Index a file (fallback without CST)
    #[cfg(not(feature = "cst_integration"))]
    pub async fn index_file(
        &mut self,
        _uri: &str,
        _language_id: &str,
        _tree: &tree_sitter::Tree,
        _source: &[u8],
    ) -> Result<()> {
        // Without CST integration, indexing is not available
        Ok(())
    }

    /// Find definition location by symbol name
    pub fn find_definition(&self, symbol_name: &str) -> Option<&SymbolLocation> {
        self.symbols.get(symbol_name).map(|def| &def.location)
    }
    
    /// Find definition with full info
    pub fn find_definition_info(&self, symbol_name: &str) -> Option<&SymbolDefinition> {
        self.symbols.get(symbol_name)
    }
    
    /// Find symbol at position in a file
    pub fn find_symbol_at_position(&self, uri: &str, line: u32, character: u32) -> Option<String> {
        // Look through symbols in this file
        if let Some(symbol_names) = self.file_symbols.get(uri) {
            for name in symbol_names {
                if let Some(def) = self.symbols.get(name) {
                    let loc = &def.location;
                    if loc.uri == uri
                        && loc.line <= line
                        && loc.end_line >= line
                    {
                        // Check character position for exact line
                        if loc.line == line && loc.character > character {
                            continue;
                        }
                        if loc.end_line == line && loc.end_character < character {
                            continue;
                        }
                        return Some(name.clone());
                    }
                }
            }
        }
        None
    }

    /// Add a reference to a symbol
    pub fn add_reference(&mut self, symbol_name: &str, location: SymbolLocation) {
        self.references
            .entry(symbol_name.to_string())
            .or_insert_with(Vec::new)
            .push(location);
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol_name: &str) -> Vec<&SymbolLocation> {
        self.references
            .get(symbol_name)
            .map(|refs| refs.iter().collect())
            .unwrap_or_default()
    }

    /// Search workspace symbols (fuzzy)
    pub fn search_symbols(&self, query: &str, limit: usize) -> Vec<(String, SymbolLocation)> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        // Simple fuzzy search: check if query is substring of symbol name
        for (name, def) in &self.symbols {
            if name.to_lowercase().contains(&query_lower) {
                results.push((name.clone(), def.location.clone()));
                if results.len() >= limit {
                    break;
                }
            }
        }
        
        // Sort by relevance (exact matches first, then by length)
        results.sort_by(|(a, _), (b, _)| {
            let a_exact = a.to_lowercase() == query_lower;
            let b_exact = b.to_lowercase() == query_lower;
            
            if a_exact && !b_exact {
                return std::cmp::Ordering::Less;
            }
            if !a_exact && b_exact {
                return std::cmp::Ordering::Greater;
            }
            
            a.len().cmp(&b.len())
        });
        
        results
    }

    /// Update index after file change
    #[cfg(feature = "cst_integration")]
    pub async fn update_file(
        &mut self,
        uri: &str,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &[u8],
    ) -> Result<()> {
        self.index_file(uri, language_id, tree, source).await
    }
    
    #[cfg(not(feature = "cst_integration"))]
    pub async fn update_file(
        &mut self,
        _uri: &str,
        _language_id: &str,
        _tree: &tree_sitter::Tree,
        _source: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    /// Remove file from index
    pub fn remove_file(&mut self, uri: &str) {
        // Get symbols for this file
        if let Some(symbol_names) = self.file_symbols.remove(uri) {
            // Remove each symbol
            for name in symbol_names {
                self.symbols.remove(&name);
                self.references.remove(&name);
            }
        }
        
        // Also remove references TO this file
        for refs in self.references.values_mut() {
            refs.retain(|loc| loc.uri != uri);
        }
    }
    
    /// Get statistics about the index
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            total_symbols: self.symbols.len(),
            total_files: self.file_symbols.len(),
            total_references: self.references.values().map(|v| v.len()).sum(),
        }
    }
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_symbols: usize,
    pub total_files: usize,
    pub total_references: usize,
}
