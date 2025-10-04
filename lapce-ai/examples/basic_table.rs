// Simplest possible LanceDB test - just create a table
use lancedb::connect;
use arrow_array::{StringArray, RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing basic table creation...");
    
    // Create temp dir
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    println!("DB path: {}", db_path);
    
    // Connect
    println!("Connecting...");
    let db = connect(db_path).execute().await?;
    println!("✅ Connected");
    
    // Simple schema - just strings
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    
    // Create data
    let ids = StringArray::from(vec!["1", "2", "3"]);
    let names = StringArray::from(vec!["Alice", "Bob", "Charlie"]);
    
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![Arc::new(ids), Arc::new(names)],
    )?;
    
    println!("Creating table...");
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
    let _table = db.create_table("users", reader)
        .execute()
        .await?;
    println!("✅ Table created");
    
    // List tables
    let tables = db.table_names().execute().await?;
    println!("Tables: {:?}", tables);
    
    println!("\n✅ SUCCESS - Basic LanceDB operations work!");
    
    Ok(())
}
