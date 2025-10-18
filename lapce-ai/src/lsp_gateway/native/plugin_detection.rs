/// Plugin Conflict Detection (LSP-028)
/// Detect and disable Lapce plugin LSP when native gateway is active

use std::sync::Arc;
use std::collections::HashSet;
use parking_lot::RwLock;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};

/// LSP source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LspSource {
    /// Native gateway (this crate)
    NativeGateway,
    /// Lapce plugin system
    LapcePlugin,
    /// External LSP server
    ExternalServer,
}

/// Language server registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRegistration {
    pub language_id: String,
    pub source: LspSource,
    pub capabilities: Vec<String>,
    pub priority: i32, // Higher = preferred
}

/// Plugin conflict detector
pub struct PluginConflictDetector {
    active_servers: Arc<RwLock<Vec<LspRegistration>>>,
    native_gateway_enabled: bool,
}

impl PluginConflictDetector {
    pub fn new(native_gateway_enabled: bool) -> Self {
        Self {
            active_servers: Arc::new(RwLock::new(Vec::new())),
            native_gateway_enabled,
        }
    }
    
    /// Register an LSP server
    pub fn register_server(&self, registration: LspRegistration) -> Result<()> {
        let mut servers = self.active_servers.write();
        
        // Check for conflicts
        if self.native_gateway_enabled && registration.source == LspSource::LapcePlugin {
            // Check if native gateway handles this language
            if self.is_language_supported_natively(&registration.language_id) {
                tracing::warn!(
                    language = %registration.language_id,
                    source = ?registration.source,
                    "Blocking Lapce plugin LSP registration - native gateway is active"
                );
                return Err(anyhow!(
                    "Native LSP gateway is active for language '{}', plugin LSP disabled",
                    registration.language_id
                ));
            }
        }
        
        // Remove existing registration for this language/source
        servers.retain(|s| !(s.language_id == registration.language_id && s.source == registration.source));
        
        servers.push(registration.clone());
        
        tracing::info!(
            language = %registration.language_id,
            source = ?registration.source,
            priority = registration.priority,
            "Registered LSP server"
        );
        
        Ok(())
    }
    
    /// Unregister an LSP server
    pub fn unregister_server(&self, language_id: &str, source: LspSource) {
        let mut servers = self.active_servers.write();
        servers.retain(|s| !(s.language_id == language_id && s.source == source));
        
        tracing::info!(
            language = %language_id,
            source = ?source,
            "Unregistered LSP server"
        );
    }
    
    /// Get active server for a language
    pub fn get_active_server(&self, language_id: &str) -> Option<LspRegistration> {
        let servers = self.active_servers.read();
        
        // Find all servers for this language
        let mut candidates: Vec<_> = servers
            .iter()
            .filter(|s| s.language_id == language_id)
            .collect();
        
        if candidates.is_empty() {
            return None;
        }
        
        // Sort by priority (highest first)
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        Some(candidates[0].clone())
    }
    
    /// Check if language is supported by native gateway
    fn is_language_supported_natively(&self, language_id: &str) -> bool {
        // This list matches the 68 languages in CST-tree-sitter LanguageRegistry
        const NATIVE_LANGUAGES: &[&str] = &[
            // 30 core languages
            "rust", "python", "go", "java", "c", "cpp", "csharp", "ruby", "php", 
            "lua", "bash", "css", "json", "swift", "scala", "elixir", "html", 
            "ocaml", "nix", "make", "cmake", "verilog", "erlang", "d", "pascal", 
            "commonlisp", "objc", "groovy", "embedded_template", "sh",
            // 38 external languages (when features enabled)
            "javascript", "typescript", "tsx", "toml", "dockerfile", "elm", 
            "kotlin", "yaml", "r", "matlab", "perl", "dart", "julia", "haskell", 
            "graphql", "sql", "zig", "vim", "abap", "nim", "clojure", "crystal", 
            "fortran", "vhdl", "racket", "ada", "prolog", "gradle", "xml", 
            "markdown", "svelte", "scheme", "fennel", "gleam", "hcl", "solidity", 
            "fsharp", "cobol", "systemverilog", "md",
        ];
        
        NATIVE_LANGUAGES.contains(&language_id)
    }
    
    /// List all active servers
    pub fn list_active_servers(&self) -> Vec<LspRegistration> {
        self.active_servers.read().clone()
    }
    
    /// Get conflict report
    pub fn get_conflict_report(&self) -> ConflictReport {
        let servers = self.active_servers.read();
        
        let mut conflicts = Vec::new();
        let mut by_language: std::collections::HashMap<String, Vec<&LspRegistration>> = 
            std::collections::HashMap::new();
        
        // Group by language
        for server in servers.iter() {
            by_language.entry(server.language_id.clone())
                .or_insert_with(Vec::new)
                .push(server);
        }
        
        // Find conflicts (multiple servers for same language)
        for (language, servers) in by_language.iter() {
            if servers.len() > 1 {
                conflicts.push(LanguageConflict {
                    language_id: language.clone(),
                    servers: servers.iter().map(|s| (*s).clone()).collect(),
                    resolved_source: servers.iter()
                        .max_by_key(|s| s.priority)
                        .map(|s| s.source),
                });
            }
        }
        
        ConflictReport {
            total_servers: servers.len(),
            conflicts,
            native_gateway_enabled: self.native_gateway_enabled,
        }
    }
    
    /// Disable all plugin LSP servers
    pub fn disable_all_plugin_servers(&self) {
        let mut servers = self.active_servers.write();
        let before_count = servers.len();
        servers.retain(|s| s.source != LspSource::LapcePlugin);
        let removed = before_count - servers.len();
        
        if removed > 0 {
            tracing::info!(
                removed = removed,
                "Disabled all Lapce plugin LSP servers"
            );
        }
    }
}

/// Language conflict details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConflict {
    pub language_id: String,
    pub servers: Vec<LspRegistration>,
    pub resolved_source: Option<LspSource>,
}

/// Conflict report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictReport {
    pub total_servers: usize,
    pub conflicts: Vec<LanguageConflict>,
    pub native_gateway_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_native_language_detection() {
        let detector = PluginConflictDetector::new(true);
        
        assert!(detector.is_language_supported_natively("rust"));
        assert!(detector.is_language_supported_natively("python"));
        assert!(detector.is_language_supported_natively("typescript"));
        assert!(!detector.is_language_supported_natively("unknown"));
    }
    
    #[test]
    fn test_plugin_blocking() {
        let detector = PluginConflictDetector::new(true);
        
        // Try to register plugin LSP for Rust
        let result = detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::LapcePlugin,
            capabilities: vec!["hover".to_string()],
            priority: 10,
        });
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_native_gateway_registration() {
        let detector = PluginConflictDetector::new(true);
        
        let result = detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::NativeGateway,
            capabilities: vec!["hover".to_string(), "definition".to_string()],
            priority: 100,
        });
        
        assert!(result.is_ok());
        
        let active = detector.get_active_server("rust");
        assert!(active.is_some());
        assert_eq!(active.unwrap().source, LspSource::NativeGateway);
    }
    
    #[test]
    fn test_priority_resolution() {
        let detector = PluginConflictDetector::new(false); // Allow plugins
        
        // Register low priority plugin
        detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::LapcePlugin,
            capabilities: vec!["hover".to_string()],
            priority: 10,
        }).unwrap();
        
        // Register high priority native
        detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::NativeGateway,
            capabilities: vec!["hover".to_string(), "definition".to_string()],
            priority: 100,
        }).unwrap();
        
        let active = detector.get_active_server("rust").unwrap();
        assert_eq!(active.source, LspSource::NativeGateway);
    }
    
    #[test]
    fn test_conflict_report() {
        let detector = PluginConflictDetector::new(false);
        
        detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::LapcePlugin,
            capabilities: vec!["hover".to_string()],
            priority: 10,
        }).unwrap();
        
        detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::NativeGateway,
            capabilities: vec!["hover".to_string()],
            priority: 100,
        }).unwrap();
        
        let report = detector.get_conflict_report();
        assert_eq!(report.total_servers, 2);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].language_id, "rust");
    }
    
    #[test]
    fn test_disable_all_plugins() {
        let detector = PluginConflictDetector::new(false);
        
        detector.register_server(LspRegistration {
            language_id: "rust".to_string(),
            source: LspSource::LapcePlugin,
            capabilities: vec![],
            priority: 10,
        }).unwrap();
        
        detector.register_server(LspRegistration {
            language_id: "python".to_string(),
            source: LspSource::NativeGateway,
            capabilities: vec![],
            priority: 100,
        }).unwrap();
        
        detector.disable_all_plugin_servers();
        
        let servers = detector.list_active_servers();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].source, LspSource::NativeGateway);
    }
}
