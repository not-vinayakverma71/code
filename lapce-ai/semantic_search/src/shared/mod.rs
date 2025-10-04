// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: Copyright The LanceDB Authors

/// shared/supported-extensions.ts (Lines 1-35) - 100% EXACT
pub mod supported_extensions {
    // Lines 3-4: Scanner extensions - all file types we support
    pub const SCANNER_EXTENSIONS: &[&str] = &[
        ".ts", ".tsx", ".js", ".jsx", ".py", ".rs", ".go", ".java", ".c", ".cpp", ".h", ".hpp",
        ".cs", ".rb", ".php", ".swift", ".kt", ".scala", ".r", ".m", ".mm", ".sh", ".bash",
        ".zsh", ".fish", ".ps1", ".yaml", ".yml", ".json", ".toml", ".xml", ".html", ".css",
        ".scss", ".sass", ".less", ".sql", ".graphql", ".vue", ".svelte", ".md", ".markdown",
        ".vb"
    ];
    
    // Lines 21-25: Extensions that should use fallback chunking
    pub const FALLBACK_EXTENSIONS: &[&str] = &[
        ".vb",    // Visual Basic .NET - no dedicated WASM parser
        ".scala", // Scala - uses fallback chunking instead of Lua query workaround  
        ".swift", // Swift - uses fallback chunking due to parser instability
    ];
    
    /// Lines 32-34: Check if extension should use fallback chunking
    pub fn should_use_fallback_chunking(extension: &str) -> bool {
        FALLBACK_EXTENSIONS.iter()
            .any(|&ext| ext.eq_ignore_ascii_case(extension))
    }
}

/// shared/get-relative-path.ts
pub mod path_utils {
    use std::path::{Path, PathBuf};
    
    /// Generate relative file path from absolute path
    pub fn generate_relative_file_path(file_path: &Path, workspace: &Path) -> String {
        file_path.strip_prefix(workspace)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string()
    }
    
    /// Generate normalized absolute path
    pub fn generate_normalized_absolute_path(file_path: &str, workspace: &Path) -> PathBuf {
        if Path::new(file_path).is_absolute() {
            PathBuf::from(file_path)
        } else {
            workspace.join(file_path)
        }
    }
    
    /// Get workspace path for context
    pub fn get_workspace_path_for_context(directory: &Path) -> PathBuf {
        // In a real implementation, this would find the workspace root
        directory.to_path_buf()
    }
}

/// shared/validation-helpers.ts
pub mod validation_helpers {
    use crate::error::{Error, Result};
    
    /// Sanitize error message for telemetry
    pub fn sanitize_error_message(message: &str) -> String {
        // Remove sensitive information from error messages
        message
            .replace("api_key", "API_KEY=***")
            .replace("apiKey", "API_KEY=***")
            .replace("api-key", "API_KEY=***")
            .replace("token", "TOKEN=***")
            .replace("password", "PASSWORD=***")
    }
    
    /// Format embedding error with context
    pub fn format_embedding_error(error: Error, max_retries: usize) -> Error {
        Error::Runtime {
            message: format!("Embedding error after {} retries: {}", max_retries, error)
        }
    }
    
    /// Validation error handling wrapper
    pub async fn with_validation_error_handling<F, T>(
        f: F,
        provider: &str,
    ) -> Result<ValidationResult>
    where
        F: std::future::Future<Output = Result<ValidationResult>>,
    {
        match f.await {
            Ok(result) => Ok(result),
            Err(error) => {
                log::error!("Validation error for {}: {:?}", provider, error);
                Ok(ValidationResult {
                    valid: false,
                    error: Some(error.to_string()),
                })
            }
        }
    }
    
    #[derive(Debug, Clone)]
    pub struct ValidationResult {
        pub valid: bool,
        pub error: Option<String>,
    }
    
    #[derive(Debug)]
    pub struct HttpError {
        pub status: u16,
        pub message: String,
    }
}

/// constants/index.ts
pub mod constants {
    use uuid::Uuid;
    
    // Code block processing
    pub const MAX_BLOCK_CHARS: usize = 4000;
    pub const MIN_BLOCK_CHARS: usize = 100;
    pub const MIN_CHUNK_REMAINDER_CHARS: usize = 500;
    pub const MAX_CHARS_TOLERANCE_FACTOR: f32 = 1.5;
    
    // File processing
    pub const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10MB
    pub const MAX_LIST_FILES_LIMIT_CODE_INDEX: usize = 50000;
    
    // Batch processing
    pub const BATCH_SEGMENT_THRESHOLD: usize = 100;
    pub const MAX_BATCH_RETRIES: usize = 3;
    pub const INITIAL_RETRY_DELAY_MS: u64 = 1000;
    pub const BATCH_DEBOUNCE_DELAY_MS: u64 = 500;
    
    // Concurrency
    pub const PARSING_CONCURRENCY: usize = 10;
    pub const BATCH_PROCESSING_CONCURRENCY: usize = 5;
    pub const MAX_PENDING_BATCHES: usize = 3;
    pub const FILE_PROCESSING_CONCURRENCY_LIMIT: usize = 10;
    
    // Embedding
    pub const MAX_BATCH_TOKENS: usize = 8192;
    pub const MAX_ITEM_TOKENS: usize = 8192;
    pub const GEMINI_MAX_ITEM_TOKENS: usize = 30720;
    
    // Vector store
    pub const QDRANT_CODE_BLOCK_NAMESPACE: Uuid = Uuid::from_u128(0x12345678_1234_5678_1234_567812345678);
}
