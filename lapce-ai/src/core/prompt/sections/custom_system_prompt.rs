//! Custom System Prompt Loader
//!
//! 1:1 Translation from Codex `src/core/prompts/sections/custom-system-prompt.ts`
//!
//! Reference: /home/verma/lapce/Codex/src/core/prompts/sections/custom-system-prompt.ts (lines 1-90)

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tokio::fs;

use crate::core::prompt::errors::{PromptError, PromptResult};
use crate::core::tools::fs::{ensure_workspace_path, utils::{FileEncoding, detect_encoding}};

/// Variables for prompt interpolation
///
/// Translation of PromptVariables type from custom-system-prompt.ts (lines 6-12)
#[derive(Debug, Clone)]
pub struct PromptVariables {
    pub workspace: Option<String>,
    pub mode: Option<String>,
    pub language: Option<String>,
    pub shell: Option<String>,
    pub operating_system: Option<String>,
}

impl PromptVariables {
    pub fn new() -> Self {
        Self {
            workspace: None,
            mode: None,
            language: None,
            shell: None,
            operating_system: None,
        }
    }
    
    /// Create variables from workspace path and mode
    pub fn from_workspace(workspace: &Path, mode: &str) -> Self {
        Self {
            workspace: Some(workspace.to_string_lossy().to_string()),
            mode: Some(mode.to_string()),
            language: Some("en".to_string()), // Default to English
            shell: std::env::var("SHELL").ok(),
            operating_system: Some(std::env::consts::OS.to_string()),
        }
    }
    
    /// Convert to HashMap for interpolation
    fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        
        if let Some(ref workspace) = self.workspace {
            map.insert("workspace".to_string(), workspace.clone());
        }
        if let Some(ref mode) = self.mode {
            map.insert("mode".to_string(), mode.clone());
        }
        if let Some(ref language) = self.language {
            map.insert("language".to_string(), language.clone());
        }
        if let Some(ref shell) = self.shell {
            map.insert("shell".to_string(), shell.clone());
        }
        if let Some(ref os) = self.operating_system {
            map.insert("operatingSystem".to_string(), os.clone());
        }
        
        map
    }
}

impl Default for PromptVariables {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpolate variables in prompt content
///
/// Translation of interpolatePromptContent() from custom-system-prompt.ts (lines 14-26)
///
/// Replaces {{variable}} placeholders with actual values
fn interpolate_prompt_content(content: &str, variables: &PromptVariables) -> String {
    let var_map = variables.to_map();
    let mut result = content.to_string();
    
    for (key, value) in var_map.iter() {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    result
}

/// Safely read a file, returning empty string if doesn't exist
///
/// Translation of safeReadFile() from custom-system-prompt.ts (lines 31-43)
async fn safe_read_file(file_path: &Path, workspace: &Path) -> PromptResult<String> {
    // Security: ensure file is within workspace
    ensure_workspace_path(workspace, file_path)
        .map_err(|e| PromptError::OutsideWorkspace(format!("{}: {}", file_path.display(), e)))?;
    
    // Check if file exists
    if !file_path.exists() {
        return Ok(String::new());
    }
    
    // Check if it's a directory
    if file_path.is_dir() {
        return Ok(String::new());
    }
    
    // Check if binary using encoding detection
    let encoding = detect_encoding(file_path)?;
    if encoding == FileEncoding::Binary {
        return Err(PromptError::BinaryFile(file_path.display().to_string()));
    }
    
    // Read file content
    let content = fs::read_to_string(file_path).await?;
    
    Ok(content.trim().to_string())
}

/// Get the path to a system prompt file for a specific mode
///
/// Translation of getSystemPromptFilePath() from custom-system-prompt.ts (lines 48-51)
pub fn get_system_prompt_file_path(workspace: &Path, mode: &str) -> PathBuf {
    workspace.join(".kilocode").join(format!("system-prompt-{}", mode))
}

/// Load custom system prompt from file at .kilocode/system-prompt-{mode}
///
/// Translation of loadSystemPromptFile() from custom-system-prompt.ts (lines 57-65)
///
/// # Arguments
///
/// * `workspace` - Workspace root directory
/// * `mode` - Mode slug (e.g., "code", "architect")
/// * `variables` - Variables for interpolation
///
/// # Returns
///
/// Interpolated prompt content, or empty string if file doesn't exist
pub async fn load_system_prompt_file(
    workspace: &Path,
    mode: &str,
    variables: &PromptVariables,
) -> PromptResult<String> {
    let file_path = get_system_prompt_file_path(workspace, mode);
    
    let raw_content = safe_read_file(&file_path, workspace).await?;
    
    if raw_content.is_empty() {
        return Ok(String::new());
    }
    
    let interpolated = interpolate_prompt_content(&raw_content, variables);
    
    Ok(interpolated)
}

/// Ensure .kilocode directory exists
///
/// Translation of ensureRooDirectory() from custom-system-prompt.ts (lines 70-89)
pub async fn ensure_kilocode_directory(workspace: &Path) -> PromptResult<()> {
    let kilocode_dir = workspace.join(".kilocode");
    
    // Check if directory exists
    if kilocode_dir.exists() {
        return Ok(());
    }
    
    // Create directory
    fs::create_dir_all(&kilocode_dir).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;
    
    #[test]
    fn test_prompt_variables_new() {
        let vars = PromptVariables::new();
        assert!(vars.workspace.is_none());
        assert!(vars.mode.is_none());
    }
    
    #[test]
    fn test_prompt_variables_from_workspace() {
        let workspace = Path::new("/tmp/workspace");
        let vars = PromptVariables::from_workspace(workspace, "code");
        
        assert_eq!(vars.mode, Some("code".to_string()));
        assert!(vars.workspace.is_some());
        assert!(vars.operating_system.is_some());
    }
    
    #[test]
    fn test_interpolate_simple() {
        let mut vars = PromptVariables::new();
        vars.mode = Some("code".to_string());
        vars.workspace = Some("/home/user/project".to_string());
        
        let content = "Mode: {{mode}}, Workspace: {{workspace}}";
        let result = interpolate_prompt_content(content, &vars);
        
        assert_eq!(result, "Mode: code, Workspace: /home/user/project");
    }
    
    #[test]
    fn test_interpolate_missing_variable() {
        let vars = PromptVariables::new();
        let content = "Mode: {{mode}}, Language: {{language}}";
        let result = interpolate_prompt_content(content, &vars);
        
        // Missing variables remain as placeholders
        assert!(result.contains("{{mode}}"));
        assert!(result.contains("{{language}}"));
    }
    
    #[tokio::test]
    async fn test_safe_read_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");
        
        let content = safe_read_file(&file_path, temp_dir.path()).await.unwrap();
        assert_eq!(content, "");
    }
    
    #[tokio::test]
    async fn test_safe_read_file_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).await.unwrap();
        
        let content = safe_read_file(&dir_path, temp_dir.path()).await.unwrap();
        assert_eq!(content, "");
    }
    
    #[tokio::test]
    async fn test_safe_read_file_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "  Hello, world!  \n").await.unwrap();
        
        let content = safe_read_file(&file_path, temp_dir.path()).await.unwrap();
        assert_eq!(content, "Hello, world!");
    }
    
    #[tokio::test]
    async fn test_load_system_prompt_file_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let vars = PromptVariables::from_workspace(temp_dir.path(), "code");
        
        let result = load_system_prompt_file(temp_dir.path(), "code", &vars).await.unwrap();
        assert_eq!(result, "");
    }
    
    #[tokio::test]
    async fn test_load_system_prompt_file_with_interpolation() {
        let temp_dir = TempDir::new().unwrap();
        let kilocode_dir = temp_dir.path().join(".kilocode");
        fs::create_dir(&kilocode_dir).await.unwrap();
        
        let prompt_file = kilocode_dir.join("system-prompt-code");
        fs::write(&prompt_file, "You are in {{mode}} mode at {{workspace}}").await.unwrap();
        
        let vars = PromptVariables::from_workspace(temp_dir.path(), "code");
        let result = load_system_prompt_file(temp_dir.path(), "code", &vars).await.unwrap();
        
        assert!(result.contains("code mode"));
        assert!(result.contains(&temp_dir.path().to_string_lossy().to_string()));
    }
    
    #[tokio::test]
    async fn test_ensure_kilocode_directory() {
        let temp_dir = TempDir::new().unwrap();
        
        ensure_kilocode_directory(temp_dir.path()).await.unwrap();
        
        let kilocode_dir = temp_dir.path().join(".kilocode");
        assert!(kilocode_dir.exists());
        assert!(kilocode_dir.is_dir());
    }
    
    #[tokio::test]
    async fn test_ensure_kilocode_directory_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let kilocode_dir = temp_dir.path().join(".kilocode");
        fs::create_dir(&kilocode_dir).await.unwrap();
        
        // Should not error if already exists
        ensure_kilocode_directory(temp_dir.path()).await.unwrap();
        assert!(kilocode_dir.exists());
    }
    
    #[test]
    fn test_get_system_prompt_file_path() {
        let workspace = Path::new("/home/user/project");
        let path = get_system_prompt_file_path(workspace, "architect");
        
        assert_eq!(
            path,
            PathBuf::from("/home/user/project/.kilocode/system-prompt-architect")
        );
    }
}
