//! Production-ready module for Lapce IDE integration
//! Complete with error handling, performance metrics, and async support

use crate::main_api::LapceTreeSitterAPI;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};

/// Production Tree-Sitter service for Lapce IDE
pub struct LapceTreeSitterService {
    api: Arc<LapceTreeSitterAPI>,
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl LapceTreeSitterService {
    /// Create new production service
    pub fn new() -> Self {
        Self {
            api: Arc::new(LapceTreeSitterAPI::new()),
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }
    
    /// Extract symbols with error handling and metrics
    pub async fn extract_symbols_safe(
        &self,
        file_path: &str,
        source_code: &str,
    ) -> Result<SymbolExtractionResult> {
        let start = Instant::now();
        
        // Check file support
        if !self.api.is_file_supported(file_path) {
            return Ok(SymbolExtractionResult {
                success: false,
                symbols: None,
                error: Some(format!("Unsupported file type: {}", file_path)),
                duration: start.elapsed(),
                language: None,
            });
        }
        
        // Extract symbols
        match self.api.extract_symbols(file_path, source_code) {
            Some(symbols) => {
                let duration = start.elapsed();
                
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.total_extractions += 1;
                metrics.successful_extractions += 1;
                metrics.total_duration += duration;
                metrics.update_average();
                
                // Detect language
                let language = detect_language(file_path);
                
                Ok(SymbolExtractionResult {
                    success: true,
                    symbols: Some(symbols),
                    error: None,
                    duration,
                    language,
                })
            }
            None => {
                let duration = start.elapsed();
                
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.total_extractions += 1;
                metrics.failed_extractions += 1;
                
                Ok(SymbolExtractionResult {
                    success: false,
                    symbols: None,
                    error: Some("Failed to extract symbols".to_string()),
                    duration,
                    language: detect_language(file_path),
                })
            }
        }
    }
    
    /// Extract from file path with error handling
    pub async fn extract_from_path_safe(
        &self,
        file_path: &str,
    ) -> Result<SymbolExtractionResult> {
        // Read file
        let source_code = tokio::fs::read_to_string(file_path)
            .await
            .context(format!("Failed to read file: {}", file_path))?;
        
        self.extract_symbols_safe(file_path, &source_code).await
    }
    
    /// Extract from directory with progress callback
    pub async fn extract_from_directory_with_progress<F>(
        &self,
        dir_path: &str,
        mut progress_callback: F,
    ) -> Result<DirectoryExtractionResult>
    where
        F: FnMut(usize, usize) + Send,
    {
        let start = Instant::now();
        
        // Get list of files first
        let files = list_supported_files(dir_path).await?;
        let total_files = files.len();
        let mut processed = 0;
        let mut results = Vec::new();
        
        for file_path in files {
            processed += 1;
            progress_callback(processed, total_files);
            
            match self.extract_from_path_safe(&file_path).await {
                Ok(result) => results.push((file_path, result)),
                Err(e) => {
                    // Log error but continue
                    eprintln!("Error processing {}: {}", file_path, e);
                }
            }
        }
        
        Ok(DirectoryExtractionResult {
            total_files: total_files,
            successful_files: results.iter().filter(|(_, r)| r.success).count(),
            failed_files: results.iter().filter(|(_, r)| !r.success).count(),
            duration: start.elapsed(),
            file_results: results,
        })
    }
    
    /// Get current performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Reset performance metrics
    pub async fn reset_metrics(&self) {
        *self.metrics.write().await = PerformanceMetrics::default();
    }
    
    /// Health check - verify parsers are working
    pub async fn health_check(&self) -> HealthStatus {
        let mut working_languages = Vec::new();
        let mut failing_languages = Vec::new();
        
        // Test each language with simple code
        let test_cases = vec![
            ("test.js", "function test() {}"),
            ("test.ts", "interface Test {}"),
            ("test.py", "def test(): pass"),
            ("test.rs", "fn test() {}"),
            ("test.go", "func test() {}"),
            ("test.java", "class Test {}"),
            ("test.cpp", "int main() {}"),
            ("test.rb", "def test; end"),
            ("test.php", "<?php function test() {} ?>"),
            ("test.cs", "class Test {}"),
        ];
        
        for (file, code) in test_cases {
            if self.api.extract_symbols(file, code).is_some() {
                working_languages.push(file.to_string());
            } else {
                failing_languages.push(file.to_string());
            }
        }
        
        let coverage = (working_languages.len() as f64 / 23.0) * 100.0;
        
        HealthStatus {
            is_healthy: !working_languages.is_empty(),
            working_languages,
            failing_languages,
            total_supported: 23,
            coverage_percentage: coverage,
        }
    }
}

/// Result of symbol extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolExtractionResult {
    pub success: bool,
    pub symbols: Option<String>,
    pub error: Option<String>,
    pub duration: Duration,
    pub language: Option<String>,
}

/// Result of directory extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryExtractionResult {
    pub total_files: usize,
    pub successful_files: usize,
    pub failed_files: usize,
    pub duration: Duration,
    pub file_results: Vec<(String, SymbolExtractionResult)>,
}

/// Performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_extractions: usize,
    pub successful_extractions: usize,
    pub failed_extractions: usize,
    pub total_duration: Duration,
    pub average_duration: Duration,
}

impl PerformanceMetrics {
    fn update_average(&mut self) {
        if self.successful_extractions > 0 {
            self.average_duration = self.total_duration / self.successful_extractions as u32;
        }
    }
    
    pub fn success_rate(&self) -> f64 {
        if self.total_extractions == 0 {
            return 0.0;
        }
        (self.successful_extractions as f64 / self.total_extractions as f64) * 100.0
    }
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub working_languages: Vec<String>,
    pub failing_languages: Vec<String>,
    pub total_supported: usize,
    pub coverage_percentage: f64,
}

/// Detect language from file path
fn detect_language(file_path: &str) -> Option<String> {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())?;
    
    Some(match ext {
        "js" | "jsx" => "JavaScript",
        "ts" => "TypeScript",
        "tsx" => "TSX",
        "py" => "Python",
        "rs" => "Rust",
        "go" => "Go",
        "c" | "h" => "C",
        "cpp" | "hpp" => "C++",
        "cs" => "C#",
        "rb" => "Ruby",
        "java" => "Java",
        "php" => "PHP",
        "swift" => "Swift",
        "lua" => "Lua",
        "ex" | "exs" => "Elixir",
        "scala" => "Scala",
        "css" => "CSS",
        "json" => "JSON",
        "toml" => "TOML",
        "sh" | "bash" => "Bash",
        "elm" => "Elm",
        "md" | "markdown" => "Markdown",
        _ => return None,
    }.to_string())
}

/// List supported files in directory
async fn list_supported_files(dir_path: &str) -> Result<Vec<String>> {
    use tokio::fs;
    use std::path::Path;
    
    let mut files = Vec::new();
    let mut entries = fs::read_dir(dir_path).await?;
    
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(path_str) = path.to_str() {
                if LapceTreeSitterAPI::new().is_file_supported(path_str) {
                    files.push(path_str.to_string());
                }
            }
        }
    }
    
    Ok(files)
}

impl Default for LapceTreeSitterService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_service_creation() {
        let service = LapceTreeSitterService::new();
        let metrics = service.get_metrics().await;
        assert_eq!(metrics.total_extractions, 0);
    }
    
    #[tokio::test]
    async fn test_symbol_extraction() {
        let service = LapceTreeSitterService::new();
        let code = "fn main() { println!(\"test\"); }";
        let result = service.extract_symbols_safe("test.rs", code).await.unwrap();
        assert!(result.success);
        assert!(result.symbols.is_some());
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let service = LapceTreeSitterService::new();
        let health = service.health_check().await;
        assert!(health.is_healthy);
        assert!(!health.working_languages.is_empty());
    }
}
