# CHUNK-20: I18N - INTERNATIONALIZATION SYSTEM

## 📁 MODULE STRUCTURE

```
Codex/src/i18n/
├── index.ts                  (42 lines)   - Public API
├── setup.ts                  (74 lines)   - i18next configuration
└── locales/                  (21 languages)
    ├── ar/                   - Arabic
    ├── ca/                   - Catalan
    ├── cs/                   - Czech
    ├── de/                   - German
    ├── en/                   - English (reference)
    │   ├── common.json       (223 lines) - General translations
    │   ├── embeddings.json   - Embeddings UI
    │   ├── kilocode.json     (138 lines) - Kilocode features
    │   ├── marketplace.json  - Extension marketplace
    │   ├── mcp.json          - MCP protocol
    │   └── tools.json        (19 lines)  - Tool messages
    ├── es/                   - Spanish
    ├── fr/                   - French
    ├── hi/                   - Hindi
    ├── id/                   - Indonesian
    ├── it/                   - Italian
    ├── ja/                   - Japanese
    ├── ko/                   - Korean
    ├── nl/                   - Dutch
    ├── pl/                   - Polish
    ├── pt-BR/                - Brazilian Portuguese
    ├── ru/                   - Russian
    ├── th/                   - Thai
    ├── tr/                   - Turkish
    ├── uk/                   - Ukrainian
    ├── vi/                   - Vietnamese
    ├── zh-CN/                - Simplified Chinese
    └── zh-TW/                - Traditional Chinese
```

**Total**: 116 lines of TypeScript + ~2,000+ lines of translations (21 languages × 6 namespaces)

---

## 🎯 PURPOSE

Provide multi-language support for the entire extension:
1. **UI Localization**: Translate all user-facing text
2. **Error Messages**: Localized error descriptions
3. **Namespace Organization**: Separate translations by feature area
4. **Interpolation**: Dynamic values in translations
5. **Pluralization**: Correct grammar for counts
6. **Fallback**: English as default for missing translations

**Library**: Uses `i18next` (industry-standard i18n framework)

---

## 🔧 CORE API: index.ts (42 lines)

### Public Functions

```typescript
import i18next from "./setup"

/**
 * Initialize i18next with the specified language
 */
export function initializeI18n(language: string): void {
    i18next.changeLanguage(language)
}

/**
 * Get the current language
 */
export function getCurrentLanguage(): string {
    return i18next.language
}

/**
 * Change the current language
 */
export function changeLanguage(language: string): void {
    i18next.changeLanguage(language)
}

/**
 * Translate a string using i18next
 * 
 * @param key - Translation key with namespace: "namespace:path.to.key"
 * @param options - Interpolation or pluralization options
 */
export function t(key: string, options?: Record<string, any>): string {
    return i18next.t(key, options)
}

export default i18next
```

### Usage Examples

```typescript
// Simple translation
t("common:welcome")  // "Welcome, {{name}}! You have {{count}} notifications."

// With interpolation
t("common:welcome", { name: "John", count: 5 })
// "Welcome, John! You have 5 notifications."

// Nested keys
t("common:errors.invalid_data_uri")
// "Invalid data URI format"

// Pluralization
t("common:items", { count: 0 })  // "No items"
t("common:items", { count: 1 })  // "One item"
t("common:items", { count: 5 })  // "5 items"

// Tool-specific translations
t("tools:readFile.linesRange", { start: 10, end: 50 })
// " (lines 10-50)"

// Error messages
t("kilocode:userFeedback.message_update_failed")
// "Failed to update message"
```

---

## ⚙️ SETUP & CONFIGURATION: setup.ts (74 lines)

### Translation Loading Strategy

```typescript
import i18next from "i18next"

// Build translations object
const translations: Record<string, Record<string, any>> = {}

// Determine if running in test environment
const isTestEnv = process.env.NODE_ENV === "test"

// Load translations based on environment
if (!isTestEnv) {
    try {
        // Dynamic imports (Node.js only)
        const fs = require("fs")
        const path = require("path")
        
        const localesDir = path.join(__dirname, "i18n", "locales")
        
        // Find all language directories
        const languageDirs = fs.readdirSync(localesDir, { withFileTypes: true })
        
        const languages = languageDirs
            .filter(dirent => dirent.isDirectory())
            .map(dirent => dirent.name)
        
        // Process each language
        languages.forEach(language => {
            const langPath = path.join(localesDir, language)
            
            // Find all JSON files in language directory
            const files = fs.readdirSync(langPath)
                .filter(file => file.endsWith(".json"))
            
            // Initialize language in translations object
            if (!translations[language]) {
                translations[language] = {}
            }
            
            // Process each namespace file
            files.forEach(file => {
                const namespace = path.basename(file, ".json")
                const filePath = path.join(langPath, file)
                
                try {
                    // Read and parse JSON file
                    const content = fs.readFileSync(filePath, "utf8")
                    translations[language][namespace] = JSON.parse(content)
                } catch (error) {
                    console.error(`Error loading translation file ${filePath}:`, error)
                }
            })
        })
        
        console.log(`Loaded translations for languages: ${Object.keys(translations).join(", ")}`)
    } catch (dirError) {
        console.error(`Error processing directory ${localesDir}:`, dirError)
    }
} catch (error) {
    console.error("Error loading translations:", error)
}

// Initialize i18next with configuration
i18next.init({
    lng: "en",              // Default language
    fallbackLng: "en",      // Fallback if key not found
    debug: false,           // Disable debug logging
    resources: translations,
    interpolation: {
        escapeValue: false, // React already escapes
    },
})

export default i18next
```

### Key Design Decisions

**1. Runtime Loading**
- Reads all JSON files at extension startup
- Builds in-memory translations object
- No compilation step needed

**2. Directory Structure = Language Code**
```
locales/en/  → translations["en"]
locales/fr/  → translations["fr"]
locales/ja/  → translations["ja"]
```

**3. Filename = Namespace**
```
common.json  → translations[lang]["common"]
tools.json   → translations[lang]["tools"]
```

**4. Test Environment Handling**
```typescript
const isTestEnv = process.env.NODE_ENV === "test"
if (!isTestEnv) {
    // Load translations from filesystem
}
```
- Skips file I/O in tests
- Tests can mock translations
- Faster test execution

---

## 📄 TRANSLATION FILE STRUCTURE

### Namespace: common.json (223 lines)

**General translations, errors, confirmations**

```json
{
    "extension": {
        "name": "Kilo Code",
        "description": "Open Source AI coding assistant..."
    },
    "number_format": {
        "thousand_suffix": "k",
        "million_suffix": "m",
        "billion_suffix": "b"
    },
    "feedback": {
        "title": "Feedback",
        "description": "We'd love to hear your feedback...",
        "githubIssues": "Report an issue on GitHub",
        "discord": "Join our Discord community"
    },
    "welcome": "Welcome, {{name}}! You have {{count}} notifications.",
    "items": {
        "zero": "No items",
        "one": "One item",
        "other": "{{count}} items"
    },
    "confirmation": {
        "reset_state": "Are you sure you want to reset all state...",
        "delete_config_profile": "Are you sure you want to delete this profile?"
    },
    "errors": {
        "invalid_data_uri": "Invalid data URI format",
        "checkpoint_timeout": "Timed out when attempting to restore checkpoint.",
        "checkpoint_failed": "Failed to restore checkpoint.",
        "git_not_installed": "Git is required for checkpoints...",
        "no_workspace": "Please open a project folder first",
        "condense_failed": "Failed to condense context",
        "url_timeout": "The website took too long to load...",
        "command_timeout": "Command execution timed out after {{seconds}} seconds"
    }
}
```

### Namespace: tools.json (19 lines)

**Tool execution messages**

```json
{
    "readFile": {
        "linesRange": " (lines {{start}}-{{end}})",
        "definitionsOnly": " (definitions only)",
        "maxLines": " (max {{max}} lines)",
        "imageTooLarge": "Image file is too large ({{size}} MB)...",
        "imageWithSize": "Image file ({{size}} KB)"
    },
    "toolRepetitionLimitReached": "Kilo Code appears to be stuck in a loop...",
    "codebaseSearch": {
        "approval": "Searching for '{{query}}' in codebase..."
    },
    "newTask": {
        "errors": {
            "policy_restriction": "Failed to create new task due to policy restrictions."
        }
    }
}
```

### Namespace: kilocode.json (138 lines)

**Kilocode-specific features**

```json
{
    "userFeedback": {
        "message_update_failed": "Failed to update message",
        "no_checkpoint_found": "No checkpoint found before this message",
        "message_updated": "Message updated successfully"
    },
    "checkpoints": {
        "nestedGitRepos": "Checkpoints are unavailable, because nested Git repositories...",
        "protectedPaths": "Checkpoints are unavailable in {{workspaceDir}}..."
    },
    "commitMessage": {
        "activated": "Kilo Code commit message generator activated",
        "gitNotFound": "⚠️ Git repository not found...",
        "generating": "Kilo: Generating commit message...",
        "noChanges": "Kilo: No changes found to analyze"
    },
    "ghost": {
        "statusBar": {
            "enabled": "$(sparkle) Kilo Code Autocomplete",
            "disabled": "$(circle-slash) Kilo Code Autocomplete"
        },
        "progress": {
            "title": "Kilo Code",
            "analyzing": "Analyzing your code...",
            "generating": "Generating suggested edits..."
        }
    },
    "terminalCommandGenerator": {
        "tipMessage": "Kilo: Press {{shortcut}} to generate terminal commands",
        "warningDialog": {
            "title": "Terminal Safety Warning",
            "message": "Double-check all generated commands before running them..."
        }
    }
}
```

---

## 🌍 SUPPORTED LANGUAGES

### Complete List (21 Languages)

| Code | Language | Region | Script |
|------|----------|--------|--------|
| `ar` | Arabic | - | RTL (Right-to-Left) |
| `ca` | Catalan | - | Latin |
| `cs` | Czech | - | Latin |
| `de` | German | - | Latin |
| `en` | English | - | Latin (Reference) |
| `es` | Spanish | - | Latin |
| `fr` | French | - | Latin |
| `hi` | Hindi | India | Devanagari |
| `id` | Indonesian | - | Latin |
| `it` | Italian | - | Latin |
| `ja` | Japanese | - | Kanji/Hiragana/Katakana |
| `ko` | Korean | - | Hangul |
| `nl` | Dutch | - | Latin |
| `pl` | Polish | - | Latin |
| `pt-BR` | Portuguese | Brazil | Latin |
| `ru` | Russian | - | Cyrillic |
| `th` | Thai | - | Thai script |
| `tr` | Turkish | - | Latin |
| `uk` | Ukrainian | - | Cyrillic |
| `vi` | Vietnamese | - | Latin (with diacritics) |
| `zh-CN` | Chinese | Simplified | Simplified Chinese |
| `zh-TW` | Chinese | Traditional | Traditional Chinese |

### Coverage Strategy

**Reference Language**: English (`en/`)
- All features documented here first
- Other languages translate from English
- Missing translations fall back to English

**Translation Workflow**:
1. Add new feature to `en/*.json`
2. Community contributes other languages
3. Automated tools detect missing keys
4. Fallback ensures no broken UI

---

## 🔄 INTEGRATION POINTS

### 1. Extension Activation

```typescript
// In extension.ts
import { initializeI18n } from "./i18n"

export async function activate(context: vscode.ExtensionContext) {
    // Get user's VSCode language
    const language = vscode.env.language || "en"
    
    // Initialize i18n
    initializeI18n(language)
    
    // Rest of extension initialization
}
```

### 2. Error Messages

```typescript
// In error handling
import { t } from "../i18n"

try {
    await checkpointRestore(commitHash)
} catch (error) {
    vscode.window.showErrorMessage(
        t("common:errors.checkpoint_failed")
    )
}
```

### 3. User Notifications

```typescript
// In commit message generator
import { t } from "../../i18n"

provider.log(t("kilocode:commitMessage.generating"))

vscode.window.showInformationMessage(
    t("kilocode:commitMessage.generated")
)
```

### 4. Webview Messages

```typescript
// Send localized strings to webview
provider.postMessageToWebview({
    type: "showError",
    text: t("common:errors.no_workspace")
})
```

### 5. Dynamic Language Switching

```typescript
// User changes language preference
import { changeLanguage } from "./i18n"

vscode.workspace.onDidChangeConfiguration(e => {
    if (e.affectsConfiguration("kilocode.language")) {
        const newLanguage = vscode.workspace.getConfiguration("kilocode")
            .get<string>("language", "en")
        
        changeLanguage(newLanguage)
        
        // Refresh UI
        provider.postStateToWebview()
    }
})
```

---

## 🦀 RUST TRANSLATION

### Strategy for Lapce

Lapce uses a different i18n approach than VSCode. Options:

**Option 1: Use `fluent` crate (Mozilla's i18n system)**
```rust
use fluent::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

pub struct I18n {
    bundles: HashMap<String, FluentBundle<FluentResource>>,
    current_language: String,
}

impl I18n {
    pub fn new(language: &str) -> Self {
        let mut bundles = HashMap::new();
        
        // Load English (fallback)
        let en_ftl = include_str!("../locales/en/common.ftl");
        let en_resource = FluentResource::try_new(en_ftl.to_string()).unwrap();
        let en_langid: LanguageIdentifier = "en-US".parse().unwrap();
        let mut en_bundle = FluentBundle::new(vec![en_langid]);
        en_bundle.add_resource(en_resource).unwrap();
        bundles.insert("en".to_string(), en_bundle);
        
        // Load other languages similarly
        
        Self {
            bundles,
            current_language: language.to_string(),
        }
    }
    
    pub fn t(&self, key: &str) -> String {
        self.t_with_args(key, None)
    }
    
    pub fn t_with_args(&self, key: &str, args: Option<HashMap<&str, &str>>) -> String {
        let bundle = self.bundles.get(&self.current_language)
            .or_else(|| self.bundles.get("en"))
            .unwrap();
        
        let msg = bundle.get_message(key).unwrap();
        let pattern = msg.value().unwrap();
        
        let mut errors = vec![];
        let value = if let Some(args_map) = args {
            let mut fluent_args = FluentArgs::new();
            for (k, v) in args_map {
                fluent_args.set(k, v);
            }
            bundle.format_pattern(pattern, Some(&fluent_args), &mut errors)
        } else {
            bundle.format_pattern(pattern, None, &mut errors)
        };
        
        value.to_string()
    }
}
```

**Option 2: Simple JSON-based approach (like TypeScript)**
```rust
use serde_json::Value;
use std::collections::HashMap;

pub struct I18n {
    translations: HashMap<String, HashMap<String, Value>>,
    current_language: String,
}

impl I18n {
    pub fn new(language: &str) -> Self {
        let mut translations = HashMap::new();
        
        // Load English
        let en_common: HashMap<String, Value> = 
            serde_json::from_str(include_str!("../locales/en/common.json")).unwrap();
        let en_tools: HashMap<String, Value> = 
            serde_json::from_str(include_str!("../locales/en/tools.json")).unwrap();
        
        let mut en_namespaces = HashMap::new();
        en_namespaces.insert("common".to_string(), en_common);
        en_namespaces.insert("tools".to_string(), en_tools);
        
        translations.insert("en".to_string(), en_namespaces);
        
        // Load other languages similarly
        
        Self {
            translations,
            current_language: language.to_string(),
        }
    }
    
    pub fn t(&self, key: &str) -> String {
        self.t_with_args(key, &HashMap::new())
    }
    
    pub fn t_with_args(&self, key: &str, args: &HashMap<&str, String>) -> String {
        // Parse "namespace:path.to.key"
        let parts: Vec<&str> = key.split(':').collect();
        let (namespace, path) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("common", key)
        };
        
        // Get translation from current language or fallback to English
        let lang_data = self.translations.get(&self.current_language)
            .or_else(|| self.translations.get("en"))
            .unwrap();
        
        let namespace_data = lang_data.get(namespace)
            .or_else(|| self.translations.get("en").unwrap().get(namespace))
            .unwrap();
        
        // Navigate nested path
        let mut value = namespace_data;
        for segment in path.split('.') {
            value = value.get(segment)?;
        }
        
        let mut result = value.as_str()?.to_string();
        
        // Simple interpolation
        for (key, val) in args {
            result = result.replace(&format!("{{{{{}}}}}", key), val);
        }
        
        result
    }
}

// Global instance
lazy_static! {
    pub static ref I18N: RwLock<I18n> = RwLock::new(I18n::new("en"));
}

// Convenience function
pub fn t(key: &str) -> String {
    I18N.read().unwrap().t(key)
}

pub fn t_with_args(key: &str, args: HashMap<&str, String>) -> String {
    I18N.read().unwrap().t_with_args(key, &args)
}
```

**Usage in Rust**:
```rust
use crate::i18n::{t, t_with_args};

// Simple translation
let error_msg = t("common:errors.checkpoint_failed");

// With interpolation
let mut args = HashMap::new();
args.insert("seconds", "30".to_string());
let timeout_msg = t_with_args("common:errors.command_timeout", args);

// In error handling
return Err(anyhow::anyhow!(t("common:errors.no_workspace")));
```

### Compile-Time Inclusion

```rust
// Include translations at compile time
const EN_COMMON: &str = include_str!("../locales/en/common.json");
const EN_TOOLS: &str = include_str!("../locales/en/tools.json");
const FR_COMMON: &str = include_str!("../locales/fr/common.json");
// ... all languages

// Or use macros to generate loading code
macro_rules! load_translations {
    ($($lang:literal => [$($namespace:literal),*]),*) => {
        {
            let mut translations = HashMap::new();
            $(
                let mut lang_data = HashMap::new();
                $(
                    let json_str = include_str!(concat!("../locales/", $lang, "/", $namespace, ".json"));
                    let data: Value = serde_json::from_str(json_str).unwrap();
                    lang_data.insert($namespace.to_string(), data);
                )*
                translations.insert($lang.to_string(), lang_data);
            )*
            translations
        }
    };
}

let translations = load_translations! {
    "en" => ["common", "tools", "kilocode"],
    "fr" => ["common", "tools", "kilocode"],
    "ja" => ["common", "tools", "kilocode"]
};
```

---

## 🎯 KEY DESIGN DECISIONS

### 1. Namespace-Based Organization

**Why separate files by feature?**
- **Modularity**: Each feature area owns its translations
- **Loading**: Can lazy-load namespaces if needed
- **Maintainability**: Easier to find and update related strings
- **Collaboration**: Different teams can work on different namespaces

### 2. Colon Separator for Namespaces

```typescript
t("common:errors.checkpoint_failed")
//  ^^^^^^ namespace
//         ^^^^^^^^^^^^^^^^^^^^^^^^^ key path
```

**Why `:`?**
- Standard i18next convention
- Clear separation from key path (which uses `.`)
- Easy to parse programmatically

### 3. Nested Keys for Hierarchy

```json
{
    "errors": {
        "checkpoint_failed": "...",
        "checkpoint_timeout": "..."
    }
}
```

**Access**: `t("common:errors.checkpoint_failed")`

**Benefits**:
- Logical grouping
- Prevents key name collisions
- Clear semantic meaning

### 4. Interpolation with Double Braces

```json
"welcome": "Welcome, {{name}}! You have {{count}} notifications."
```

**Why `{{}}`?**
- Mustache template syntax (familiar to developers)
- Visually distinct from regular text
- Prevents accidental variable expansion

### 5. Pluralization with Special Keys

```json
"items": {
    "zero": "No items",
    "one": "One item",
    "other": "{{count}} items"
}
```

**i18next automatically selects**:
- `count === 0` → "zero"
- `count === 1` → "one"
- `count > 1` → "other"

**Language-specific rules**: Some languages have more than 3 plural forms (e.g., Polish has 5)

### 6. Runtime Loading vs Compile-Time

**TypeScript (Runtime)**:
- Loads JSON files at extension startup
- Easy to add new languages
- Larger initial bundle

**Rust (Compile-Time)**:
- Translations embedded in binary
- Zero runtime I/O
- Smaller incremental updates

---

## 📊 TRANSLATION STATISTICS

### By Namespace (English)

| Namespace | Lines | Keys (approx) | Purpose |
|-----------|-------|---------------|---------|
| `common.json` | 223 | ~100 | Errors, confirmations, general UI |
| `kilocode.json` | 138 | ~50 | Kilocode-specific features |
| `tools.json` | 19 | ~10 | Tool execution messages |
| `embeddings.json` | ~50 | ~20 | Embeddings UI |
| `marketplace.json` | ~40 | ~15 | Extension marketplace |
| `mcp.json` | ~30 | ~12 | MCP protocol |
| **Total** | **~500** | **~207** | - |

### By Language

- **21 languages** × **6 namespaces** = **126 translation files**
- **~500 lines per language** × **21** = **~10,500 total lines**
- **Coverage**: ~80-95% (not all languages have all namespaces)

---

## 🧪 TESTING CONSIDERATIONS

### Test Setup

```typescript
// Mock translations for tests
process.env.NODE_ENV = "test"

import { initializeI18n, t } from "../i18n"

// Manually provide test translations
i18next.init({
    lng: "en",
    fallbackLng: "en",
    resources: {
        en: {
            common: {
                "errors.test_error": "Test error occurred"
            }
        }
    }
})

describe("i18n", () => {
    it("should translate key", () => {
        expect(t("common:errors.test_error")).toBe("Test error occurred")
    })
    
    it("should interpolate values", () => {
        i18next.addResourceBundle("en", "common", {
            "welcome": "Hello {{name}}"
        }, true, true)
        
        expect(t("common:welcome", { name: "World" })).toBe("Hello World")
    })
})
```

---

## 🔗 DEPENDENCIES

**NPM Packages**:
- `i18next` (^23.7.0) - Core i18n framework

**Rust Crates**:
- Option 1: `fluent` (0.16) + `fluent-bundle` (0.15)
- Option 2: `serde_json` (1.0) for JSON parsing
- `lazy_static` (1.4) for global instance
- `parking_lot` (0.12) for RwLock

---

## 🎓 KEY TAKEAWAYS

✅ **21 Languages**: Comprehensive multi-language support

✅ **Namespace Organization**: Feature-based file structure

✅ **Runtime Loading**: Dynamic translation loading in TypeScript

✅ **Fallback Strategy**: Always defaults to English

✅ **Interpolation**: Dynamic values in translations

✅ **Pluralization**: Correct grammar for different counts

✅ **Simple API**: Single `t()` function for all translations

✅ **Test-Friendly**: Skips file I/O in test environment

---

## 📊 TRANSLATION ESTIMATE

**Complexity**: Low (for Rust implementation)
**Estimated Effort**: 4-6 hours
**Lines of Rust**: ~200-300 lines
**Dependencies**: `serde_json` or `fluent`
**Key Challenge**: Choosing between fluent vs simple JSON approach
**Risk**: Low - straightforward data structure

**Recommendation**: Use simple JSON-based approach for Lapce
- Easier to maintain
- Familiar structure to TypeScript version
- No learning curve for contributors
- `fluent` can be added later if needed

---

**Status**: ✅ Deep analysis complete
**Next**: CHUNK-38 (packages/ipc/)
