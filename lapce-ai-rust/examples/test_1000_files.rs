// Test indexing 1000 files for performance
use lancedb::{connect, query::{QueryBase, ExecutableQuery}};
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing 1000 File Indexing Performance\n");
    
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Connect
    let db = connect(db_path).execute().await?;
    
    // Schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            128,
        ), false),
    ]));
    
    // Generate 1000 files worth of data
    println!("Generating 1000 test documents...");
    let mut ids = Vec::new();
    let mut paths = Vec::new();
    let mut contents = Vec::new();
    let mut vectors_flat = Vec::new();
    
    for i in 0..1000 {
        ids.push(format!("doc_{:04}", i));
        paths.push(format!("/src/file_{:04}.rs", i));
        contents.push(format!("fn function_{}() {{ /* code */ }}", i));
        
        // Generate mock 128-dim vector
        for j in 0..128 {
            let val = ((i as f32 * 7.0 + j as f32 * 3.0) % 100.0) / 100.0;
            vectors_flat.push(val);
        }
    }
    
    // Create batch
    let id_array = StringArray::from(ids);
    let path_array = StringArray::from(paths);
    let content_array = StringArray::from(contents);
    let vector_array = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        128,
        Arc::new(Float32Array::from(vectors_flat)),
        None
    );
    
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(id_array),
            Arc::new(path_array),
            Arc::new(content_array),
            Arc::new(vector_array),
        ],
    )?;
    
    // Time indexing
    println!("Indexing 1000 documents...");
    let index_start = Instant::now();
    
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
    let table = db.create_table("large_test", reader)
        .execute()
        .await?;
    
    let index_time = index_start.elapsed();
    let files_per_sec = 1000.0 / index_time.as_secs_f64();
    
    println!("✅ Indexed 1000 documents in {:?}", index_time);
    println!("   Speed: {:.1} files/sec", files_per_sec);
    
    // Test query on large dataset
    println!("\n🔍 Testing query on 1000 documents...");
    let query_vec = vec![0.5f32; 128];
    
    let query_start = Instant::now();
    let mut results = table
        .query()
        .nearest_to(query_vec)?
        .limit(10)
        .execute()
        .await?;
    
    use futures_util::StreamExt;
    let mut batches = Vec::new();
    while let Some(batch) = results.next().await {
        batches.push(batch?);
    }
    let query_time = query_start.elapsed();
    
    println!("✅ Query completed in {:?}", query_time);
    println!("   Latency: {:.2}ms", query_time.as_secs_f64() * 1000.0);
    
    // Performance summary
    println!("\n📊 Performance Summary:");
    println!("   Documents indexed: 1000");
    println!("   Indexing speed: {:.1} files/sec", files_per_sec);
    println!("   Query latency: {:.2}ms", query_time.as_secs_f64() * 1000.0);
    
    // Check against targets
    if files_per_sec >= 1000.0 {
        println!("   ✅ PASSED: Indexing speed >= 1000 files/sec");
    } else {
        println!("   ❌ FAILED: Target is 1000+ files/sec, got {:.1}", files_per_sec);
    }
    
    if query_time.as_millis() < 5 {
        println!("   ✅ PASSED: Query latency < 5ms");
    } else {
        println!("   ⚠️  SLOW: Query took {}ms (target: <5ms)", query_time.as_millis());
    }
    
    Ok(())
}
