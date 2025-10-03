// Native file reading operations - Direct I/O without MCP overhead
use std::path::Path;
use std::io;
use tokio::fs;

/// Direct file reading - no protocol overhead
pub async fn read_file(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).await
}

/// Read file with size limit
pub async fn read_file_limited(path: &Path, max_bytes: usize) -> io::Result<String> {
    let content = fs::read_to_string(path).await?;
    if content.len() > max_bytes {
        Ok(content.chars().take(max_bytes).collect())
    } else {
        Ok(content)
    }
}
