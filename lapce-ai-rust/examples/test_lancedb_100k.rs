/// Run 100K+ Files Production Test
/// Tests all 8 performance requirements on real code

use lapce_ai_rust::lancedb::test_100k_files::run_100k_production_test;

fn main() {
    println!("🚀 Starting 100K+ Files Production Test");
    println!("This will create and index 120,000 real code files");
    println!("Expected time: 2-5 minutes\n");
    
    run_100k_production_test();
    
    println!("\n✅ Test completed!");
}
