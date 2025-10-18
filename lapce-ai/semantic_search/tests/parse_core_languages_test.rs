// Test that all 31 core languages can parse their minimal samples successfully
// External grammars require additional build steps

mod fixtures;

use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use lancedb::processors::language_registry;
use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_parse_31_core_languages() {
    let pipeline = CstToAstPipeline::new();
    let samples = fixtures::minimal_samples::get_minimal_samples();
    let all_langs = language_registry::get_all_languages();
    
    // Create a temp directory for test files
    let temp_dir = TempDir::new().unwrap();
    
    let mut total_core = 0;
    let mut successful = 0;
    let mut failed = Vec::new();
    
    println!("\n=== Testing Parse for 31 Core Languages ===\n");
    
    // Test only core languages (those with is_core = true)
    for lang_info in all_langs.iter().filter(|l| l.is_core) {
        let lang_name = lang_info.name;
        
        // Get the sample code for this language
        let sample_code = match samples.get(lang_name) {
            Some(code) => *code,
            None => {
                println!("❌ {} - No sample code provided", lang_name);
                failed.push((lang_name, "No sample code"));
                total_core += 1;
                continue;
            }
        };
        
        // Create a test file with the appropriate extension
        let extension = fixtures::minimal_samples::get_file_extension(lang_name);
        let file_name = format!("test.{}", extension);
        let file_path = temp_dir.path().join(&file_name);
        
        // Write the sample code to the file
        if let Err(e) = fs::write(&file_path, sample_code) {
            println!("❌ {} - Failed to write test file: {}", lang_name, e);
            failed.push((lang_name, "File write error"));
            total_core += 1;
            continue;
        }
        
        // Try to parse the file
        total_core += 1;
        match pipeline.process_file(&file_path).await {
            Ok(output) => {
                // Check that we got a valid AST
                if output.ast.node_type != lancedb::processors::cst_to_ast_pipeline::AstNodeType::Unknown {
                    println!("✅ {} - Parsed successfully ({})", lang_name, output.language);
                    successful += 1;
                } else {
                    println!("⚠️  {} - Parsed but AST is Unknown type", lang_name);
                    failed.push((lang_name, "AST type is Unknown"));
                }
            }
            Err(e) => {
                println!("❌ {} - Parse failed: {}", lang_name, e);
                failed.push((lang_name, "Parse error"));
            }
        }
        
        // Clean up the test file
        let _ = fs::remove_file(&file_path);
    }
    
    println!("\n=== Core Languages Parse Test Summary ===");
    println!("Total core languages: {}", total_core);
    println!("Successful: {} ({:.1}%)", successful, (successful as f64 / total_core as f64) * 100.0);
    println!("Failed: {} ({:.1}%)", failed.len(), (failed.len() as f64 / total_core as f64) * 100.0);
    
    if !failed.is_empty() {
        println!("\nFailed languages:");
        for (lang, reason) in &failed {
            println!("  - {} ({})", lang, reason);
        }
    }
    
    // Acceptance criteria: All 31 core languages must parse successfully
    assert_eq!(
        successful, 31,
        "All 31 core languages must parse successfully. {} failed: {:?}",
        failed.len(),
        failed.iter().map(|(l, _)| l).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn test_external_grammars_status() {
    let all_langs = language_registry::get_all_languages();
    let external_langs: Vec<_> = all_langs.iter()
        .filter(|l| !l.is_core)
        .map(|l| l.name)
        .collect();
    
    println!("\n=== External Grammar Languages (36) ===");
    println!("These languages require CST-tree-sitter external grammar builds:");
    for lang in &external_langs {
        println!("  - {}", lang);
    }
    
    assert_eq!(external_langs.len(), 36, "Should have 36 external grammar languages");
    println!("\n✅ All 36 external languages identified (build required for full support)");
}
