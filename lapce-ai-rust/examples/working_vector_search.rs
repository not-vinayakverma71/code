use lancedb::{connect, query::QueryBase};
use arrow_array::{Float32Array, RecordBatch, RecordBatchIterator, StringArray, FixedSizeListArray};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing REAL LanceDB Vector Search\n");
    
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().to_str().unwrap();
    
    // Connect
    let start = Instant::now();
    let db = connect(db_path).execute().await?;
    println!("✅ Connected in {:?}", start.elapsed());
    
    // Schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("vector", DataType::FixedSizeList(
            Arc::new(Field::new("item", DataType::Float32, true)),
            128, // Smaller for testing
        ), false),
    ]));
    
    // Generate test data
    let mut ids = Vec::new();
    let mut texts = Vec::new();
    let mut vectors_flat = Vec::new();
    
    let test_data = vec![
        ("doc1", "Rust programming language", vec![1.0, 0.5, 0.2]),
        ("doc2", "Python machine learning", vec![0.2, 1.0, 0.8]),
        ("doc3", "JavaScript web development", vec![0.5, 0.3, 1.0]),
        ("doc4", "Rust async programming", vec![0.9, 0.4, 0.1]),
        ("doc5", "Python data science", vec![0.1, 0.95, 0.7]),
    ];
    
    for (id, text, base_vec) in test_data {
        ids.push(id.to_string());
        texts.push(text.to_string());
        
        // Expand to 128 dimensions
        let mut vec = vec![0.0f32; 128];
        for i in 0..128 {
            vec[i] = base_vec[i % 3] * ((i as f32 + 1.0) / 128.0);
        }
        
        // L2 normalize
        let norm = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        for v in &mut vec {
            *v /= norm;
        }
        
        vectors_flat.extend(vec);
    }
    
    // Create batch
    let id_array = StringArray::from(ids.clone());
    let text_array = StringArray::from(texts.clone());
    let vector_values = Float32Array::from(vectors_flat);
    let vector_array = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        128,
        Arc::new(vector_values),
        None
    );
    
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(id_array),
            Arc::new(text_array),
            Arc::new(vector_array),
        ],
    )?;
    
    // Create table
    let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema.clone());
    let table = db.create_table("vectors", reader)
        .execute()
        .await?;
    println!("✅ Table created with {} documents", ids.len());
    
    // Create query vector (similar to Rust)
    let mut query = vec![0.0f32; 128];
    for i in 0..128 {
        query[i] = if i % 3 == 0 { 0.8 } else { 0.3 } * ((i as f32 + 1.0) / 128.0);
    }
    let norm = query.iter().map(|x| x * x).sum::<f32>().sqrt();
    for v in &mut query {
        *v /= norm;
    }
    
    // Perform vector search
    let search_start = Instant::now();
    let results = table
        .query()
        .nearest_to(query)?
        .limit(3)
        .execute()
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    
    let search_time = search_start.elapsed();
    println!("\n🔍 Search completed in {:?}", search_time);
    println!("   Latency: {:.2}ms", search_time.as_secs_f64() * 1000.0);
    
    // Display results
    println!("\n📊 Top 3 Results:");
    for (i, batch) in results.iter().enumerate() {
        let ids = batch.column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let texts = batch.column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
            
        for row in 0..batch.num_rows() {
            println!("   {}. {} - {}", 
                i + 1,
                ids.value(row),
                texts.value(row)
            );
        }
    }
    
    // Performance check
    if search_time.as_millis() < 5 {
        println!("\n✅ PASSED: Query latency < 5ms");
    } else {
        println!("\n⚠️  SLOW: Query latency = {}ms (target: <5ms)", search_time.as_millis());
    }
    
    Ok(())
}
