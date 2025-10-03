//! Simple synchronous test - REAL tree-sitter parsing
use std::fs;
use std::io::{self, Write};
use tree_sitter::{Parser, Tree};
use walkdir::WalkDir;

fn main() {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "🔬 REAL CST MEMORY TEST - /home/verma/lapce/Codex").unwrap();
    writeln!(handle, "================================================================================\n").unwrap();
    handle.flush().unwrap();
    
    // Collect files
    let files: Vec<_> = WalkDir::new("/home/verma/lapce/Codex")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                matches!(ext.to_str(), Some("ts") | Some("js") | Some("tsx") | Some("jsx"))
            } else {
                false
            }
        })
        .map(|e| e.path().to_owned())
        .take(1000)  // Test 1000 files
        .collect();
    
    writeln!(handle, "Found {} TypeScript/JavaScript files\n", files.len()).unwrap();
    handle.flush().unwrap();
    
    // Parse with REAL tree-sitter
    let mut js_parser = Parser::new();
    let mut ts_parser = Parser::new();
    
    // Use the same pattern as lib.rs line 97
    let js_lang = unsafe { tree_sitter_javascript::LANGUAGE };
    let ts_lang = unsafe { tree_sitter_typescript::LANGUAGE_TYPESCRIPT };
    
    writeln!(handle, "Setting up parsers...").unwrap();
    handle.flush().unwrap();
    
    if let Err(e) = js_parser.set_language(&js_lang.into()) {
        writeln!(handle, "Failed to set JS language: {:?}", e).unwrap();
        return;
    }
    
    if let Err(e) = ts_parser.set_language(&ts_lang.into()) {
        writeln!(handle, "Failed to set TS language: {:?}", e).unwrap();
        return;
    }
    
    writeln!(handle, "Parsers ready!\n").unwrap();
    handle.flush().unwrap();
    
    let mut trees: Vec<(Tree, Vec<u8>)> = Vec::new();
    let mut parsed = 0;
    let mut failed = 0;
    let mut total_source_bytes = 0;
    
    writeln!(handle, "Parsing files...").unwrap();
    handle.flush().unwrap();
    for (i, path) in files.iter().enumerate() {
        if (i + 1) % 100 == 0 {
            writeln!(handle, "  Progress: {}/{}", i + 1, files.len()).unwrap();
            handle.flush().unwrap();
        }
        
        if let Ok(source) = fs::read(&path) {
            let parser = if path.extension().and_then(|s| s.to_str()) == Some("ts") ||
                          path.extension().and_then(|s| s.to_str()) == Some("tsx") {
                &mut ts_parser
            } else {
                &mut js_parser
            };
            
            if let Some(tree) = parser.parse(&source, None) {
                total_source_bytes += source.len();
                trees.push((tree, source));
                parsed += 1;
            } else {
                failed += 1;
            }
        } else {
            failed += 1;
        }
    }
    
    writeln!(handle, "\n📊 RESULTS:\n").unwrap();
    writeln!(handle, "Files parsed: {} ({:.1}%)", parsed, (parsed as f64 / files.len() as f64) * 100.0).unwrap();
    writeln!(handle, "Files failed: {}", failed).unwrap();
    writeln!(handle, "\n🔬 MEMORY MEASUREMENT:\n").unwrap();
    writeln!(handle, "Source code size: {:.2} MB", total_source_bytes as f64 / 1_048_576.0).unwrap();
    
    // Count nodes
    let mut total_nodes = 0;
    for (tree, _) in &trees {
        total_nodes += count_nodes(tree.root_node());
    }
    
    // Calculate memory
    let tree_structure_memory = total_nodes * 50; // ~50 bytes per node
    let total_cst_memory = total_source_bytes + tree_structure_memory;
    
    writeln!(handle, "Total nodes in all CSTs: {}", total_nodes).unwrap();
    writeln!(handle, "Avg nodes per file: {:.0}", total_nodes as f64 / parsed as f64).unwrap();
    writeln!(handle, "\n💾 CST MEMORY (ALL {} TREES STORED IN RAM):", parsed).unwrap();
    writeln!(handle, "  Source code: {:.2} MB", total_source_bytes as f64 / 1_048_576.0).unwrap();
    writeln!(handle, "  Tree structures: {:.2} MB (~50 bytes/node)", tree_structure_memory as f64 / 1_048_576.0).unwrap();
    writeln!(handle, "  TOTAL CST MEMORY: {:.2} MB", total_cst_memory as f64 / 1_048_576.0).unwrap();
    writeln!(handle, "  Memory overhead: {:.1}x source size", total_cst_memory as f64 / total_source_bytes as f64).unwrap();
    writeln!(handle, "\n🎯 ANSWER: {} Codex files require {:.2} MB of CST memory", 
        parsed, total_cst_memory as f64 / 1_048_576.0).unwrap();
    handle.flush().unwrap();
}

fn count_nodes(node: tree_sitter::Node) -> usize {
    let mut count = 1;
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            count += count_nodes(cursor.node());
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    count
}
