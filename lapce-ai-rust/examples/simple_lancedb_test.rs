// Simplified LanceDB test - just connection and basic operations
use lancedb::connect;
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing basic LanceDB operations...");
    
    // Create temp directory
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Connect to LanceDB
    let db = connect(db_path).execute().await?;
    println!("✅ Connected to LanceDB");
    
    // Define simple schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("embedding", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            3, // Small vector for testing
        ), false),
    ]));
    
    // Create data
    let ids = StringArray::from(vec!["1", "2"]);
    let vector_values = Float32Array::from(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    let embeddings = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        3,
        Arc::new(vector_values),
        None
    );
    
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ids),
            Arc::new(embeddings),
        ],
    )?;
    
    // Create table
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema.clone());
    let _table = db.create_table("test", reader)
        .execute()
        .await?;
    println!("✅ Created table");
    
    // List tables
    let tables = db.table_names().execute().await?;
    println!("✅ Tables: {:?}", tables);
    
    println!("\n🎉 Basic LanceDB test passed!");
    Ok(())
}
