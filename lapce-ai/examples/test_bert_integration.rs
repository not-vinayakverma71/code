// Test real BERT integration with performance metrics
// use lapce_ai_rust::lancedb // Module not available
// Original: use lapce_ai_rust::lancedb::bert_integration::BertEmbedder;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Testing BERT Integration with Real Weights\n");
    
    let model_path = Path::new("models/bert-base-uncased");
    if !model_path.exists() {
        eprintln!("❌ Model not found at {:?}", model_path);
        eprintln!("Run: python download_bert.py");
        return Ok(());
    }
    
    println!("Loading BERT model from {:?}...", model_path);
    let start = Instant::now();
    let embedder = BertEmbedder::new(model_path)?;
    let load_time = start.elapsed();
    println!("✅ Model loaded in {:?}\n", load_time);
    
    // Test single encoding
    println!("Testing single text encoding:");
    let text = "semantic search implementation";
    let start = Instant::now();
    let embedding = embedder.encode(text)?;
    let encode_time = start.elapsed();
    
    println!("  Text: '{}'", text);
    println!("  Embedding dims: {}", embedding.len());
    println!("  Encoding time: {:?}", encode_time);
    println!("  Latency: {:.2}ms", encode_time.as_millis());
    
    // Test batch encoding
    println!("\nTesting batch encoding:");
    let texts = vec![
        "async function implementation".to_string(),
        "error handling in rust".to_string(),
        "memory optimization techniques".to_string(),
        "vector database indexing".to_string(),
        "concurrent query processing".to_string(),
    ];
    
    let start = Instant::now();
    let embeddings = embedder.encode_batch(&texts)?;
    let batch_time = start.elapsed();
    
    println!("  Batch size: {}", texts.len());
    println!("  Total time: {:?}", batch_time);
    println!("  Per-text latency: {:.2}ms", batch_time.as_millis() as f64 / texts.len() as f64);
    
    // Memory usage check
    println!("\n📊 Performance Summary:");
    println!("  Model load time: {:?}", load_time);
    println!("  Single encode: {:.2}ms", encode_time.as_millis());
    println!("  Batch encode (5 texts): {:.2}ms", batch_time.as_millis());
    println!("  Avg per text in batch: {:.2}ms", batch_time.as_millis() as f64 / 5.0);
    
    // Check against target
    let avg_latency = batch_time.as_millis() as f64 / 5.0;
    if avg_latency < 5.0 {
        println!("\n✅ MEETS <5ms target!");
    } else {
        println!("\n❌ EXCEEDS 5ms target ({:.2}ms)", avg_latency);
    }
    
    Ok(())
}
