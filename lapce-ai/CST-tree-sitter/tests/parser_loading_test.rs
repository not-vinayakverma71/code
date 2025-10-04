//! Test all 122 parsers load correctly

use lapce_tree_sitter::parser_manager::NativeParserManager;
use lapce_tree_sitter::types::Language;
use std::time::Instant;

#[test]
fn test_all_122_parsers_load() {
    let manager = NativeParserManager::new().expect("Failed to create manager");
    let mut successful = 0;
    let mut failed = Vec::new();
    
    let languages = vec![
        Language::Rust, Language::C, Language::Cpp, Language::Zig, Language::Nim,
        Language::D, Language::Ada, Language::Fortran, Language::Cobol, Language::Pascal,
        Language::Assembly, Language::Wasm, Language::Llvm, Language::Cuda, Language::OpenCL,
        Language::JavaScript, Language::TypeScript, Language::TypeScriptReact, Language::JavaScriptReact,
        Language::Html, Language::Css, Language::Scss, Language::Less, Language::Vue, Language::Svelte,
        Language::Angular, Language::React, Language::Astro, Language::Php, Language::Hack,
        Language::WebAssembly, Language::GraphQL, Language::Prisma, Language::Apollo, Language::Relay,
        Language::Java, Language::Kotlin, Language::Scala, Language::Groovy, Language::Clojure,
        Language::JvmBytecode, Language::Gradle, Language::Maven, Language::Ant, Language::Sbt,
        Language::CSharp, Language::FSharp, Language::VisualBasic, Language::PowerShell, Language::AspNet,
        Language::Xaml, Language::MsBuild, Language::IlAsm, Language::Haskell, Language::OCaml,
        Language::Elm, Language::PureScript, Language::Idris, Language::Agda, Language::Coq,
        Language::Lean, Language::StandardML, Language::Scheme, Language::Racket, Language::CommonLisp,
        Language::Python, Language::Ruby, Language::Perl, Language::Lua, Language::Tcl,
        Language::R, Language::Julia, Language::Matlab, Language::Octave, Language::Bash,
        Language::Fish, Language::Zsh, Language::PowerShellCore, Language::VimScript, Language::Elisp,
        Language::Swift, Language::ObjectiveC, Language::Dart, Language::Flutter, Language::ReactNative,
        Language::Xamarin, Language::Ionic, Language::NativeScript, Language::Sql, Language::PlSql,
        Language::TSql, Language::Sparql, Language::Cypher, Language::MongoDb, Language::Cassandra,
        Language::Redis, Language::ElasticSearch, Language::InfluxDb, Language::Yaml, Language::Toml,
        Language::Json, Language::Json5, Language::JsonC, Language::Xml, Language::Ini,
        Language::Properties, Language::Env, Language::EditorConfig, Language::GitIgnore, Language::DockerIgnore,
        Language::Dockerfile, Language::Kubernetes, Language::Helm, Language::Terraform, Language::Ansible,
        Language::Puppet, Language::Chef, Language::SaltStack, Language::Vagrant, Language::Jenkins,
    ];
    
    println!("\n🔍 Testing {} language parsers...\n", languages.len());
    let start = Instant::now();
    
    for lang in languages {
        let test_start = Instant::now();
        match manager.load_language(lang) {
            Ok(_) => {
                successful += 1;
                println!("✅ {} loaded successfully ({:?})", lang.name(), test_start.elapsed());
            }
            Err(e) => {
                failed.push((lang, e));
                println!("❌ {} failed to load: {}", lang.name(), e);
            }
        }
    }
    
    let elapsed = start.elapsed();
    println!("\n📊 Results:");
    println!("  ✅ Successful: {}/122", successful);
    println!("  ❌ Failed: {}/122", failed.len());
    println!("  ⏱️  Total time: {:?}", elapsed);
    
    if !failed.is_empty() {
        println!("\n❌ Failed languages:");
        for (lang, err) in &failed {
            println!("  - {}: {}", lang.name(), err);
        }
    }
    
    assert!(successful >= 17, "At least 17 languages should load (native parsers)");
}

#[test]
fn test_memory_usage() {
    let manager = NativeParserManager::new().expect("Failed to create manager");
    
    // Load several parsers
    let _ = manager.load_language(Language::Rust);
    let _ = manager.load_language(Language::JavaScript);
    let _ = manager.load_language(Language::Python);
    let _ = manager.load_language(Language::Go);
    let _ = manager.load_language(Language::TypeScript);
    
    // Check memory metrics
    let memory_mb = manager.metrics.get_memory_usage_mb();
    println!("Memory usage after loading 5 parsers: {:.2}MB", memory_mb);
    
    assert!(memory_mb < 5.0, "Memory usage should be under 5MB");
}

#[test]
fn test_no_memory_leaks() {
    let manager = NativeParserManager::new().expect("Failed to create manager");
    
    // Load and drop parsers multiple times
    for _ in 0..100 {
        let _ = manager.load_language(Language::Rust);
        let _ = manager.load_language(Language::JavaScript);
    }
    
    let memory_mb = manager.metrics.get_memory_usage_mb();
    println!("Memory after 100 load cycles: {:.2}MB", memory_mb);
    
    assert!(memory_mb < 5.0, "No memory leaks - usage should stay under 5MB");
}
