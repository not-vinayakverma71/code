/// Simple code chunking without tree-sitter
/// Uses line-based sliding window approach

use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub path: PathBuf,
    pub content: String,
    pub language: String,
    pub start_line: u32,
    pub end_line: u32,
}

pub struct SimpleChunker {
    chunk_size: usize,     // Lines per chunk
    overlap: usize,        // Overlap between chunks
    max_chunk_chars: usize, // Max characters per chunk
}

impl SimpleChunker {
    pub fn new() -> Self {
        Self {
            chunk_size: 30,      // 30 lines per chunk
            overlap: 5,           // 5 lines overlap
            max_chunk_chars: 2000, // Max 2000 chars for embedding
        }
    }
    
    /// Get language from file extension
    fn get_language(path: &Path) -> String {
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("text")
            .to_string()
    }
    
    /// Chunk a file into overlapping segments
    pub async fn chunk_file(&self, path: &Path) -> Result<Vec<CodeChunk>> {
        let content = fs::read_to_string(path).await?;
        let lines: Vec<&str> = content.lines().collect();
        
        if lines.is_empty() {
            return Ok(vec![]);
        }
        
        let language = Self::get_language(path);
        let mut chunks = Vec::new();
        let stride = self.chunk_size.saturating_sub(self.overlap);
        
        let mut i = 0;
        while i < lines.len() {
            let end = (i + self.chunk_size).min(lines.len());
            
            // Skip if chunk is too small
            if end - i < 5 {
                break;
            }
            
            let chunk_lines = &lines[i..end];
            let chunk_content = chunk_lines.join("\n");
            
            // Skip if content is too large for embedding
            if chunk_content.len() > self.max_chunk_chars {
                // Try to reduce chunk size
                let reduced_end = i + (self.chunk_size / 2);
                if reduced_end > i + 5 {
                    let chunk_lines = &lines[i..reduced_end.min(lines.len())];
                    let chunk_content = chunk_lines.join("\n");
                    
                    chunks.push(CodeChunk {
                        path: path.to_path_buf(),
                        content: chunk_content,
                        language: language.clone(),
                        start_line: (i + 1) as u32,
                        end_line: reduced_end.min(lines.len()) as u32,
                    });
                }
            } else {
                chunks.push(CodeChunk {
                    path: path.to_path_buf(),
                    content: chunk_content,
                    language: language.clone(),
                    start_line: (i + 1) as u32,
                    end_line: end as u32,
                });
            }
            
            i += stride;
        }
        
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_chunking() {
        let chunker = SimpleChunker::new();
        // Test with a sample file
    }
}
