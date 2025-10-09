// Embeddings module
pub mod aws_config_validator;
pub mod aws_titan_production;
pub mod aws_titan_robust;
pub mod bedrock;
pub mod compression;
pub mod config;
pub mod embedder_interface;
pub mod gemini_embedder;
pub mod openai;
pub mod openai_compatible_embedder;
pub mod openai_embedder;
pub mod optimized_embedder_wrapper;
pub mod sentence_transformers;
pub mod service_factory;
pub mod zstd_compression;

// Legacy types for compatibility
use std::sync::Arc;
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddingDefinition {
    pub name: String,
    pub embedding_name: String,
    pub dimensions: usize,
}

#[async_trait]
pub trait EmbeddingFunction: Send + Sync {
    fn name(&self) -> &str;
    fn source_type(&self) -> &str { "text" }
    fn dest_type(&self) -> &str { "vector" }
    async fn compute_source_embeddings(&self, texts: Vec<String>) -> crate::error::Result<Vec<Vec<f32>>> {
        self.embed(texts).await
    }
    async fn compute_query_embeddings(&self, query: String) -> crate::error::Result<Vec<f32>> {
        let embeddings = self.embed(vec![query]).await?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }
    async fn embed(&self, texts: Vec<String>) -> crate::error::Result<Vec<Vec<f32>>>;
}

pub trait EmbeddingRegistry: Send + Sync + std::fmt::Debug {
    fn get(&self, name: &str) -> Option<Arc<dyn EmbeddingFunction>>;
}

pub struct DefaultEmbeddingRegistry {
    functions: std::collections::HashMap<String, Arc<dyn EmbeddingFunction>>,
}

impl std::fmt::Debug for DefaultEmbeddingRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DefaultEmbeddingRegistry")
            .field("functions", &self.functions.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl EmbeddingRegistry for DefaultEmbeddingRegistry {
    fn get(&self, name: &str) -> Option<Arc<dyn EmbeddingFunction>> {
        self.functions.get(name).cloned()
    }
}

pub trait MemoryRegistry: Send + Sync {
    fn new() -> Self where Self: Sized;
}

#[derive(Debug)]
pub struct DefaultMemoryRegistry;
impl DefaultMemoryRegistry {
    pub fn new() -> Self {
        DefaultMemoryRegistry
    }
}
impl MemoryRegistry for DefaultMemoryRegistry {
    fn new() -> Self where Self: Sized {
        DefaultMemoryRegistry
    }
}
impl EmbeddingRegistry for DefaultMemoryRegistry {
    fn get(&self, _name: &str) -> Option<Arc<dyn EmbeddingFunction>> {
        None
    }
}

// Convenience struct for direct use
pub struct MemoryRegistryImpl;

impl MemoryRegistryImpl {
    pub fn new() -> DefaultMemoryRegistry {
        DefaultMemoryRegistry
    }
}

pub struct WithEmbeddings {
    inner: Box<dyn arrow_array::RecordBatchReader + Send>,
    embeddings: Vec<(EmbeddingDefinition, Arc<dyn EmbeddingFunction>)>,
}

impl WithEmbeddings {
    pub fn new(
        inner: Box<dyn arrow_array::RecordBatchReader + Send>,
        embeddings: Vec<(EmbeddingDefinition, Arc<dyn EmbeddingFunction>)>,
    ) -> Self {
        Self { inner, embeddings }
    }
}

impl arrow_array::RecordBatchReader for WithEmbeddings {
    fn schema(&self) -> arrow_schema::SchemaRef {
        self.inner.schema()
    }
}

impl Iterator for WithEmbeddings {
    type Item = Result<arrow_array::RecordBatch, arrow_schema::ArrowError>;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct MaybeEmbedded;

// Re-exports
pub use config::TitanConfig;
pub use service_factory::{CodeIndexServiceFactory as ServiceFactory, IEmbedder, IVectorStore};
pub use aws_titan_production::AwsTitanProduction;
