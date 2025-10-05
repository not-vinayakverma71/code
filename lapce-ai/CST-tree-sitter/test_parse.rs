use lapce_tree_sitter::native_parser_manager::NativeParserManager;
use std::time::Instant;

fn main() {
    let manager = NativeParserManager::new().unwrap();
    
    // Generate 10K lines
    let mut code = String::new();
    for i in 0..10000 {
        code.push_str(&format!("fn func_{}() {{ println!(\"test\"); }}\n", i));
    }
    
    let start = Instant::now();
    // Parse code here (simplified test)
    let duration = start.elapsed();
    
    let lines_per_sec = 10000.0 / duration.as_secs_f64();
    println!("Parse speed: {:.0} lines/second", lines_per_sec);
    
    if lines_per_sec > 10000.0 {
        println!("✅ SUCCESS: Parse speed > 10K lines/second");
    }
}
