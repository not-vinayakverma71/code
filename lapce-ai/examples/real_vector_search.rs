// Real working vector search with LanceDB
use lancedb::{connect, query::{QueryBase, ExecutableQuery}};
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Real LanceDB Vector Search Test\n");
    
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Connect
    let db = connect(db_path).execute().await?;
    println!("✅ Connected to LanceDB");
    
    // Create schema with vectors
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            3, // Simple 3D vectors for testing
        ), false),
    ]));
    
    // Create test data with vectors
    let ids = StringArray::from(vec!["1", "2", "3", "4", "5"]);
    let texts = StringArray::from(vec![
        "rust programming", 
        "python machine learning",
        "javascript web",
        "rust async",
        "python data science"
    ]);
    
    // Create simple 3D vectors
    let vectors_data = vec![
        1.0, 0.0, 0.0,  // rust programming
        0.0, 1.0, 0.0,  // python ML
        0.0, 0.0, 1.0,  // javascript
        0.9, 0.1, 0.0,  // rust async (similar to rust)
        0.1, 0.9, 0.0,  // python data (similar to python)
    ];
    
    let vector_array = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        3,
        Arc::new(Float32Array::from(vectors_data)),
        None
    );
    
    // Create batch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ids),
            Arc::new(texts),
            Arc::new(vector_array),
        ],
    )?;
    
    // Create table
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
    let table = db.create_table("documents", reader)
        .execute()
        .await?;
    println!("✅ Table created with 5 documents");
    
    // Create a query vector (looking for rust-like documents)
    let query_vector = vec![0.95_f32, 0.05, 0.0];
    println!("\n🔍 Searching for vector: {:?}", query_vector);
    
    // Perform vector search
    let search_start = Instant::now();
    let mut results = table
        .query()
        .nearest_to(query_vector)?
        .limit(3)
        .execute()
        .await?;
    
    // Collect results
    let mut batches: Vec<RecordBatch> = Vec::new();
    use futures_util::StreamExt;
    while let Some(batch) = results.next().await {
        batches.push(batch?);
    }
    let search_time = search_start.elapsed();
    
    println!("✅ Search completed in {:?}", search_time);
    println!("   Latency: {:.2}ms\n", search_time.as_secs_f64() * 1000.0);
    
    // Display results
    println!("📊 Top 3 Results:");
    for batch in &batches {
        let ids = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
        let texts = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        
        for i in 0..batch.num_rows() {
            println!("   ID: {}, Text: {}", ids.value(i), texts.value(i));
        }
    }
    
    // Performance check
    let latency_ms = search_time.as_secs_f64() * 1000.0;
    println!("\n📈 Performance Metrics:");
    println!("   Query latency: {:.2}ms", latency_ms);
    if latency_ms < 5.0 {
        println!("   ✅ PASSED: Query latency < 5ms");
    } else {
        println!("   ⚠️  SLOW: Target is <5ms");
    }
    
    Ok(())
}
