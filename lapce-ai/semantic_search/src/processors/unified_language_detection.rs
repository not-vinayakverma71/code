// Unified language detection system
// Integrates language_registry with CST LanguageRegistry (when cst_ts feature is enabled)

use crate::error::{Error, Result};
use crate::processors::language_registry;
use std::path::Path;

#[cfg(feature = "cst_ts")]
use lapce_tree_sitter::language::registry::LanguageRegistry;

/// Detect language from file path using extension
pub fn detect_language(path: &Path) -> Result<String> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| Error::Runtime {
            message: format!("No file extension found for: {}", path.display())
        })?;
    
    // First try language_registry (fast, no external dependencies)
    if let Some(lang_name) = language_registry::get_language_by_extension(ext) {
        return Ok(lang_name.to_string());
    }
    
    // If cst_ts feature enabled, try CST LanguageRegistry
    #[cfg(feature = "cst_ts")]
    {
        let registry = LanguageRegistry::instance();
        if let Ok(lang_info) = registry.by_extension(ext) {
            return Ok(lang_info.name.to_string());
        }
    }
    
    Err(Error::Runtime {
        message: format!("Unknown language for extension: {}", ext)
    })
}

/// Get language by name, validates it exists
pub fn get_language_info(name: &str) -> Result<LanguageInfo> {
    // Check language_registry first
    for lang in language_registry::get_all_languages() {
        if lang.name == name {
            return Ok(LanguageInfo {
                name: lang.name.to_string(),
                extensions: lang.extensions.iter().map(|s| s.to_string()).collect(),
                is_core: lang.is_core,
                is_available: true,
            });
        }
    }
    
    // If cst_ts feature enabled, check CST LanguageRegistry
    #[cfg(feature = "cst_ts")]
    {
        let registry = LanguageRegistry::instance();
        if let Ok(lang_info) = registry.by_name(name) {
            return Ok(LanguageInfo {
                name: lang_info.name.to_string(),
                extensions: vec![], // CST doesn't provide extensions directly
                is_core: false, // Assume external if not in language_registry
                is_available: true,
            });
        }
    }
    
    Err(Error::Runtime {
        message: format!("Language not found: {}", name)
    })
}

/// Information about a supported language
#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
    pub is_core: bool,
    pub is_available: bool,
}

/// Get all supported languages
pub fn get_all_supported_languages() -> Vec<LanguageInfo> {
    let mut languages = Vec::new();
    
    // Get all from language_registry
    for lang in language_registry::get_all_languages() {
        languages.push(LanguageInfo {
            name: lang.name.to_string(),
            extensions: lang.extensions.iter().map(|s| s.to_string()).collect(),
            is_core: lang.is_core,
            is_available: true,
        });
    }
    
    // Note: CST LanguageRegistry contains the same languages, so no need to query it separately
    // since language_registry is the source of truth for our 67 languages
    
    languages
}

/// Check if a language is supported
pub fn is_language_supported(name: &str) -> bool {
    get_language_info(name).is_ok()
}

/// Get language statistics
pub fn get_language_stats() -> LanguageStats {
    let (core_count, external_count) = language_registry::get_language_count();
    
    LanguageStats {
        total: core_count + external_count,
        core: core_count,
        external: external_count,
    }
}

#[derive(Debug, Clone)]
pub struct LanguageStats {
    pub total: usize,
    pub core: usize,
    pub external: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_detect_language() {
        let test_cases = vec![
            ("test.rs", "rust"),
            ("test.js", "javascript"),
            ("test.py", "python"),
            ("test.go", "go"),
            ("test.java", "java"),
        ];
        
        for (filename, expected) in test_cases {
            let path = PathBuf::from(filename);
            let detected = detect_language(&path).unwrap();
            assert_eq!(detected, expected, "Failed for {}", filename);
        }
    }
    
    #[test]
    fn test_get_language_info() {
        let info = get_language_info("rust").unwrap();
        assert_eq!(info.name, "rust");
        assert!(info.is_core);
        assert!(info.is_available);
        assert!(info.extensions.contains(&"rs".to_string()));
    }
    
    #[test]
    fn test_language_stats() {
        let stats = get_language_stats();
        assert_eq!(stats.total, 67);
        assert_eq!(stats.core, 31);
        assert_eq!(stats.external, 36);
    }
    
    #[test]
    fn test_is_language_supported() {
        assert!(is_language_supported("rust"));
        assert!(is_language_supported("javascript"));
        assert!(is_language_supported("python"));
        assert!(!is_language_supported("nonexistent"));
    }
    
    #[test]
    fn test_get_all_supported_languages() {
        let languages = get_all_supported_languages();
        assert_eq!(languages.len(), 67);
        
        // Check some key languages
        assert!(languages.iter().any(|l| l.name == "rust"));
        assert!(languages.iter().any(|l| l.name == "javascript"));
        assert!(languages.iter().any(|l| l.name == "python"));
    }
}
