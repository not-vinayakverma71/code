//! Test if we can compress trees to reduce memory

use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::sync::Arc;
use std::path::PathBuf;

fn get_rss_kb() -> u64 {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().unwrap_or(0);
                }
            }
        }
    }
    0
}

// Serialize tree to bytes
fn serialize_tree(tree: &tree_sitter::Tree, source: &[u8]) -> Vec<u8> {
    let mut serialized = Vec::new();
    
    // Store source length
    serialized.extend_from_slice(&(source.len() as u32).to_le_bytes());
    
    // Store source
    serialized.extend_from_slice(source);
    
    // Serialize tree structure
    fn serialize_node(node: tree_sitter::Node, output: &mut Vec<u8>, source: &[u8]) {
        // Node kind (2 bytes)
        output.extend_from_slice(&node.kind_id().to_le_bytes());
        
        // Start byte (4 bytes)
        output.extend_from_slice(&(node.start_byte() as u32).to_le_bytes());
        
        // End byte (4 bytes)
        output.extend_from_slice(&(node.end_byte() as u32).to_le_bytes());
        
        // Child count (2 bytes)
        output.extend_from_slice(&node.child_count().to_le_bytes());
        
        // Serialize children
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                serialize_node(child, output, source);
            }
        }
    }
    
    serialize_node(tree.root_node(), &mut serialized, source);
    
    serialized
}

fn main() {
    println!("=====================================");
    println!(" TREE COMPRESSION TEST");
    println!(" Finding how to reduce 7.5 GB");
    println!("=====================================\n");

    let rt = tokio::runtime::Runtime::new().unwrap();
    
    rt.block_on(async {
        let manager = Arc::new(NativeParserManager::new().unwrap());
        
        // Test with different file sizes
        let test_cases = vec![
            ("Small (100 lines)", generate_code(100)),
            ("Medium (500 lines)", generate_code(500)),
            ("Large (1000 lines)", generate_code(1000)),
        ];
        
        println!("Testing compression on different file sizes:\n");
        
        for (name, code) in test_cases {
            let temp_file = std::env::temp_dir().join("compress_test.rs");
            std::fs::write(&temp_file, &code).unwrap();
            
            // Measure uncompressed
            let baseline = get_rss_kb();
            
            let result = manager.parse_file(&temp_file).await.unwrap();
            let tree = result.tree;
            let source = code.as_bytes();
            
            let after_parse = get_rss_kb();
            let uncompressed_mem = after_parse.saturating_sub(baseline);
            
            // Serialize
            let serialized = serialize_tree(&tree, source);
            
            // Compress with different algorithms
            let zstd_compressed = zstd::encode_all(&serialized[..], 3).unwrap();
            let lz4_compressed = lz4::block::compress(&serialized, None, false).unwrap();
            
            println!("📄 {}", name);
            println!("  Source: {} bytes", source.len());
            println!("  Uncompressed (in memory): {} KB", uncompressed_mem);
            println!("  Serialized: {} bytes", serialized.len());
            println!("  ZSTD compressed: {} bytes ({}x smaller)", 
                zstd_compressed.len(), 
                serialized.len() / zstd_compressed.len());
            println!("  LZ4 compressed: {} bytes ({}x smaller)", 
                lz4_compressed.len(),
                serialized.len() / lz4_compressed.len());
            println!("  Memory reduction: {:.1}x (uncompressed to ZSTD)", 
                uncompressed_mem as f64 / (zstd_compressed.len() as f64 / 1024.0));
            println!();
            
            std::fs::remove_file(&temp_file).ok();
        }
        
        println!("=====================================");
        println!(" SCALING TO 10K FILES");
        println!("=====================================\n");
        
        // Calculate for 10K files × 1K lines
        let avg_source_bytes = 30_000; // ~1K lines
        let avg_uncompressed_kb = 750; // from our test: 768 KB per file
        let avg_serialized = avg_source_bytes + (avg_source_bytes / 3); // rough estimate
        let compression_ratio = 8; // ZSTD typically gets 8x on code
        let avg_compressed = avg_serialized / compression_ratio;
        
        let total_uncompressed_mb = avg_uncompressed_kb * 10_000 / 1024;
        let total_compressed_mb = avg_compressed * 10_000 / 1024 / 1024;
        
        println!("For 10,000 files × 1K lines:");
        println!("  Current approach (in-memory trees): {} MB", total_uncompressed_mb);
        println!("  Serialized + ZSTD compressed: ~{} MB", total_compressed_mb);
        println!("  Reduction: {}x", total_uncompressed_mb / total_compressed_mb);
        println!();
        
        println!("Options:");
        println!("  1. Store compressed, decompress on use");
        println!("     Memory: ~{} MB", total_compressed_mb);
        println!("     Cost: Decompression CPU time");
        println!();
        println!("  2. LRU cache of hot trees + compressed cold trees");
        println!("     Memory: ~{} MB (1K hot) + ~{} MB (9K compressed)", 
            avg_uncompressed_kb * 1000 / 1024,
            avg_compressed * 9000 / 1024 / 1024);
        println!("     Total: ~{} MB", 
            (avg_uncompressed_kb * 1000 / 1024) + (avg_compressed * 9000 / 1024 / 1024));
        println!();
        println!("  3. Store on disk, LRU cache in memory");
        println!("     Memory: ~{} MB (1K hot trees only)", 
            avg_uncompressed_kb * 1000 / 1024);
        println!("     Disk: ~{} MB compressed", total_compressed_mb);
        println!();
        
        if total_compressed_mb < 800 {
            println!("✅ With compression, we can fit under 800 MB!");
        } else {
            println!("⚠️  Even compressed, still over 800 MB target");
        }
    });
}

fn generate_code(lines: usize) -> String {
    let mut code = String::new();
    code.push_str("// Test file\n");
    
    for i in 0..lines/5 {
        code.push_str(&format!(r#"
fn function_{}(x: i32, y: i32) -> i32 {{
    let result = x + y + {};
    println!("Result: {{}}", result);
    result
}}
"#, i, i));
    }
    
    code
}
