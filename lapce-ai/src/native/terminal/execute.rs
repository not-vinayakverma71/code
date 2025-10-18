use anyhow::Result;
use tokio::process::Command;

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn new() -> Self { Self }
    
    pub async fn execute(&self, cmd: &str) -> Result<String> {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
