use anyhow::Result;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use std::path::Path;

// MiniLM-L6 - Only 22MB model size, 384 dimensions
const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";
const EMBEDDING_DIM: usize = 384;

pub struct LocalEmbedding {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl LocalEmbedding {
    pub async fn new() -> Result<Self> {
        let device = Device::Cpu;
        
        // Download model if not cached
        let api = Api::new()?;
        let repo = api.repo(Repo::new(MODEL_ID.to_string(), RepoType::Model));
        
        let weights_filename = repo.get("pytorch_model.bin").await?;
        let tokenizer_filename = repo.get("tokenizer.json").await?;
        
        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_filename)?;
        
        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_filename], candle_core::DType::F32, &device)?
        };
        
        let config = Config {
            vocab_size: 30522,
            hidden_size: 384,
            num_hidden_layers: 6,
            num_attention_heads: 12,
            intermediate_size: 1536,
            max_position_embeddings: 512,
            ..Default::default()
        };
        
        let model = BertModel::load(vb, &config)?;
        
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Tokenize
        let encoding = self.tokenizer.encode(text, true)?;
        let token_ids = encoding.get_ids();
        let token_ids = Tensor::new(token_ids, &self.device)?;
        
        // Get embeddings
        let embeddings = self.model.forward(&token_ids.unsqueeze(0)?)?;
        
        // Mean pooling
        let mean_embedding = embeddings.mean(1)?;
        let embedding_vec = mean_embedding.squeeze(0)?.to_vec1::<f32>()?;
        
        // Normalize
        let norm = embedding_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized = embedding_vec.iter().map(|x| x / norm).collect();
        
        Ok(normalized)
    }
    
    pub fn memory_usage(&self) -> usize {
        // MiniLM-L6: ~22MB model + ~1MB tokenizer = ~23MB total
        23_000_000
    }
}
