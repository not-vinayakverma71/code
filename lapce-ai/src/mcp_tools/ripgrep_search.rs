use std::path::{Path, PathBuf};
use anyhow::Result;
use serde_json::{json, Value};
use grep::{
    regex::RegexMatcher,
    searcher::{BinaryDetection, SearcherBuilder},
    matcher::Matcher,
};
use ignore::WalkBuilder;
use tokio::sync::mpsc;

pub struct RipgrepSearch {
    max_results: usize,
    max_file_size: u64,
}

impl RipgrepSearch {
    pub fn new() -> Self {
        Self {
            max_results: 10000,
            max_file_size: 10 * 1024 * 1024, // 10MB
        }
    }
    
    pub async fn search(
        &self,
        pattern: &str,
        path: &Path,
        case_sensitive: bool,
        whole_word: bool,
        file_types: Option<Vec<String>>,
    ) -> Result<Vec<SearchResult>> {
        let matcher = if case_sensitive {
            RegexMatcher::new(pattern)?
        } else {
            RegexMatcher::new(&format!("(?i){}", pattern))?
        };
        
        let pattern = if whole_word {
            format!(r"\b{}\b", pattern)
        } else {
            pattern.to_string()
        };
        
        let matcher = RegexMatcher::new(&pattern)?;
        
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\0'))
            .line_number(true)
            .build();
        
        let (tx, mut rx) = mpsc::channel(100);
        let walker = self.build_walker(path, file_types);
        
        // Walk files in parallel - simplified implementation
        // Use the walk iterator directly
        tokio::spawn(async move {
            for entry in walker.build() {
                if let Ok(entry) = entry {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        let mut matches = Vec::new();
                        let max_results = 100; // Default max results
                        
                        use grep::searcher::sinks::UTF8;
                        let mut searcher = searcher.clone();
                        let _ = searcher.search_path(
                            &matcher,
                            entry.path(),
                            UTF8(|line_num, line| {
                                if matches.len() < max_results {
                                    matches.push(SearchMatch {
                                        line_number: line_num as u32,
                                        line_content: line.to_string(),
                                        column_start: 0,
                                        column_end: line.len(),
                                    });
                                }
                                Ok(true)
                            })
                        );
                        
                        if !matches.is_empty() {
                            let _ = tx.blocking_send(SearchResult {
                                file_path: entry.path().to_path_buf(),
                                matches,
                            });
                        }
                    }
                }
            }
        });
        
        // tx is already moved into the async block, no need to drop
        
        // Collect results
        let mut results = Vec::new();
        while let Some(result) = rx.recv().await {
            results.push(result);
            if results.len() >= self.max_results {
                break;
            }
        }
        
        Ok(results)
    }
    
    fn build_walker(&self, path: &Path, file_types: Option<Vec<String>>) -> WalkBuilder {
        let mut builder = WalkBuilder::new(path);
        
        builder
            .standard_filters(true)
            .max_filesize(Some(self.max_file_size))
            .threads(num_cpus::get())
            .follow_links(false);
        
        // Add file type filters
        if let Some(types) = file_types {
            for file_type in types {
                let mut types_builder = ignore::types::TypesBuilder::new();
                types_builder.add_defaults();
                types_builder.select(&file_type);
                let types = types_builder.build().unwrap();
                builder.types(types);
            }
        }
        
        builder
    }
    
    pub async fn search_and_replace(
        &self,
        pattern: &str,
        replacement: &str,
        path: &Path,
        dry_run: bool,
    ) -> Result<Vec<ReplacementResult>> {
        let results = self.search(pattern, path, true, false, None).await?;
        let mut replacements = Vec::new();
        
        for result in results {
            if !dry_run {
                let content = tokio::fs::read_to_string(&result.file_path).await?;
                let new_content = content.replace(pattern, replacement);
                tokio::fs::write(&result.file_path, new_content).await?;
            }
            
            replacements.push(ReplacementResult {
                file_path: result.file_path,
                replacements_count: result.matches.len(),
            });
        }
        
        Ok(replacements)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line_number: u32,
    pub line_content: String,
    pub column_start: usize,
    pub column_end: usize,
}

#[derive(Debug)]
pub struct ReplacementResult {
    pub file_path: PathBuf,
    pub replacements_count: usize,
}

impl SearchResult {
    pub fn to_json(&self) -> Value {
        json!({
            "file": self.file_path.display().to_string(),
            "matches": self.matches.iter().map(|m| json!({
                "line": m.line_number,
                "content": m.line_content,
                "column_start": m.column_start,
                "column_end": m.column_end,
            })).collect::<Vec<_>>()
        })
    }
}
