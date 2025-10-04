use anyhow::Result;
use tokio::fs;
use std::path::Path;

pub async fn write_file(path: &Path, content: &str) -> Result<()> {
    Ok(fs::write(path, content).await?)
}
