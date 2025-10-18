// Test with 100K files for scale requirement
// use lapce_ai_rust::lancedb // Module not available
// Original: use lapce_ai_rust::lancedb::*;
use std::sync::Arc;
use std::time::Instant;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 100K Files Scale Test\n");
    
    // Setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_str().unwrap();
    let engine = Arc::new(SemanticSearchEngine::new(db_path).await?);
    
    // Create optimized table with IVF_PQ
    let _ = ivf_pq_optimized::create_optimized_table(
        &engine.connection,
        "scale_test_100k"
    ).await?;
    
    // Generate 100K test files
    println!("Generating 100K test files...");
    let test_corpus = generate_test_corpus(100_000)?;
    
    // Test indexing performance
    println!("\n📊 Indexing 100K files...");
    let indexer = CodeIndexer::new(engine.clone());
    let index_start = Instant::now();
    
    let stats = indexer.index_repository(&test_corpus).await?;
    let index_time = index_start.elapsed();
    
    // Calculate metrics
    let files_per_sec = stats.files_indexed as f64 / index_time.as_secs_f64();
    
    println!("\n✅ Indexing Complete:");
    println!("   Files: {}", stats.files_indexed);
    println!("   Chunks: {}", stats.chunks_created);
    println!("   Time: {:?}", index_time);
    println!("   Speed: {:.0} files/sec", files_per_sec);
    
    // Test query performance at scale
    println!("\n🔍 Testing Query Performance at 100K scale...");
    let query_tests = vec![
        "semantic search",
        "async function",
        "error handling",
        "memory optimization",
        "performance test"
    ];
    
    let mut total_latency = 0u128;
    let mut max_latency = 0u128;
    let mut min_latency = u128::MAX;
    
    for query in &query_tests {
        let start = Instant::now();
        let results = engine.codebase_search(query, None).await?;
        let latency = start.elapsed().as_millis();
        
        total_latency += latency;
        max_latency = max_latency.max(latency);
        min_latency = min_latency.min(latency);
        
        println!("   Query '{}': {}ms ({} results)", query, latency, results.results.len());
    }
    
    let avg_latency = total_latency / query_tests.len() as u128;
    
    println!("\n📊 Query Performance Summary:");
    println!("   Avg: {}ms", avg_latency);
    println!("   Min: {}ms", min_latency);
    println!("   Max: {}ms", max_latency);
    
    // Memory check
    let mem = MemoryOptimizer::get_memory_usage_mb();
    println!("\n💾 Memory Usage: {:.1}MB", mem);
    
    // Success criteria check
    println!("\n✅ Success Criteria:");
    if stats.files_indexed >= 100_000 {
        println!("   ✅ Scale: {}K files indexed", stats.files_indexed / 1000);
    } else {
        println!("   ❌ Scale: Only {}K files", stats.files_indexed / 1000);
    }
    
    if files_per_sec > 1000.0 {
        println!("   ✅ Index Speed: {:.0} files/sec > 1000", files_per_sec);
    } else {
        println!("   ❌ Index Speed: {:.0} files/sec < 1000", files_per_sec);
    }
    
    if avg_latency < 5 {
        println!("   ✅ Query Latency: {}ms < 5ms", avg_latency);
    } else {
        println!("   ❌ Query Latency: {}ms > 5ms", avg_latency);
    }
    
    if mem < 10.0 {
        println!("   ✅ Memory: {:.1}MB < 10MB", mem);
    } else {
        println!("   ❌ Memory: {:.1}MB > 10MB", mem);
    }
    
    Ok(())
}

fn generate_test_corpus(num_files: usize) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let corpus_path = temp_dir.path().to_path_buf();
    
    println!("Creating {} test files in {:?}", num_files, corpus_path);
    
    // Create directory structure
    let languages = vec!["rust", "python", "javascript", "go", "java"];
    let extensions = vec!["rs", "py", "js", "go", "java"];
    
    for i in 0..num_files {
        let lang_idx = i % languages.len();
        let lang = languages[lang_idx];
        let ext = extensions[lang_idx];
        
        // Create subdirectories
        let subdir = corpus_path.join(format!("{}/{:04}", lang, i / 100));
        fs::create_dir_all(&subdir)?;
        
        // Generate file content
        let filename = subdir.join(format!("file_{:06}.{}", i, ext));
        let content = generate_code_content(lang, i);
        fs::write(filename, content)?;
        
        if i % 10000 == 0 {
            println!("  Generated {}/{} files...", i, num_files);
        }
    }
    
    // Leak the temp dir so it doesn't get deleted
    std::mem::forget(temp_dir);
    
    Ok(corpus_path)
}

fn generate_code_content(language: &str, index: usize) -> String {
    match language {
        "rust" => format!(
            r#"// File {}
use std::collections::HashMap;

pub fn process_data_{index}(data: Vec<String>) -> Result<HashMap<String, usize>, Error> {{
    let mut result = HashMap::new();
    for item in data {{
        let count = result.entry(item).or_insert(0);
        *count += 1;
    }}
    Ok(result)
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_process_{index}() {{
        let data = vec!["test".to_string()];
        assert!(process_data_{index}(data).is_ok());
    }}
}}"#, index, index=index
        ),
        "python" => format!(
            r#"# File {}
import asyncio
from typing import Dict, List

async def process_data_{}(data: List[str]) -> Dict[str, int]:
    """Process data and return frequency map."""
    result = {{}}
    for item in data:
        result[item] = result.get(item, 0) + 1
    await asyncio.sleep(0)  # Simulate async operation
    return result

def main():
    data = ["test", "data", "test"]
    loop = asyncio.get_event_loop()
    result = loop.run_until_complete(process_data_{}(data))
    print(result)

if __name__ == "__main__":
    main()
"#, index, index, index
        ),
        _ => format!("// Generic code file {}\nfunction test() {{ return {}; }}", index, index)
    }
}
