use anyhow::Result;
use std::path::Path;

pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

pub async fn list_files(path: &Path) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(path).await?;
    
    while let Some(entry) = dir.next_entry().await? {
        let metadata = entry.metadata().await?;
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }
    
    Ok(entries)
}
