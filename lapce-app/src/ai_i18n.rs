// i18n Translation Provider for AI Chat
// Phase 0.4: Minimal foundation, will expand in later phases

use std::collections::HashMap;
use std::sync::Arc;

use floem::reactive::{RwSignal, SignalGet, create_rw_signal};

// ============================================================================
// Translation Provider
// ============================================================================

#[derive(Clone)]
pub struct TranslationProvider {
    current_lang: RwSignal<String>,
    translations: Arc<HashMap<String, LanguageBundle>>,
}

impl TranslationProvider {
    pub fn new(lang: String) -> Self {
        let translations = Self::load_translations();
        
        Self {
            current_lang: create_rw_signal(lang),
            translations: Arc::new(translations),
        }
    }
    
    /// Translate a key with optional interpolation values
    pub fn t(&self, key: &str) -> String {
        let lang = self.current_lang.get();
        
        // Try current language
        if let Some(bundle) = self.translations.get(&lang) {
            if let Some(text) = bundle.get(key) {
                return text.clone();
            }
        }
        
        // Fallback to English
        if lang != "en" {
            if let Some(bundle) = self.translations.get("en") {
                if let Some(text) = bundle.get(key) {
                    return text.clone();
                }
            }
        }
        
        // Last resort: return key itself
        key.to_string()
    }
    
    /// Change current language  
    #[allow(dead_code)]
    pub fn set_language(&self, lang: String) {
        // For now, language is fixed at initialization
        // TODO: Implement language switching when needed
        let _ = lang;
    }
    
    /// Load all translation bundles
    fn load_translations() -> HashMap<String, LanguageBundle> {
        let mut translations = HashMap::new();
        
        // English bundle (minimal for Phase 0)
        let mut en = HashMap::new();
        
        // Chat keys
        en.insert("chat.placeholder".to_string(), "Type a message...".to_string());
        en.insert("chat.send".to_string(), "Send".to_string());
        en.insert("chat.taskCompleted".to_string(), "Task Completed".to_string());
        
        // Connection keys
        en.insert("connection.disconnected".to_string(), "Disconnected - Backend not available".to_string());
        en.insert("connection.connecting".to_string(), "Connecting...".to_string());
        en.insert("connection.connected".to_string(), "Connected".to_string());
        en.insert("connection.retry".to_string(), "Retry Connection".to_string());
        
        // Tool keys (minimal, will expand in Phase 3)
        en.insert("tools.readFile".to_string(), "wants to read".to_string());
        en.insert("tools.writeFile".to_string(), "wants to write to".to_string());
        en.insert("tools.searchFiles".to_string(), "wants to search for".to_string());
        en.insert("tools.executeCommand".to_string(), "wants to execute".to_string());
        
        // Approval keys
        en.insert("approval.approve".to_string(), "Yes".to_string());
        en.insert("approval.reject".to_string(), "No".to_string());
        en.insert("approval.message".to_string(), "Message".to_string());
        
        // Settings keys (minimal, will expand in Phase 6)
        en.insert("settings.title".to_string(), "AI Chat Settings".to_string());
        en.insert("settings.autoApproval".to_string(), "Auto-Approval".to_string());
        en.insert("settings.sounds".to_string(), "Sounds".to_string());
        
        translations.insert("en".to_string(), en);
        
        // TODO Phase 0.4+: Load additional languages from JSON files
        // For now, English only
        
        translations
    }
}

impl Default for TranslationProvider {
    fn default() -> Self {
        Self::new("en".to_string())
    }
}

type LanguageBundle = HashMap<String, String>;

// ============================================================================
// Helper macro for translation (Phase 1+)
// ============================================================================

// TODO: Add t! macro in Phase 1 when we integrate with UI components
// macro_rules! t {
//     ($provider:expr, $key:expr) => {
//         $provider.t($key)
//     };
// }
