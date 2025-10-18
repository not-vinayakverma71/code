// Test that all 67 languages can parse their minimal samples successfully

mod fixtures;

use lancedb::processors::cst_to_ast_pipeline::CstToAstPipeline;
use lancedb::processors::language_registry;
use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_parse_all_67_languages() {
    let pipeline = CstToAstPipeline::new();
    let samples = fixtures::minimal_samples::get_minimal_samples();
    let all_langs = language_registry::get_all_languages();
    
    // Create a temp directory for test files
    let temp_dir = TempDir::new().unwrap();
    
    let mut total = 0;
    let mut successful = 0;
    let mut failed = Vec::new();
    
    println!("\n=== Testing Parse for All 67 Languages ===\n");
    
    for lang_info in all_langs {
        let lang_name = lang_info.name;
        
        // Get the sample code for this language
        let sample_code = match samples.get(lang_name) {
            Some(code) => *code,
            None => {
                println!("❌ {} - No sample code provided", lang_name);
                failed.push((lang_name, "No sample code"));
                total += 1;
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
            total += 1;
            continue;
        }
        
        // Try to parse the file
        total += 1;
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
    
    println!("\n=== Parse Test Summary ===");
    println!("Total languages: {}", total);
    println!("Successful: {} ({:.1}%)", successful, (successful as f64 / total as f64) * 100.0);
    println!("Failed: {} ({:.1}%)", failed.len(), (failed.len() as f64 / total as f64) * 100.0);
    
    if !failed.is_empty() {
        println!("\nFailed languages:");
        for (lang, reason) in &failed {
            println!("  - {} ({})", lang, reason);
        }
    }
    
    // Acceptance criteria: All 67 languages must parse successfully
    assert_eq!(
        successful, 67,
        "All 67 languages must parse successfully. {} failed: {:?}",
        failed.len(),
        failed.iter().map(|(l, _)| l).collect::<Vec<_>>()
    );
}

#[test]
fn test_all_samples_provided() {
    let samples = fixtures::minimal_samples::get_minimal_samples();
    let all_langs = language_registry::get_all_languages();
    
    let mut missing = Vec::new();
    
    for lang_info in all_langs {
        if !samples.contains_key(lang_info.name) {
            missing.push(lang_info.name);
        }
    }
    
    assert!(
        missing.is_empty(),
        "Missing sample code for {} languages: {:?}",
        missing.len(),
        missing
    );
    
    println!("✅ All 67 languages have sample code");
}

#[test]
fn test_extension_mapping_complete() {
    let all_langs = language_registry::get_all_languages();
    
    for lang_info in all_langs {
        let ext = fixtures::minimal_samples::get_file_extension(lang_info.name);
        assert_ne!(
            ext, "txt",
            "Language {} missing extension mapping",
            lang_info.name
        );
    }
    
    println!("✅ All 67 languages have extension mappings");
}
