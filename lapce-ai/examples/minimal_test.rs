// Absolute minimal test to find the hanging point
fn main() {
    println!("1. Starting...");
    
    // Test basic runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    println!("2. Tokio runtime created");
    
    // Test async execution
    rt.block_on(async {
        println!("3. Inside async block");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        println!("4. After sleep");
    });
    
    println!("5. Done!");
}
