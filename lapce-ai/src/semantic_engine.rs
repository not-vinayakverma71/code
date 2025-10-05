/// Semantic Search Engine with LanceDB
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use lancedb::{Connection, Table};
use lancedb::query::ExecutableQuery;
use arrow_array::{RecordBatch, RecordBatchIterator, StringArray, Float32Array, FixedSizeListArray};
use arrow_schema::{Schema, Field, DataType};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_bedrockruntime::primitives::Blob;
use serde_json::json;
use futures::TryStreamExt;

#[derive(Clone)]
pub struct SemanticEngine {
    lancedb: Arc<Connection>,
    bedrock_client: BedrockClient,
    table: Option<Table>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub score: f32,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub language: Option<String>,
    pub path_pattern: Option<String>,
    pub min_score: Option<f32>,
    pub max_results: Option<usize>,
}

pub type SemanticSearchEngine = SemanticEngine;

impl SemanticEngine {
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        let connection = lancedb::connect(&db_path.to_string_lossy()).execute().await?;
        let bedrock_client = Self::create_bedrock_client().await?;
        
        Ok(Self {
            lancedb: Arc::new(connection),
            bedrock_client,
            table: None,
        })
    }
    
    async fn create_bedrock_client() -> Result<BedrockClient> {
        let config = aws_config::load_from_env().await;
        Ok(BedrockClient::new(&config))
    }
    
    pub async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let request = json!({
            "inputText": text,
            "dimensions": 1536,
            "normalize": true
        });
        
        let response = self.bedrock_client
            .invoke_model()
            .model_id("amazon.titan-embed-text-v1")
            .content_type("application/json")
            .body(Blob::new(request.to_string()))
            .send()
            .await?;
        
        let body = response.body().as_ref();
        let result: serde_json::Value = serde_json::from_slice(body)?;
        
        let embedding: Vec<f32> = result["embedding"]
            .as_array()
            .ok_or_else(|| anyhow!("Invalid embedding response"))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    pub async fn index_file(&mut self, file_path: &str, content: &str) -> Result<()> {
        // Split content into chunks
        let chunks = split_into_chunks(content, 500);
        
        for (i, chunk) in chunks.iter().enumerate() {
            // Get embedding for this chunk
            let embedding = self.get_embedding(chunk).await?;
            
            // Store in LanceDB
            let schema = Arc::new(Schema::new(vec![
                Field::new("file_path", DataType::Utf8, false),
                Field::new("chunk_id", DataType::Int32, false),
                Field::new("content", DataType::Utf8, false),
                Field::new("embedding", DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    1536,
                ), false),
            ]));
            
            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(StringArray::from(vec![file_path])),
                    Arc::new(arrow_array::Int32Array::from(vec![i as i32])),
                    Arc::new(StringArray::from(vec![chunk.as_str()])),
                    Arc::new(FixedSizeListArray::new(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        1536,
                        Arc::new(Float32Array::from(embedding)),
                        None,
                    )),
                ],
            )?;
            
            // Add to table
            if self.table.is_none() {
                // Create an iterator for LanceDB
                let batches = vec![Ok(batch.clone())];
                let reader = RecordBatchIterator::new(batches.into_iter(), schema.clone());
                let table = self.lancedb
                    .create_table("code_index", reader)
                    .execute()
                    .await?;
                self.table = Some(table);
            } else {
                // Add data to existing table using iterator
                let batches = vec![Ok(batch)];
                let reader = RecordBatchIterator::new(batches.into_iter(), schema);
                self.table.as_ref().unwrap()
                    .add(reader)
                    .execute()
                    .await?;
            }
        }
        
        Ok(())
    }
    
    pub async fn search(&self, query: &str, limit: usize, filters: Option<SearchFilters>) -> Result<Vec<SearchResult>> {
        if self.table.is_none() {
            return Ok(vec![]);
        }
        
        let query_embedding = self.get_embedding(query).await?;
        
        // Search in LanceDB using vector similarity
        let results = self.table.as_ref().unwrap()
            .vector_search(query_embedding)?
            .execute()
            .await?
            .try_collect::<Vec<_>>()
            .await?;
        
        let mut search_results = Vec::new();
        for batch in results.iter().take(limit) {
            let file_paths = batch.column_by_name("file_path")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            
            let contents = batch.column_by_name("content")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            
            let scores = batch.column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>());
            
            let file_paths = batch.column_by_name("file_path")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            
            let num_rows = batch.num_rows();
            for i in 0..num_rows {
                let doc_content = contents.value(i).to_string();
                let score = scores.map(|s| s.value(i)).unwrap_or(0.0);
                let file_path = file_paths.value(i).to_string();
                
                let mut metadata = HashMap::new();
                metadata.insert("language".to_string(), "unknown".to_string());
                metadata.insert("path".to_string(), file_path.clone());
                
                search_results.push(SearchResult {
                    file_path,
                    score,
                    content: doc_content,
                    start_line: 0,
                    end_line: 0,
                });
            }
        }
        
        Ok(search_results)
    }
}

fn split_into_chunks(text: &str, max_lines: usize) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut chunks = Vec::new();
    
    for chunk in lines.chunks(max_lines) {
        chunks.push(chunk.join("\n"));
    }
    
    chunks
}
