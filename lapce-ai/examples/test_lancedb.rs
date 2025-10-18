use lancedb::{connect, arrow::IntoArrow};
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing LanceDB connection...");
    
    // Create a temporary directory for testing
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Connect to LanceDB
    let db = connect(db_path).execute().await?;
    println!("✅ Connected to LanceDB at: {}", db_path);
    
    // Define schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            768,
        ), false),
    ]));
    
    // Create initial data for table creation
    let init_ids = StringArray::from(vec!["init"]);
    let init_texts = StringArray::from(vec!["initial"]);
    
    // Create proper FixedSizeListArray for vectors
    let vector_values = Float32Array::from(vec![0.0f32; 768]);
    let init_vectors = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        768,
        Arc::new(vector_values),
        None
    );
    
    let init_batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(init_ids),
            Arc::new(init_texts),
            Arc::new(init_vectors),
        ],
    )?;
    
    // Create table with RecordBatchIterator
    let batches = vec![init_batch];
    let reader = RecordBatchIterator::new(batches.into_iter().map(Ok), schema.clone());
    
    let table = db.create_table("test_embeddings", reader)
        .execute()
        .await?;
    println!("✅ Created table 'test_embeddings'");
    
    // Create sample data
    let ids = StringArray::from(vec!["1", "2", "3"]);
    let texts = StringArray::from(vec!["hello world", "test document", "sample text"]);
    
    // Create dummy 768-dim vectors
    let mut vector_data = Vec::new();
    for i in 0..3 {
        for j in 0..768 {
            vector_data.push((i as f32 + j as f32) / 768.0);
        }
    }
    let vector_values = Float32Array::from(vector_data);
    let vectors = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        768,
        Arc::new(vector_values),
        None
    );
    
    // Create record batch
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(texts),
            Arc::new(vectors),
        ],
    )?;
    
    // Insert data
    table.add(vec![batch]).execute().await?;
    println!("✅ Inserted 3 records");
    
    // Query data
    let results = table.query()
        .limit(10)
        .execute()
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    
    println!("✅ Query returned {} batches", results.len());
    for batch in &results {
        println!("  Batch has {} rows", batch.num_rows());
    }
    
    // Create IVF_PQ index
    table.create_index(&["vector"], lancedb::index::Index::IvfPq)
        .num_partitions(10)
        .num_sub_vectors(8)
        .execute()
        .await?;
    println!("✅ Created IVF_PQ index");
    
    // Test vector search
    let query_vector = vec![0.5f32; 768];
    let search_results = table.search(&query_vector)
        .limit(2)
        .execute()
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    
    println!("✅ Vector search returned {} results", search_results.len());
    
    println!("\n🎉 All LanceDB tests passed!");
    
    Ok(())
}
