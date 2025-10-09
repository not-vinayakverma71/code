//! Unified language registry for all tree-sitter grammars
//! Handles both LANGUAGE constant and language() function APIs

use std::collections::HashMap;
use std::sync::Arc;
use once_cell::sync::Lazy;
use tree_sitter::Language;

#[derive(Debug, thiserror::Error)]
pub enum LanguageError {
    #[error("Unknown file extension: {0}")]
    UnknownExtension(String),
    
    #[error("Unknown language: {0}")]
    UnknownLanguage(String),
    
    #[error("Language not compiled in: {0}")]
    LanguageNotAvailable(String),
}

#[derive(Debug, Clone)]
pub struct LanguageInfo {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub language: Language,
}

/// Registry of all available tree-sitter languages
pub struct LanguageRegistry {
    by_extension: HashMap<String, Arc<LanguageInfo>>,
    by_name: HashMap<String, Arc<LanguageInfo>>,
}

impl LanguageRegistry {
    /// Get singleton instance
    pub fn instance() -> &'static LanguageRegistry {
        &REGISTRY
    }
    
    /// Get language by file extension
    pub fn by_extension(&self, ext: &str) -> Result<&LanguageInfo, LanguageError> {
        // Normalize extension (remove leading dot if present)
        let ext = if ext.starts_with('.') {
            &ext[1..]
        } else {
            ext
        };
        
        self.by_extension
            .get(ext)
            .map(|info| info.as_ref())
            .ok_or_else(|| LanguageError::UnknownExtension(ext.to_string()))
    }
    
    /// Get language by name
    pub fn by_name(&self, name: &str) -> Result<&LanguageInfo, LanguageError> {
        self.by_name
            .get(&name.to_lowercase())
            .map(|info| info.as_ref())
            .ok_or_else(|| LanguageError::UnknownLanguage(name.to_string()))
    }
    
    /// Get language for a file path
    pub fn for_path(&self, path: &std::path::Path) -> Result<&LanguageInfo, LanguageError> {
        // Check special cases first (files without extensions)
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Special case for Makefile
            if filename == "Makefile" {
                return self.by_name("make");
            }
            
            // Special case for Dockerfile
            if filename.starts_with("Dockerfile") {
                return self.by_name("dockerfile");
            }
            
            // Special case for CMakeLists.txt
            if filename == "CMakeLists.txt" {
                return self.by_name("cmake");
            }
        }
        
        // Try extension-based lookup
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| LanguageError::UnknownExtension("(no extension)".to_string()))?;
        
        self.by_extension(ext)
    }
    
    /// List all available languages
    pub fn list_languages(&self) -> Vec<&LanguageInfo> {
        let mut langs: Vec<_> = self.by_name.values().map(|i| i.as_ref()).collect();
        langs.sort_by_key(|info| info.name);
        langs
    }
    
    /// Get total count of languages
    pub fn count(&self) -> usize {
        self.by_name.len()
    }
}

// Macro to handle languages with LANGUAGE constant
macro_rules! lang_const {
    ($name:expr, $module:ident, $exts:expr) => {
        Arc::new(LanguageInfo {
            name: $name,
            extensions: $exts,
            language: $module::LANGUAGE.into(),
        })
    };
}

// Macro to handle languages with language() function
#[allow(unused_macros)]
macro_rules! lang_fn {
    ($name:expr, $module:ident, $exts:expr) => {
        Arc::new(LanguageInfo {
            name: $name,
            extensions: $exts,
            language: $module::language(),
        })
    };
}

// Global registry instance
static REGISTRY: Lazy<LanguageRegistry> = Lazy::new(|| {
    let mut by_extension = HashMap::new();
    let mut by_name = HashMap::new();
    
    // Helper to register a language
    let mut register = |info: Arc<LanguageInfo>| {
        // Register by name
        by_name.insert(info.name.to_lowercase(), info.clone());
        
        // Register by extensions
        for ext in info.extensions {
            by_extension.insert(ext.to_string(), info.clone());
        }
    };
    
    // Register all 73 languages
    // Using LANGUAGE constant (most crates)
    register(lang_const!("rust", tree_sitter_rust, &["rs"]));
    register(lang_const!("python", tree_sitter_python, &["py", "pyi"]));
    register(lang_const!("go", tree_sitter_go, &["go"]));
    register(lang_const!("java", tree_sitter_java, &["java"]));
    register(lang_const!("c", tree_sitter_c, &["c", "h"]));
    register(lang_const!("cpp", tree_sitter_cpp, &["cpp", "cc", "cxx", "hpp", "hh", "hxx"]));
    register(lang_const!("csharp", tree_sitter_c_sharp, &["cs"]));
    register(lang_const!("ruby", tree_sitter_ruby, &["rb"]));
    // PHP exports language_php() in 0.23.x
    register(Arc::new(LanguageInfo {
        name: "php",
        extensions: &["php"],
        language: tree_sitter_php::language_php(),
    }));
    register(lang_const!("lua", tree_sitter_lua, &["lua"]));
    register(lang_const!("bash", tree_sitter_bash, &["sh", "bash"]));
    register(lang_const!("css", tree_sitter_css, &["css"]));
    register(lang_const!("json", tree_sitter_json, &["json"]));
    register(lang_const!("swift", tree_sitter_swift, &["swift"]));
    register(lang_const!("scala", tree_sitter_scala, &["scala"]));
    register(lang_const!("elixir", tree_sitter_elixir, &["ex", "exs"]));
    register(lang_const!("html", tree_sitter_html, &["html", "htm"]));
    
    // OCaml uses LANGUAGE_OCAML constant in version 0.23.2
    register(Arc::new(LanguageInfo {
        name: "ocaml",
        extensions: &["ml", "mli"],
        language: tree_sitter_ocaml::LANGUAGE_OCAML.into(),
    }));
    
    register(lang_const!("nix", tree_sitter_nix, &["nix"]));
    register(lang_const!("make", tree_sitter_make, &["mk"])); // Makefile handled specially
    register(lang_const!("cmake", tree_sitter_cmake, &["cmake"])); // CMakeLists.txt handled specially
    register(lang_const!("verilog", tree_sitter_verilog, &["v", "vh"]));
    register(lang_const!("erlang", tree_sitter_erlang, &["erl", "hrl"]));
    register(lang_const!("d", tree_sitter_d, &["d"]));
    register(lang_const!("pascal", tree_sitter_pascal, &["pas", "pp"]));
    // Common Lisp uses LANGUAGE_COMMONLISP constant
    register(Arc::new(LanguageInfo {
        name: "commonlisp",
        extensions: &["lisp", "cl"],
        language: tree_sitter_commonlisp::LANGUAGE_COMMONLISP.into(),
    }));
    // ObjC exports language() function
    #[cfg(feature = "lang-objc")]
    register(lang_fn!("objc", tree_sitter_objc, &["m"])); // Note: conflicts with MATLAB
    register(lang_const!("groovy", tree_sitter_groovy, &["groovy", "gradle"]));
    register(lang_const!("embedded_template", tree_sitter_embedded_template, &["erb", "ejs"]));
    
    // === EXTERNAL GRAMMARS (37) - Optional via feature flags ===
    
    #[cfg(feature = "lang-javascript")]
    register(Arc::new(LanguageInfo {
        name: "javascript",
        extensions: &["js", "jsx", "mjs"],
        language: tree_sitter_javascript::language(),
    }));
    
    #[cfg(feature = "lang-typescript")]
    {
        register(Arc::new(LanguageInfo {
            name: "typescript",
            extensions: &["ts"],
            language: tree_sitter_typescript::language_typescript(),
        }));
        
        register(Arc::new(LanguageInfo {
            name: "tsx",
            extensions: &["tsx"],
            language: tree_sitter_typescript::language_tsx(),
        }));
    }
    
    #[cfg(feature = "lang-toml")]
    register(Arc::new(LanguageInfo {
        name: "toml",
        extensions: &["toml"],
        language: tree_sitter_toml::language(),
    }));
    
    #[cfg(feature = "lang-dockerfile")]
    register(Arc::new(LanguageInfo {
        name: "dockerfile",
        extensions: &[],  // Handled specially
        language: tree_sitter_dockerfile::language(),
    }));
    
    #[cfg(feature = "lang-elm")]
    register(lang_fn!("elm", tree_sitter_elm, &["elm"]));
    
    #[cfg(feature = "lang-kotlin")]
    register(lang_fn!("kotlin", tree_sitter_kotlin, &["kt", "kts"]));
    
    #[cfg(feature = "lang-yaml")]
    register(lang_fn!("yaml", tree_sitter_yaml, &["yaml", "yml"]));
    
    #[cfg(feature = "lang-r")]
    register(lang_fn!("r", tree_sitter_r, &["r", "R"]));
    
    #[cfg(feature = "lang-matlab")]
    register(lang_fn!("matlab", tree_sitter_matlab, &["mat"])); // .m conflicts with objc
    
    #[cfg(feature = "lang-perl")]
    register(lang_fn!("perl", tree_sitter_perl, &["pl", "pm"]));
    
    #[cfg(feature = "lang-dart")]
    register(lang_fn!("dart", tree_sitter_dart, &["dart"]));
    
    #[cfg(feature = "lang-julia")]
    register(lang_fn!("julia", tree_sitter_julia, &["jl"]));
    
    #[cfg(feature = "lang-haskell")]
    register(lang_fn!("haskell", tree_sitter_haskell, &["hs", "lhs"]));
    
    #[cfg(feature = "lang-graphql")]
    register(lang_fn!("graphql", tree_sitter_graphql, &["graphql", "gql"]));
    
    #[cfg(feature = "lang-sql")]
    register(lang_fn!("sql", tree_sitter_sql, &["sql"]));
    
    #[cfg(feature = "lang-zig")]
    register(lang_fn!("zig", tree_sitter_zig, &["zig"]));
    
    #[cfg(feature = "lang-vim")]
    register(lang_fn!("vim", tree_sitter_vim, &["vim"]));
    
    #[cfg(feature = "lang-abap")]
    register(lang_fn!("abap", tree_sitter_abap, &["abap"]));
    
    #[cfg(feature = "lang-nim")]
    register(lang_fn!("nim", tree_sitter_nim, &["nim", "nims"]));
    
    #[cfg(feature = "lang-clojure")]
    register(lang_fn!("clojure", tree_sitter_clojure, &["clj", "cljs", "cljc"]));
    
    #[cfg(feature = "lang-crystal")]
    register(lang_fn!("crystal", tree_sitter_crystal, &["cr"]));
    
    #[cfg(feature = "lang-fortran")]
    register(lang_fn!("fortran", tree_sitter_fortran, &["f90", "f95", "f03", "f"]));
    
    #[cfg(feature = "lang-vhdl")]
    register(lang_fn!("vhdl", tree_sitter_vhdl, &["vhd", "vhdl"]));
    
    #[cfg(feature = "lang-racket")]
    register(lang_fn!("racket", tree_sitter_racket, &["rkt"]));
    
    #[cfg(feature = "lang-ada")]
    register(lang_fn!("ada", tree_sitter_ada, &["ads", "adb"]));
    
    #[cfg(feature = "lang-prolog")]
    register(lang_fn!("prolog", tree_sitter_prolog, &["pro"])); // .pl conflicts with perl
    
    #[cfg(feature = "lang-gradle")]
    register(lang_fn!("gradle", tree_sitter_gradle, &[])); // .gradle handled by groovy
    
    #[cfg(feature = "lang-xml")]
    register(lang_fn!("xml", tree_sitter_xml, &["xml"]));
    
    #[cfg(feature = "lang-markdown")]
    register(lang_fn!("markdown", tree_sitter_md, &["md", "markdown"]));
    
    #[cfg(feature = "lang-svelte")]
    register(lang_fn!("svelte", tree_sitter_svelte, &["svelte"]));
    
    #[cfg(feature = "lang-scheme")]
    register(lang_fn!("scheme", tree_sitter_scheme, &["scm", "ss"]));
    
    #[cfg(feature = "lang-fennel")]
    register(lang_fn!("fennel", tree_sitter_fennel, &["fnl"]));
    
    #[cfg(feature = "lang-gleam")]
    register(lang_fn!("gleam", tree_sitter_gleam, &["gleam"]));
    
    #[cfg(feature = "lang-hcl")]
    register(lang_fn!("hcl", tree_sitter_hcl, &["hcl", "tf"]));
    
    #[cfg(feature = "lang-solidity")]
    register(lang_fn!("solidity", tree_sitter_solidity, &["sol"]));
    
    #[cfg(feature = "lang-fsharp")]
    register(lang_fn!("fsharp", tree_sitter_fsharp, &["fs", "fsx", "fsi"]));
    
    #[cfg(feature = "lang-cobol")]
    register(lang_fn!("cobol", tree_sitter_cobol, &["cob", "cbl"]));
    
    #[cfg(feature = "lang-systemverilog")]
    register(lang_fn!("systemverilog", tree_sitter_systemverilog, &["sv", "svh"]));
    
    LanguageRegistry {
        by_extension,
        by_name,
    }
});

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry_singleton() {
        let reg1 = LanguageRegistry::instance();
        let reg2 = LanguageRegistry::instance();
        assert_eq!(reg1 as *const _, reg2 as *const _);
    }
    
    #[test]
    fn test_by_extension() {
        let registry = LanguageRegistry::instance();
        
        // Test with dot
        assert!(registry.by_extension(".rs").is_ok());
        assert_eq!(registry.by_extension(".rs").unwrap().name, "rust");
        
        // Test without dot
        assert!(registry.by_extension("py").is_ok());
        assert_eq!(registry.by_extension("py").unwrap().name, "python");
        
        // Test unknown extension
        assert!(registry.by_extension("xyz").is_err());
    }
    
    #[test]
    fn test_by_name() {
        let registry = LanguageRegistry::instance();
        
        // Test case insensitive
        assert!(registry.by_name("rust").is_ok());
        assert!(registry.by_name("RUST").is_ok());
        assert!(registry.by_name("Rust").is_ok());
        
        // Test unknown language
        assert!(registry.by_name("unknown").is_err());
    }
    
    #[test]
    fn test_for_path() {
        let registry = LanguageRegistry::instance();
        
        // Test regular file
        let path = std::path::Path::new("test.rs");
        assert_eq!(registry.for_path(path).unwrap().name, "rust");
        
        // Test Makefile
        let path = std::path::Path::new("Makefile");
        assert_eq!(registry.for_path(path).unwrap().name, "make");
        
        // Test Dockerfile (only if feature enabled)
        #[cfg(feature = "lang-dockerfile")]
        {
            let path = std::path::Path::new("Dockerfile");
            assert_eq!(registry.for_path(path).unwrap().name, "dockerfile");
        }
        
        // Test CMakeLists.txt
        let path = std::path::Path::new("CMakeLists.txt");
        assert_eq!(registry.for_path(path).unwrap().name, "cmake");
    }
    
    #[test]
    fn test_list_languages() {
        let registry = LanguageRegistry::instance();
        let languages = registry.list_languages();
        
        // Should have at least the core languages from crates.io
        // Some optional grammars (e.g., Objective-C) may be disabled by default.
        assert!(languages.len() >= 28);
        
        // Check sorted
        for _i in 1..languages.len() {
            assert!(languages[i-1].name <= languages[i].name);
        }
    }
}
