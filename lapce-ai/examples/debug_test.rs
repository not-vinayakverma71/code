// Minimal debug test to identify the hanging issue
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting debug test...");
    let start = Instant::now();
    
    // Test 1: Basic async runtime
    println!("1. Testing async runtime...");
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    println!("   ✅ Async works");
    
    // Test 2: TempDir creation
    println!("2. Testing TempDir...");
    let temp_dir = tempfile::TempDir::new()?;
    println!("   ✅ TempDir created: {:?}", temp_dir.path());
    
    // Test 3: LanceDB connection
    println!("3. Testing LanceDB connection...");
    let db_path = temp_dir.path().join("lancedb");
    std::fs::create_dir_all(&db_path)?;
    
    println!("   Connecting to: {:?}", db_path);
    let db = ::lancedb::connect(db_path.to_str().unwrap())
        .execute()
        .await?;
    println!("   ✅ LanceDB connected");
    
    // Test 4: Simple table operations
    println!("4. Testing table operations...");
    let tables = db.table_names().execute().await?;
    println!("   Tables: {:?}", tables);
    println!("   ✅ Table listing works");
    
    let elapsed = start.elapsed();
    println!("\n✅ All tests passed in {:?}", elapsed);
    
    Ok(())
}
