/// Test and fix bloom filter accuracy
use crate::cache::bloom_filter::BloomFilter;
use crate::cache::types::CacheKey;

fn main() {
    println!("=== TESTING BLOOM FILTER ACCURACY ===\n");
    
    // Test 1: Basic functionality
    println!("Test 1: Basic functionality");
    let mut bf = BloomFilter::new(10_000, 0.01);
    
    let key1 = CacheKey("test_key_1".to_string());
    let key2 = CacheKey("test_key_2".to_string());
    
    println!("  Inserting key1");
    bf.insert(&key1);
    
    println!("  Contains key1: {}", bf.contains(&key1));
    println!("  Contains key2: {}", bf.contains(&key2));
    
    // Test 2: Accuracy test
    println!("\nTest 2: Accuracy with 1000 items");
    let mut bf2 = BloomFilter::new(1000, 0.01);
    
    // Insert 500 items
    for i in 0..500 {
        let key = CacheKey(format!("key_{}", i));
        bf2.insert(&key);
    }
    
    // Test accuracy
    let mut true_positives = 0;
    let mut false_positives = 0;
    let mut true_negatives = 0;
    
    for i in 0..1000 {
        let key = CacheKey(format!("key_{}", i));
        let contains = bf2.contains(&key);
        
        if i < 500 {
            // Should be in filter
            if contains {
                true_positives += 1;
            }
        } else {
            // Should NOT be in filter
            if !contains {
                true_negatives += 1;
            } else {
                false_positives += 1;
            }
        }
    }
    
    let accuracy = (true_positives + true_negatives) as f64 / 1000.0 * 100.0;
    let fp_rate = false_positives as f64 / 500.0 * 100.0;
    
    println!("  True positives: {}/500", true_positives);
    println!("  True negatives: {}/500", true_negatives);
    println!("  False positives: {}/500", false_positives);
    println!("  Accuracy: {:.1}%", accuracy);
    println!("  False positive rate: {:.1}%", fp_rate);
    
    // Test 3: Size calculation
    println!("\nTest 3: Optimal size calculation");
    let capacity = 100_000;
    let fp_rate: f64 = 0.01;
    let ln2 = 2.0_f64.ln();
    let optimal_size = (-(capacity as f64) * fp_rate.ln() / (ln2 * ln2)).ceil() as usize;
    let optimal_hashes = ((optimal_size as f64 / capacity as f64) * ln2).ceil() as usize;
    
    println!("  For capacity={}, FP rate={}", capacity, fp_rate);
    println!("  Optimal bit array size: {}", optimal_size);
    println!("  Optimal hash count: {}", optimal_hashes);
    
    // Test 4: Estimated FP rate
    println!("\nTest 4: Estimated false positive rate");
    let bf3 = BloomFilter::new(10_000, 0.01);
    let estimated_fp = bf3.estimated_false_positive_rate();
    println!("  Target FP rate: 0.01");
    println!("  Estimated FP rate: {:.4}", estimated_fp);
}
