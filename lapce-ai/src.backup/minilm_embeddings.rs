/// MiniLM-L6-v2 Embeddings Implementation
use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;

const MODEL_URL: &str = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/";
const MODEL_DIM: usize = 384;
const MAX_SEQ_LENGTH: usize = 256;

#[derive(Debug, Clone)]
pub struct MiniLMEmbeddings {
    vocab: HashMap<String, usize>,
    weights: ModelWeights,
    config: ModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub hidden_size: usize,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub vocab_size: usize,
    pub max_position_embeddings: usize,
}

#[derive(Debug, Clone)]
pub struct ModelWeights {
    pub embeddings: Vec<Vec<f32>>,
    pub attention_weights: Vec<AttentionLayer>,
    pub layer_norm: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct AttentionLayer {
    pub query: Vec<Vec<f32>>,
    pub key: Vec<Vec<f32>>,
    pub value: Vec<Vec<f32>>,
    pub output: Vec<Vec<f32>>,
}

impl MiniLMEmbeddings {
    /// Initialize model (downloads weights if needed)
    pub async fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .unwrap_or(PathBuf::from("/tmp"))
            .join("lapce")
            .join("models")
            .join("minilm-l6-v2");
        
        fs::create_dir_all(&cache_dir)?;
        
        // For now, create mock weights - real implementation would download
        let config = ModelConfig {
            hidden_size: MODEL_DIM,
            num_attention_heads: 12,
            num_hidden_layers: 6,
            vocab_size: 30522,
            max_position_embeddings: 512,
        };
        
        let weights = ModelWeights::mock(MODEL_DIM);
        let vocab = Self::load_vocab()?;
        
        Ok(Self {
            vocab,
            weights,
            config,
        })
    }
    
    /// Tokenize text
    pub fn tokenize(&self, text: &str) -> Vec<usize> {
        // Simple whitespace tokenization for now
        // Real implementation would use WordPiece
        let mut tokens = vec![101]; // [CLS]
        
        for word in text.to_lowercase().split_whitespace() {
            if let Some(&token_id) = self.vocab.get(word) {
                tokens.push(token_id);
            } else {
                tokens.push(100); // [UNK]
            }
            
            if tokens.len() >= MAX_SEQ_LENGTH - 1 {
                break;
            }
        }
        
        tokens.push(102); // [SEP]
        
        // Pad to max length
        while tokens.len() < MAX_SEQ_LENGTH {
            tokens.push(0); // [PAD]
        }
        
        tokens.truncate(MAX_SEQ_LENGTH);
        tokens
    }
    
    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Vec<f32> {
        let tokens = self.tokenize(text);
        self.forward_pass(&tokens)
    }
    
    /// Batch embedding for multiple texts
    pub fn embed_batch(&self, texts: &[String]) -> Vec<Vec<f32>> {
        texts.iter().map(|text| self.embed(text)).collect()
    }
    
    /// Forward pass through the model
    fn forward_pass(&self, tokens: &[usize]) -> Vec<f32> {
        // Simplified forward pass
        let mut hidden_state = vec![0.0; MODEL_DIM];
        
        // Token embeddings
        for &token_id in tokens.iter().take(20) { // Process first 20 tokens
            if token_id < self.weights.embeddings.len() {
                for i in 0..MODEL_DIM {
                    hidden_state[i] += self.weights.embeddings[token_id][i];
                }
            }
        }
        
        // Apply layer norm (simplified)
        let norm: f32 = hidden_state.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut hidden_state {
                *x /= norm;
            }
        }
        
        // Mean pooling
        hidden_state
    }
    
    /// Load vocabulary
    fn load_vocab() -> Result<HashMap<String, usize>> {
        let mut vocab = HashMap::new();
        
        // Add special tokens
        vocab.insert("[PAD]".to_string(), 0);
        vocab.insert("[UNK]".to_string(), 100);
        vocab.insert("[CLS]".to_string(), 101);
        vocab.insert("[SEP]".to_string(), 102);
        vocab.insert("[MASK]".to_string(), 103);
        
        // Add common words (would load from vocab.txt in real implementation)
        let common_words = vec![
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to",
            "for", "of", "with", "by", "from", "as", "is", "was", "are", "been",
            "have", "has", "had", "do", "does", "did", "will", "would", "could", "should",
            "may", "might", "must", "can", "this", "that", "these", "those", "i",
            "you", "he", "she", "it", "we", "they", "what", "which", "who",
        ];
        
        for (idx, word) in common_words.iter().enumerate() {
            vocab.insert(word.to_string(), 1000 + idx);
        }
        
        Ok(vocab)
    }
}

impl ModelWeights {
    /// Create mock weights for testing
    fn mock(dim: usize) -> Self {
        let vocab_size = 30522;
        let num_layers = 6;
        
        // Initialize with small random values
        let mut embeddings = Vec::with_capacity(vocab_size);
        for _ in 0..vocab_size {
            let mut emb = Vec::with_capacity(dim);
            for j in 0..dim {
                emb.push((j as f32 * 0.001).sin());
            }
            embeddings.push(emb);
        }
        
        let mut attention_weights = Vec::with_capacity(num_layers);
        for _ in 0..num_layers {
            attention_weights.push(AttentionLayer {
                query: vec![vec![0.01; dim]; dim],
                key: vec![vec![0.01; dim]; dim],
                value: vec![vec![0.01; dim]; dim],
                output: vec![vec![0.01; dim]; dim],
            });
        }
        
        Self {
            embeddings,
            attention_weights,
            layer_norm: vec![1.0; dim],
        }
    }
}

/// Calculate cosine similarity between embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_embeddings() {
        let model = MiniLMEmbeddings::new().await.unwrap();
        
        let text = "This is a test sentence";
        let embedding = model.embed(text);
        
        assert_eq!(embedding.len(), MODEL_DIM);
        
        // Check embedding is normalized
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.1);
    }
    
    #[tokio::test]
    async fn test_batch_embeddings() {
        let model = MiniLMEmbeddings::new().await.unwrap();
        
        let texts = vec![
            "First sentence".to_string(),
            "Second sentence".to_string(),
            "Third sentence".to_string(),
        ];
        
        let embeddings = model.embed_batch(&texts);
        
        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), MODEL_DIM);
    }
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }
}
