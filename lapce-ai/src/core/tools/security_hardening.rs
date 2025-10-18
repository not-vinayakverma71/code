// Security Hardening System - Production-grade protection
// Part of Security hardening TODO #9 - pre-IPC

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use anyhow::{Result, bail, Context};
use serde::{Serialize, Deserialize};
use parking_lot::RwLock;
use std::sync::Arc;
use regex::Regex;

// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub workspace_root: PathBuf,
    pub enable_path_traversal_check: bool,
    pub enable_workspace_boundary: bool,
    pub enable_command_filtering: bool,
    pub enable_secrets_scanning: bool,
    pub max_path_depth: usize,
    pub strict_mode: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            enable_path_traversal_check: true,
            enable_workspace_boundary: true,
            enable_command_filtering: true,
            enable_secrets_scanning: true,
            max_path_depth: 100,
            strict_mode: true,
        }
    }
}

// Command security rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPolicy {
    pub denied_commands: HashSet<String>,
    pub allowed_commands: HashSet<String>,
    pub dangerous_patterns: Vec<String>,
    pub require_trash_put: bool,
    pub sandbox_mode: bool,
}

impl Default for CommandPolicy {
    fn default() -> Self {
        let mut denied = HashSet::new();
        denied.insert("rm".to_string());
        denied.insert("rmdir".to_string());
        denied.insert("del".to_string());
        denied.insert("format".to_string());
        denied.insert("fdisk".to_string());
        denied.insert("dd".to_string());
        denied.insert("mkfs".to_string());
        denied.insert("shutdown".to_string());
        denied.insert("reboot".to_string());
        denied.insert("kill".to_string());
        denied.insert("killall".to_string());
        
        let dangerous_patterns = vec![
            r"rm\s+-rf".to_string(),
            r">\s*/dev/".to_string(),
            r"fork\s*bomb".to_string(),
            r":\(\)\{.*\}:".to_string(),
            r"sudo\s+rm".to_string(),
            r"chmod\s+777".to_string(),
            r"curl.*\|\s*sh".to_string(),
            r"wget.*\|\s*bash".to_string(),
        ];
        
        Self {
            denied_commands: denied,
            allowed_commands: HashSet::new(),
            dangerous_patterns,
            require_trash_put: true,
            sandbox_mode: false,
        }
    }
}

// Path security validator
pub struct PathValidator {
    workspace_root: PathBuf,
    config: SecurityConfig,
}

impl PathValidator {
    pub fn new(config: SecurityConfig) -> Result<Self> {
        let workspace_root = config.workspace_root.canonicalize()
            .context("Failed to canonicalize workspace root")?;
        
        Ok(Self {
            workspace_root,
            config,
        })
    }
    
    /// Check for path traversal attempts
    pub fn check_path_traversal(&self, path: &Path) -> Result<()> {
        if !self.config.enable_path_traversal_check {
            return Ok(());
        }
        
        let path_str = path.to_string_lossy();
        
        // Check for traversal patterns
        if path_str.contains("../") || path_str.contains("..\\") {
            bail!("Path traversal detected: {}", path_str);
        }
        
        // Check for absolute paths outside workspace
        if path.is_absolute() && !path.starts_with(&self.workspace_root) {
            bail!("Absolute path outside workspace: {}", path_str);
        }
        
        // Check for symlink traversal
        if let Ok(canonical) = path.canonicalize() {
            if !canonical.starts_with(&self.workspace_root) {
                bail!("Path resolves outside workspace: {}", canonical.display());
            }
        }
        
        // Check path depth
        let depth = path.components().count();
        if depth > self.config.max_path_depth {
            bail!("Path too deep: {} levels (max {})", depth, self.config.max_path_depth);
        }
        
        Ok(())
    }
    
    /// Enforce workspace boundaries
    pub fn enforce_workspace_boundary(&self, path: &Path) -> Result<PathBuf> {
        if !self.config.enable_workspace_boundary {
            return Ok(path.to_path_buf());
        }
        
        // Convert to absolute path
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };
        
        // Canonicalize and check
        let canonical = abs_path.canonicalize()
            .unwrap_or(abs_path.clone());
        
        if !canonical.starts_with(&self.workspace_root) {
            bail!("Path outside workspace boundary: {}", canonical.display());
        }
        
        Ok(canonical)
    }
    
    /// Validate multiple paths
    pub fn validate_paths(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        paths.iter()
            .map(|p| {
                self.check_path_traversal(p)?;
                self.enforce_workspace_boundary(p)
            })
            .collect()
    }
}

// Command security validator
pub struct CommandValidator {
    policy: CommandPolicy,
    patterns: Vec<Regex>,
}

impl CommandValidator {
    pub fn new(policy: CommandPolicy) -> Result<Self> {
        let patterns = policy.dangerous_patterns.iter()
            .map(|p| Regex::new(p))
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to compile command patterns")?;
        
        Ok(Self {
            policy,
            patterns,
        })
    }
    
    /// Validate command for execution
    pub fn validate_command(&self, command: &str) -> Result<String> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Empty command");
        }
        
        let base_command = parts[0];
        
        // Check denied list
        if self.policy.denied_commands.contains(base_command) {
            if self.policy.require_trash_put && (base_command == "rm" || base_command == "rmdir") {
                bail!(
                    "Command '{}' is denied for safety. Use 'trash-put' instead for safe deletion.",
                    base_command
                );
            } else {
                bail!("Command '{}' is denied by security policy", base_command);
            }
        }
        
        // Check allowed list if not empty
        if !self.policy.allowed_commands.is_empty() 
            && !self.policy.allowed_commands.contains(base_command) {
            bail!("Command '{}' is not in allowed list", base_command);
        }
        
        // Check dangerous patterns
        for pattern in &self.patterns {
            if pattern.is_match(command) {
                bail!("Command matches dangerous pattern: {}", pattern.as_str());
            }
        }
        
        // Suggest safer alternatives
        if command.contains("rm ") && !command.contains("trash-put") {
            return Ok(self.suggest_safe_alternative(command));
        }
        
        Ok(command.to_string())
    }
    
    fn suggest_safe_alternative(&self, command: &str) -> String {
        command.replace("rm ", "trash-put ")
            .replace("rm -r ", "trash-put ")
            .replace("rm -rf ", "trash-put ")
            .replace("rmdir ", "trash-put ")
    }
}

// Secrets scanner
pub struct SecretsScanner {
    patterns: Vec<(Regex, String)>,
}

impl SecretsScanner {
    pub fn new() -> Result<Self> {
        let patterns = vec![
            (r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?([a-zA-Z0-9_-]{20,})['"]?"#, "API Key"),
            (r#"(?i)(secret|password|passwd|pwd)\s*[:=]\s*['"]?([^\s"']{8,})['"]?"#, "Password/Secret"),
            (r#"(?i)aws[_-]?access[_-]?key[_-]?id\s*[:=]\s*['"]?([A-Z0-9]{20})['"]?"#, "AWS Access Key"),
            (r#"(?i)aws[_-]?secret[_-]?access[_-]?key\s*[:=]\s*['"]?([a-zA-Z0-9/+=]{40})['"]?"#, "AWS Secret Key"),
            (r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Token"),
            (r"ghs_[a-zA-Z0-9]{36}", "GitHub Secret"),
            (r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----", "Private Key"),
            (r"(?i)bearer\s+[a-zA-Z0-9_\-\.=]+", "Bearer Token"),
            (r"(?i)(mongodb|postgres|mysql|redis)://[^\s]+", "Database Connection String"),
        ];
        
        let compiled = patterns.into_iter()
            .map(|(pattern, name)| {
                Regex::new(pattern)
                    .map(|r| (r, name.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to compile secret patterns")?;
        
        Ok(Self {
            patterns: compiled,
        })
    }
    
    /// Scan content for secrets
    pub fn scan_content(&self, content: &str) -> Vec<SecretMatch> {
        let mut matches = Vec::new();
        
        for (pattern, secret_type) in &self.patterns {
            for mat in pattern.find_iter(content) {
                matches.push(SecretMatch {
                    secret_type: secret_type.clone(),
                    position: mat.start(),
                    length: mat.len(),
                    masked_value: Self::mask_secret(mat.as_str()),
                    line_number: content[..mat.start()].lines().count(),
                });
            }
        }
        
        matches
    }
    
    /// Scan file for secrets
    pub fn scan_file(&self, path: &Path) -> Result<Vec<SecretMatch>> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read file for scanning")?;
        
        Ok(self.scan_content(&content))
    }
    
    fn mask_secret(value: &str) -> String {
        if value.len() <= 8 {
            "*".repeat(value.len())
        } else {
            format!("{}...{}", &value[..4], "*".repeat(4))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMatch {
    pub secret_type: String,
    pub position: usize,
    pub length: usize,
    pub masked_value: String,
    pub line_number: usize,
}

// Unified security manager
pub struct SecurityManager {
    path_validator: Arc<PathValidator>,
    command_validator: Arc<CommandValidator>,
    secrets_scanner: Arc<SecretsScanner>,
    config: SecurityConfig,
}

impl SecurityManager {
    pub fn new(config: SecurityConfig) -> Result<Self> {
        Ok(Self {
            path_validator: Arc::new(PathValidator::new(config.clone())?),
            command_validator: Arc::new(CommandValidator::new(CommandPolicy::default())?),
            secrets_scanner: Arc::new(SecretsScanner::new()?),
            config,
        })
    }
    
    /// Validate path security
    pub fn validate_path(&self, path: &Path) -> Result<PathBuf> {
        self.path_validator.check_path_traversal(path)?;
        self.path_validator.enforce_workspace_boundary(path)
    }
    
    /// Validate command security
    pub fn validate_command(&self, command: &str) -> Result<String> {
        self.command_validator.validate_command(command)
    }
    
    /// Check for secrets
    pub fn check_for_secrets(&self, content: &str) -> Result<()> {
        if !self.config.enable_secrets_scanning {
            return Ok(());
        }
        
        let matches = self.secrets_scanner.scan_content(content);
        if !matches.is_empty() {
            bail!(
                "Potential secrets detected: {}",
                matches.iter()
                    .map(|m| format!("{} at line {}", m.secret_type, m.line_number))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        
        Ok(())
    }
}

// Global security manager
lazy_static::lazy_static! {
    static ref SECURITY_MANAGER: Arc<RwLock<Option<SecurityManager>>> = 
        Arc::new(RwLock::new(None));
}

pub fn init_security(config: SecurityConfig) -> Result<()> {
    let manager = SecurityManager::new(config)?;
    *SECURITY_MANAGER.write() = Some(manager);
    Ok(())
}

pub fn validate_path_security(path: &Path) -> Result<PathBuf> {
    if let Some(ref manager) = *SECURITY_MANAGER.read() {
        manager.validate_path(path)
    } else {
        Ok(path.to_path_buf())
    }
}

pub fn validate_command_security(command: &str) -> Result<String> {
    if let Some(ref manager) = *SECURITY_MANAGER.read() {
        manager.validate_command(command)
    } else {
        // Fallback to default validation when manager not initialized
        let policy = CommandPolicy::default();
        let validator = CommandValidator::new(policy)?;
        validator.validate_command(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_path_traversal_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = SecurityConfig {
            workspace_root: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let validator = PathValidator::new(config).unwrap();
        
        // Test traversal patterns
        assert!(validator.check_path_traversal(Path::new("../etc/passwd")).is_err());
        assert!(validator.check_path_traversal(Path::new("../../root")).is_err());
        assert!(validator.check_path_traversal(Path::new("normal/path")).is_ok());
    }
    
    #[test]
    fn test_command_validation() {
        let validator = CommandValidator::new(CommandPolicy::default()).unwrap();
        
        // Test denied commands
        assert!(validator.validate_command("rm -rf /").is_err());
        assert!(validator.validate_command("dd if=/dev/zero of=/dev/sda").is_err());
        
        // Test safe alternatives
        let result = validator.validate_command("rm file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("trash-put"));
    }
    
    #[test]
    fn test_secrets_scanning() {
        let scanner = SecretsScanner::new().unwrap();
        
        let content = r#"
            API_KEY=sk-1234567890abcdef1234567890abcdef
            password: mysecretpassword123
            aws_access_key_id = AKIAIOSFODNN7EXAMPLE
            ghp_1234567890abcdef1234567890abcdef1234
        "#;
        
        let matches = scanner.scan_content(content);
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.secret_type.contains("API Key")));
        assert!(matches.iter().any(|m| m.secret_type.contains("GitHub")));
    }
}
