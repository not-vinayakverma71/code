// Real LanceDB Vector Store implementation
use crate::error::{Error, Result};
use crate::embeddings::service_factory::{IVectorStore, PointStruct};
use crate::query::codebase_search::{VectorStoreSearchResult, SearchPayload};
use crate::connection::Connection;
use crate::table::Table;
use crate::query::{QueryBase, ExecutableQuery};
use arrow_array::{Float32Array, StringArray, ArrayRef, FixedSizeListArray, Int64Array, RecordBatchIterator};
use arrow_array::types::Float32Type;
use arrow_schema::{DataType, Field, Schema};
use arrow_array::RecordBatch;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::Mutex;

pub struct LanceVectorStore {
    db_path: PathBuf,
    vector_dimension: usize,
    connection: Arc<Mutex<Option<Connection>>>,
    table_name: String,
}

impl LanceVectorStore {
    pub fn new(workspace_path: PathBuf, vector_dimension: usize) -> Self {
        let db_path = workspace_path.join(".lance_index");
        Self {
            db_path,
            vector_dimension,
            connection: Arc::new(Mutex::new(None)),
            table_name: "code_embeddings".to_string(),
        }
    }
    
    async fn ensure_initialized(&self) -> Result<Connection> {
        let mut conn_guard = self.connection.lock().await;
        if conn_guard.is_none() {
            // Create connection to LanceDB
            let conn = crate::connect(&self.db_path.to_string_lossy())
                .execute()
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to connect to LanceDB: {}", e)
                })?;
            *conn_guard = Some(conn);
        }
        Ok(conn_guard.as_ref().unwrap().clone())
    }
    
    fn create_schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    self.vector_dimension as i32
                ),
                false
            ),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("code_chunk", DataType::Utf8, false),
            Field::new("start_line", DataType::Int64, false),
            Field::new("end_line", DataType::Int64, false),
            Field::new("segment_hash", DataType::Utf8, false),
        ]))
    }
}

#[async_trait]
impl IVectorStore for LanceVectorStore {
    async fn initialize(&self) -> Result<bool> {
        // Create directory if needed
        std::fs::create_dir_all(&self.db_path).map_err(|e| Error::Runtime {
            message: format!("Failed to create Lance DB directory: {}", e)
        })?;
        
        let conn = self.ensure_initialized().await?;
        
        // Check if table exists
        let tables = conn.table_names()
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to list tables: {}", e)
            })?;
        
        if !tables.contains(&self.table_name) {
            // Create empty table with schema
            let schema = self.create_schema();
            let empty_batch = RecordBatch::new_empty(schema.clone());
            
            // Use RecordBatchIterator to properly implement RecordBatchReader
            let batch_reader = RecordBatchIterator::new(
                vec![Ok(empty_batch)],
                schema.clone()
            );
            
            conn.create_table(&self.table_name, Box::new(batch_reader))
                .execute()
                .await
                .map_err(|e| Error::Runtime {
                    message: format!("Failed to create table: {}", e)
                })?;
        }
        
        Ok(true)
    }
    
    async fn upsert_points(&self, points: Vec<PointStruct>) -> Result<()> {
        if points.is_empty() {
            return Ok(());
        }
        
        let conn = self.ensure_initialized().await?;
        
        // Convert points to Arrow arrays
        let mut ids = Vec::new();
        let mut vectors_data: Vec<Vec<f32>> = Vec::new();
        let mut file_paths = Vec::new();
        let mut code_chunks = Vec::new();
        let mut start_lines = Vec::new();
        let mut end_lines = Vec::new();
        let mut segment_hashes = Vec::new();
        
        for point in &points {
            ids.push(point.id.clone());
            
            // Extract from payload
            file_paths.push(point.payload.get("filePath")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string());
            code_chunks.push(point.payload.get("codeChunk")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string());
            start_lines.push(point.payload.get("startLine")
                .and_then(|v| v.as_i64())
                .unwrap_or(0));
            end_lines.push(point.payload.get("endLine")
                .and_then(|v| v.as_i64())
                .unwrap_or(0));
            segment_hashes.push(point.payload.get("segmentHash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string());
        }
        
        // Create vector array as FixedSizeListArray
        let flat_vectors: Vec<f32> = points.iter()
            .flat_map(|p| p.vector.clone())
            .collect();
        let vector_values = Float32Array::from(flat_vectors);
        let vectors_list = FixedSizeListArray::new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            self.vector_dimension as i32,
            Arc::new(vector_values),
            None,
        );
        
        // Create RecordBatch
        let batch = RecordBatch::try_new(
            self.create_schema(),
            vec![
                Arc::new(StringArray::from(ids)) as ArrayRef,
                Arc::new(vectors_list) as ArrayRef,
                Arc::new(StringArray::from(file_paths)) as ArrayRef,
                Arc::new(StringArray::from(code_chunks)) as ArrayRef,
                Arc::new(Int64Array::from(start_lines)) as ArrayRef,
                Arc::new(Int64Array::from(end_lines)) as ArrayRef,
                Arc::new(StringArray::from(segment_hashes)) as ArrayRef,
            ]
        ).map_err(|e| Error::Runtime {
            message: format!("Failed to create record batch: {}", e)
        })?;
        
        // Append to table
        let table = conn.open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to open table: {}", e)
            })?;
        
        // Use RecordBatchIterator for proper RecordBatchReader implementation
        let batch_reader = RecordBatchIterator::new(
            vec![Ok(batch.clone())],
            batch.schema()
        );
        
        table.add(Box::new(batch_reader))
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to add batch to table: {}", e)
            })?;
        
        Ok(())
    }
    
    async fn delete_points_by_file_path(&self, path: &str) -> Result<()> {
        let conn = self.ensure_initialized().await?;
        let table = conn.open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to open table: {}", e)
            })?;
        
        // Delete rows where file_path matches
        table.delete(&format!("file_path = '{}'", path))
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to delete points: {}", e)
            })?;
        
        Ok(())
    }
    
    async fn delete_points_by_multiple_file_paths(&self, paths: &[String]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }
        
        let conn = self.ensure_initialized().await?;
        let table = conn.open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to open table: {}", e)
            })?;
        
        // Build IN clause for batch deletion
        let paths_list = paths.iter()
            .map(|p| format!("'{}'", p))
            .collect::<Vec<_>>()
            .join(", ");
        
        table.delete(&format!("file_path IN ({})", paths_list))
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to delete points: {}", e)
            })?;
        
        Ok(())
    }
    
    async fn search(
        &self,
        vector: Vec<f32>,
        directory_prefix: Option<&str>,
        _min_score: Option<f32>,
        max_results: Option<usize>,
    ) -> Result<Vec<VectorStoreSearchResult>> {
        let conn = self.ensure_initialized().await?;
        let table = conn.open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to open table: {}", e)
            })?;
        
        let limit = max_results.unwrap_or(10);
        
        // Build query with vector search
        let mut query = table.vector_search(vector)
            .map_err(|e| Error::Runtime {
                message: format!("Failed to create vector query: {}", e)
            })?;
        
        query = query.limit(limit);
        
        // Add directory filter if specified
        if let Some(prefix) = directory_prefix {
            query = query.only_if(&format!("file_path LIKE '{}%'", prefix));
        }
        
        // Execute query
        let results = query
            .execute()
            .await
            .map_err(|e| Error::Runtime {
                message: format!("Failed to execute search: {}", e)
            })?;
        
        // Convert to VectorStoreSearchResult
        let mut search_results = Vec::new();
        use futures::TryStreamExt;
        let mut stream = Box::pin(results);
        
        while let Some(batch) = stream.try_next().await.map_err(|e| Error::Runtime {
            message: format!("Failed to read search results: {}", e)
        })? {
            // Extract fields from batch
            for i in 0..batch.num_rows() {
                // Extract string columns
                let id_col = batch.column_by_name("id")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|arr| arr.value(i).to_string())
                    .unwrap_or_default();
                
                let file_path = batch.column_by_name("file_path")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|arr| arr.value(i).to_string())
                    .unwrap_or_default();
                
                let code_chunk = batch.column_by_name("code_chunk")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|arr| arr.value(i).to_string())
                    .unwrap_or_default();
                
                let start_line = batch.column_by_name("start_line")
                    .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
                    .map(|arr| arr.value(i) as u32)
                    .unwrap_or(0);
                
                let end_line = batch.column_by_name("end_line")
                    .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
                    .map(|arr| arr.value(i) as u32)
                    .unwrap_or(0);
                
                // Extract distance score from _distance column
                let score = batch.column_by_name("_distance")
                    .and_then(|c| c.as_any().downcast_ref::<arrow_array::Float32Array>())
                    .map(|arr| {
                        let distance = arr.value(i);
                        // Convert distance to similarity score (cosine distance range is [0, 2])
                        // Lower distance = higher similarity
                        1.0 - (distance / 2.0).min(1.0).max(0.0)
                    })
                    .unwrap_or(0.5);
                
                search_results.push(VectorStoreSearchResult {
                    id: id_col,
                    score,
                    payload: Some(SearchPayload {
                        file_path,
                        code_chunk,
                        start_line,
                        end_line,
                    }),
                });
            }
        }
        
        Ok(search_results)
    }
}
