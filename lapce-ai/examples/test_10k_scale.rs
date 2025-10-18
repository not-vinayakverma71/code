// Test 10K file scale and memory usage
use lancedb::{connect, query::{QueryBase, ExecutableQuery}};
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;
use std::time::Instant;

fn get_memory_usage() -> f64 {
    // Read from /proc/self/status
    use std::fs;
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<f64>().unwrap_or(0.0) / 1024.0; // Convert KB to MB
            }
        }
    }
    0.0
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing 10K File Scale & Memory Usage\n");
    
    let mem_start = get_memory_usage();
    println!("Initial memory: {:.1} MB", mem_start);
    
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    let db = connect(db_path).execute().await?;
    
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            128,
        ), false),
    ]));
    
    println!("Generating 10,000 documents...");
    let gen_start = Instant::now();
    
    // Process in batches of 1000 to avoid huge memory spike
    let mut total_index_time = std::time::Duration::ZERO;
    
    for batch_num in 0..10 {
        let batch_start = batch_num * 1000;
        let batch_end = batch_start + 1000;
        
        let mut ids = Vec::new();
        let mut paths = Vec::new();
        let mut contents = Vec::new();
        let mut vectors_flat = Vec::new();
        
        for i in batch_start..batch_end {
            ids.push(format!("doc_{:05}", i));
            paths.push(format!("/src/module_{}/file_{:05}.rs", i / 100, i));
            contents.push(format!(
                "// File {}\nfn process_{}() {{\n    // Processing logic\n    let result = compute({});\n}}", 
                i, i, i
            ));
            
            // Generate vector
            for j in 0..128 {
                let val = ((i as f32 * 7.0 + j as f32 * 3.0) % 100.0) / 100.0;
                vectors_flat.push(val);
            }
        }
        
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(StringArray::from(paths)),
                Arc::new(StringArray::from(contents)),
                Arc::new(FixedSizeListArray::new(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    128,
                    Arc::new(Float32Array::from(vectors_flat)),
                    None
                )),
            ],
        )?;
        
        let index_start = Instant::now();
        let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema.clone());
        
        if batch_num == 0 {
            db.create_table("scale_test", reader)
                .execute()
                .await?;
        } else {
            let table = db.open_table("scale_test").execute().await?;
            let batches: Vec<RecordBatch> = reader.collect::<Result<Vec<_>, _>>()?;
            for batch in batches {
                let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema.clone());
                table.add(reader).execute().await?;
            }
        }
        
        let batch_time = index_start.elapsed();
        total_index_time += batch_time;
        
        if batch_num % 2 == 0 {
            let mem_current = get_memory_usage();
            println!("  Batch {}/10: {:.2?}, Memory: {:.1} MB", 
                     batch_num + 1, batch_time, mem_current);
        }
    }
    
    let gen_time = gen_start.elapsed();
    let files_per_sec = 10000.0 / total_index_time.as_secs_f64();
    
    println!("\n✅ Indexed 10,000 documents");
    println!("   Total time: {:?}", gen_time);
    println!("   Index time: {:?}", total_index_time);
    println!("   Speed: {:.1} files/sec", files_per_sec);
    
    // Memory after indexing
    let mem_after_index = get_memory_usage();
    let mem_used = mem_after_index - mem_start;
    println!("\n📊 Memory Usage:");
    println!("   After indexing: {:.1} MB", mem_after_index);
    println!("   Memory used: {:.1} MB", mem_used);
    
    // Test query performance at scale
    println!("\n🔍 Testing queries on 10K documents...");
    let table = db.open_table("scale_test").execute().await?;
    
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
    
    // Final memory
    let mem_final = get_memory_usage();
    println!("\n📊 Final Memory: {:.1} MB", mem_final);
    
    // Performance summary
    println!("\n📈 Performance Summary:");
    println!("   Documents: 10,000");
    println!("   Indexing: {:.1} files/sec", files_per_sec);
    println!("   Query latency: {:.2}ms", query_time.as_secs_f64() * 1000.0);
    println!("   Memory used: {:.1} MB", mem_used);
    
    // Check targets
    if files_per_sec >= 1000.0 {
        println!("   ✅ PASSED: Indexing >= 1000 files/sec");
    } else {
        println!("   ❌ FAILED: Indexing {:.1} files/sec (target: 1000+)", files_per_sec);
    }
    
    if query_time.as_millis() < 5 {
        println!("   ✅ PASSED: Query < 5ms");
    } else {
        println!("   ❌ FAILED: Query {}ms (target: <5ms)", query_time.as_millis());
    }
    
    if mem_used < 10.0 {
        println!("   ✅ PASSED: Memory < 10MB");
    } else {
        println!("   ❌ FAILED: Memory {:.1}MB (target: <10MB)", mem_used);
    }
    
    Ok(())
}
