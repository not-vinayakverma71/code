// Filesystem tools module - P0-4: FS tools batch 1

pub mod read_file;
pub mod read_file_v2;
pub mod list_files;
pub mod search_files;
pub mod write_file;
pub mod write_file_v2;
pub mod edit_file;
pub mod insert_content;
pub mod search_and_replace;
pub mod search_and_replace_v2;
pub mod utils;

pub use read_file::ReadFileTool;
pub use read_file_v2::ReadFileToolV2;
pub use list_files::ListFilesTool;
pub use search_files::SearchFilesTool;
pub use write_file::WriteFileTool;
pub use write_file_v2::WriteFileToolV2;
pub use edit_file::EditFileTool;
pub use insert_content::InsertContentTool;
pub use search_and_replace::SearchAndReplaceTool;
pub use search_and_replace_v2::SearchAndReplaceToolV2;

use std::path::{Path, PathBuf};
use serde_json::Value;

/// Helper to extract tool data from parsed XML
/// Handles both flat and nested structures from the parser
pub fn extract_tool_data(parsed: &Value) -> &Value {
    if parsed.get("tool").is_some() {
        &parsed["tool"]
    } else {
        parsed
    }
}

/// Check if a file is binary by examining first few bytes
/// Delegates to utils for enhanced detection
pub fn is_binary_file(path: &Path) -> bool {
    match utils::get_file_info(path) {
        Ok(info) => info.is_binary,
        Err(_) => {
            // Fallback to simple detection
            use std::fs::File;
            use std::io::Read;
            
            let mut file = match File::open(path) {
                Ok(f) => f,
                Err(_) => return false,
            };
            
            let mut buffer = [0u8; 8192];
            let bytes_read = match file.read(&mut buffer) {
                Ok(n) => n,
                Err(_) => return false,
            };
            
            // Check for null bytes (common in binary files)
            for &byte in &buffer[..bytes_read] {
                if byte == 0 {
                    return true;
                }
            }
            
            // Check for high ratio of non-printable characters
            let non_printable = buffer[..bytes_read]
                .iter()
                .filter(|&&b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r')
                .count();
            
            non_printable as f32 / bytes_read as f32 > 0.3
        }
    }
}

/// Check if file is an image by extension
pub fn is_image_file(path: &Path) -> bool {
    let image_extensions = [
        "jpg", "jpeg", "png", "gif", "bmp", "webp", 
        "svg", "tiff", "ico", "heic", "heif"
    ];
    
    if let Some(ext) = path.extension() {
        if let Some(ext_str) = ext.to_str() {
            return image_extensions.contains(&ext_str.to_lowercase().as_str());
        }
    }
    false
}

/// Ensure path is within workspace
pub fn ensure_workspace_path(workspace: &Path, target: &Path) -> Result<PathBuf, String> {
    let abs_target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        workspace.join(target)
    };
    
    let canonical_workspace = workspace.canonicalize()
        .map_err(|e| format!("Failed to canonicalize workspace: {}", e))?;
    
    let canonical_target = abs_target.canonicalize()
        .unwrap_or(abs_target.clone());
    
    if !canonical_target.starts_with(&canonical_workspace) {
        return Err(format!(
            "Path '{}' is outside workspace '{}'",
            target.display(),
            workspace.display()
        ));
    }
    
    Ok(canonical_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_is_binary_file() {
        let temp_dir = TempDir::new().unwrap();
        
        // Text file
        let text_path = temp_dir.path().join("text.txt");
        let mut text_file = File::create(&text_path).unwrap();
        writeln!(text_file, "Hello, world!").unwrap();
        assert!(!is_binary_file(&text_path));
        
        // Binary file
        let binary_path = temp_dir.path().join("binary.bin");
        let mut binary_file = File::create(&binary_path).unwrap();
        binary_file.write_all(&[0, 1, 2, 3, 255, 254]).unwrap();
        assert!(is_binary_file(&binary_path));
    }
    
    #[test]
    fn test_is_image_file() {
        assert!(is_image_file(Path::new("test.jpg")));
        assert!(is_image_file(Path::new("test.PNG")));
        assert!(is_image_file(Path::new("test.svg")));
        assert!(!is_image_file(Path::new("test.txt")));
        assert!(!is_image_file(Path::new("test.rs")));
    }
    
    #[test]
    fn test_ensure_workspace_path() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();
        
        // Path within workspace
        let valid_path = workspace.join("subdir/file.txt");
        let result = ensure_workspace_path(workspace, &valid_path);
        assert!(result.is_ok());
        
        // Path outside workspace
        let invalid_path = Path::new("/tmp/outside.txt");
        let result = ensure_workspace_path(workspace, invalid_path);
        assert!(result.is_err());
    }
}
