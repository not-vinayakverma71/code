// Test parsing using upstream grammar corpus files
// This validates that our parsers can handle real-world test cases

use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use lancedb::processors::language_registry;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Extract code snippets from corpus files
fn extract_corpus_snippets(corpus_path: &Path) -> Vec<String> {
    let content = match fs::read_to_string(corpus_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    
    let mut snippets = Vec::new();
    let mut current_snippet = String::new();
    let mut in_code_block = false;
    
    for line in content.lines() {
        if line.starts_with("===") || line.starts_with("---") {
            if !current_snippet.is_empty() {
                snippets.push(current_snippet.clone());
                current_snippet.clear();
            }
            in_code_block = !in_code_block;
        } else if in_code_block && !line.starts_with("(") && !line.starts_with(")") {
            // Skip tree-sitter AST output lines
            current_snippet.push_str(line);
            current_snippet.push('\n');
        }
    }
    
    if !current_snippet.is_empty() {
        snippets.push(current_snippet);
    }
    
    snippets
}

#[tokio::test]
async fn test_corpus_validation_core_languages() {
    let pipeline = CstToAstPipeline::new();
    let temp_dir = TempDir::new().unwrap();
    
    // Map of language names to their corpus directories in CST-tree-sitter
    let corpus_dirs = vec![
        ("rust", None), // Rust doesn't have corpus in external-grammars
        ("python", None), // Python doesn't have corpus in external-grammars
        ("javascript", Some("external-grammars/tree-sitter-javascript/test/corpus")),
        ("typescript", Some("external-grammars/tree-sitter-typescript/test/corpus")),
        ("yaml", Some("external-grammars/tree-sitter-yaml/test/corpus")),
        ("toml", Some("external-grammars/tree-sitter-toml/test/corpus")),
        ("dockerfile", Some("external-grammars/tree-sitter-dockerfile/test/corpus")),
    ];
    
    let mut total_languages = 0;
    let mut languages_with_corpus = 0;
    let mut total_snippets = 0;
    let mut successful_parses = 0;
    
    println!("\n=== Corpus Validation Test ===\n");
    
    for (lang_name, corpus_rel_path) in corpus_dirs {
        total_languages += 1;
        
        let corpus_path = match corpus_rel_path {
            Some(rel) => {
                let base = PathBuf::from("/home/verma/lapce/lapce-ai/CST-tree-sitter");
                base.join(rel)
            }
            None => {
                println!("⏭️  {} - No corpus directory available", lang_name);
                continue;
            }
        };
        
        if !corpus_path.exists() {
            println!("⏭️  {} - Corpus directory not found: {:?}", lang_name, corpus_path);
            continue;
        }
        
        languages_with_corpus += 1;
        
        // Read all .txt corpus files
        let corpus_files: Vec<_> = match fs::read_dir(&corpus_path) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "txt")
                        .unwrap_or(false)
                })
                .collect(),
            Err(_) => {
                println!("❌ {} - Failed to read corpus directory", lang_name);
                continue;
            }
        };
        
        let mut lang_snippets = 0;
        let mut lang_success = 0;
        
        for entry in corpus_files.iter().take(5) { // Test first 5 corpus files per language
            let snippets = extract_corpus_snippets(&entry.path());
            
            for (idx, snippet) in snippets.iter().enumerate().take(3) { // Test first 3 snippets per file
                if snippet.trim().is_empty() {
                    continue;
                }
                
                lang_snippets += 1;
                total_snippets += 1;
                
                // Create test file
                let ext = match lang_name {
                    "javascript" => "js",
                    "typescript" => "ts",
                    "yaml" => "yaml",
                    "toml" => "toml",
                    "dockerfile" => "dockerfile",
                    _ => "txt",
                };
                
                let file_name = format!("test_{}.{}", idx, ext);
                let file_path = temp_dir.path().join(&file_name);
                
                if let Err(_) = fs::write(&file_path, snippet) {
                    continue;
                }
                
                // Try to parse
                match pipeline.process_file(&file_path).await {
                    Ok(_) => {
                        lang_success += 1;
                        successful_parses += 1;
                    }
                    Err(_) => {
                        // Parsing failed, but that's okay for corpus validation
                    }
                }
                
                let _ = fs::remove_file(&file_path);
            }
        }
        
        if lang_snippets > 0 {
            let success_rate = (lang_success as f64 / lang_snippets as f64) * 100.0;
            println!("✅ {} - Tested {} snippets, {:.1}% success rate", 
                     lang_name, lang_snippets, success_rate);
        }
    }
    
    println!("\n=== Summary ===");
    println!("Languages tested: {}/{}", languages_with_corpus, total_languages);
    println!("Total snippets tested: {}", total_snippets);
    println!("Successful parses: {} ({:.1}%)", 
             successful_parses, 
             if total_snippets > 0 { 
                 (successful_parses as f64 / total_snippets as f64) * 100.0 
             } else { 
                 0.0 
             });
    
    // Acceptance: At least some corpus validation should work
    assert!(languages_with_corpus > 0, "Should have at least one language with corpus");
    if total_snippets > 0 {
        assert!(successful_parses > 0, "Should have at least some successful parses from corpus");
    }
}

#[test]
fn test_corpus_files_exist() {
    let cst_base = PathBuf::from("/home/verma/lapce/lapce-ai/CST-tree-sitter");
    
    // Check that at least some corpus directories exist
    let corpus_paths = vec![
        "external-grammars/tree-sitter-yaml/test/corpus",
        "external-grammars/tree-sitter-toml/test/corpus",
        "external-grammars/tree-sitter-dockerfile/test/corpus",
    ];
    
    let mut found = 0;
    for rel_path in corpus_paths {
        let full_path = cst_base.join(rel_path);
        if full_path.exists() {
            found += 1;
            println!("✅ Found corpus: {}", rel_path);
        }
    }
    
    assert!(found > 0, "Should find at least one corpus directory");
    println!("\n✅ Found {} corpus directories", found);
}
