// Test query optimization with IVF_PQ
use lancedb::{connect, query::{QueryBase, ExecutableQuery}};
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;
use std::time::Instant;
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Query Optimization for <5ms Latency\n");
    
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    let db = connect(db_path).execute().await?;
    
    // Create schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            768,
        ), false),
    ]));
    
    // Generate test data
    println!("Generating 10,000 documents...");
    let mut ids = Vec::new();
    let mut texts = Vec::new();
    let mut vectors_flat = Vec::new();
    
    for i in 0..10000 {
        ids.push(format!("doc_{:05}", i));
        texts.push(format!("content {}", i));
        
        // Generate normalized vector
        let mut vec = Vec::with_capacity(768);
        for j in 0..768 {
            vec.push(((i as f32 * 7.0 + j as f32 * 3.0) % 100.0) / 100.0);
        }
        
        // L2 normalize
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        for v in &vec {
            vectors_flat.push(v / norm);
        }
    }
    
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(texts)),
            Arc::new(FixedSizeListArray::new(
                Arc::new(Field::new("item", DataType::Float32, true)),
                768,
                Arc::new(Float32Array::from(vectors_flat)),
                None
            )),
        ],
    )?;
    
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
    let table = db.create_table("optimized_test", reader)
        .execute()
        .await?;
    
    println!("✅ Indexed 10,000 documents\n");
    
    // Test different optimization strategies
    let query_vec = vec![0.5f32; 768];
    
    // Strategy 1: Default query
    println!("1️⃣ Default query (no optimization):");
    let start = Instant::now();
    let mut results = table
        .query()
        .nearest_to(query_vec.clone())?
        .limit(10)
        .execute()
        .await?;
    while let Some(batch) = results.next().await {
        let _ = batch?;
    }
    let default_time = start.elapsed();
    println!("   Time: {:.2}ms", default_time.as_secs_f64() * 1000.0);
    
    // Strategy 2: With nprobes (reduced search space)
    println!("\n2️⃣ Query with nprobes=5:");
    let start = Instant::now();
    let mut results = table
        .query()
        .nearest_to(query_vec.clone())?
        .limit(10)
        .nprobes(5)  // Search only 5 partitions
        .execute()
        .await?;
    while let Some(batch) = results.next().await {
        let _ = batch?;
    }
    let nprobes_time = start.elapsed();
    println!("   Time: {:.2}ms", nprobes_time.as_secs_f64() * 1000.0);
    
    // Strategy 3: With refine factor
    println!("\n3️⃣ Query with refine_factor=2:");
    let start = Instant::now();
    let mut results = table
        .query()
        .nearest_to(query_vec.clone())?
        .limit(10)
        .refine_factor(2)  // Less refinement
        .execute()
        .await?;
    while let Some(batch) = results.next().await {
        let _ = batch?;
    }
    let refine_time = start.elapsed();
    println!("   Time: {:.2}ms", refine_time.as_secs_f64() * 1000.0);
    
    // Strategy 4: Combined optimizations
    println!("\n4️⃣ Combined optimizations:");
    let start = Instant::now();
    let mut results = table
        .query()
        .nearest_to(query_vec)?
        .limit(10)
        .nprobes(3)
        .refine_factor(1)
        .execute()
        .await?;
    while let Some(batch) = results.next().await {
        let _ = batch?;
    }
    let combined_time = start.elapsed();
    println!("   Time: {:.2}ms", combined_time.as_secs_f64() * 1000.0);
    
    // Results
    println!("\n📊 Performance Summary:");
    println!("   Default:   {:.2}ms", default_time.as_secs_f64() * 1000.0);
    println!("   Nprobes:   {:.2}ms ({:.1}x speedup)", 
             nprobes_time.as_secs_f64() * 1000.0,
             default_time.as_secs_f64() / nprobes_time.as_secs_f64());
    println!("   Refine:    {:.2}ms ({:.1}x speedup)", 
             refine_time.as_secs_f64() * 1000.0,
             default_time.as_secs_f64() / refine_time.as_secs_f64());
    println!("   Combined:  {:.2}ms ({:.1}x speedup)", 
             combined_time.as_secs_f64() * 1000.0,
             default_time.as_secs_f64() / combined_time.as_secs_f64());
    
    // Check if we achieved <5ms
    let best_time = combined_time.min(nprobes_time).min(refine_time);
    if best_time.as_millis() < 5 {
        println!("\n✅ SUCCESS: Achieved <5ms query latency!");
    } else {
        println!("\n❌ FAILED: Best time {}ms (target: <5ms)", best_time.as_millis());
        println!("   Need more aggressive optimizations or IVF_PQ index");
    }
    
    Ok(())
}
