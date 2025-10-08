// Database types and traits
use std::sync::Arc;
use async_trait::async_trait;
use crate::error::Result;
use crate::table::Table;
use arrow_schema::SchemaRef;
use std::collections::HashMap;

/// Database trait for managing tables
#[async_trait]
pub trait Database: Send + Sync + std::fmt::Debug + std::fmt::Display {
    fn as_any(&self) -> &dyn std::any::Any;
    async fn create_table(&self, request: CreateTableRequest) -> Result<Table>;
    async fn open_table(&self, request: OpenTableRequest) -> Result<Table>;
    async fn drop_table(&self, name: &str) -> Result<()>;
    async fn rename_table(&self, old_name: &str, new_name: &str) -> Result<()>;
    async fn drop_all_tables(&self) -> Result<()>;
    async fn table_names(&self, request: TableNamesRequest) -> Result<Vec<String>>;
    async fn list_namespaces(&self, request: ListNamespacesRequest) -> Result<Vec<String>>;
    async fn create_namespace(&self, request: CreateNamespaceRequest) -> Result<()>;
    async fn drop_namespace(&self, request: DropNamespaceRequest) -> Result<()>;
}

/// Options for database configuration
pub trait DatabaseOptions: Send + Sync {
    fn serialize_into_map(&self, map: &mut HashMap<String, String>);
}

/// Base table trait
#[async_trait]
pub trait BaseTable: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> SchemaRef;
}

/// Create table request
#[derive(Debug, Clone)]
pub struct CreateTableRequest {
    pub name: String,
    pub namespace: Vec<String>,
    pub schema: Option<SchemaRef>,
    pub mode: CreateTableMode,
    pub data: Option<CreateTableData>,
    pub write_options: Option<crate::table::WriteOptions>,
}

impl CreateTableRequest {
    pub fn new(name: String, namespace: Vec<String>) -> Self {
        Self {
            name,
            namespace,
            schema: None,
            mode: CreateTableMode::Create,
            data: None,
            write_options: None,
        }
    }
}

/// Create table mode
pub enum CreateTableMode {
    Create,
    Overwrite,
    CreateIfNotExists,
    ExistOk(Box<dyn Fn(OpenTableRequest) -> OpenTableRequest + Send + Sync>),
}

impl Clone for CreateTableMode {
    fn clone(&self) -> Self {
        match self {
            CreateTableMode::Create => CreateTableMode::Create,
            CreateTableMode::Overwrite => CreateTableMode::Overwrite,
            CreateTableMode::CreateIfNotExists => CreateTableMode::CreateIfNotExists,
            CreateTableMode::ExistOk(_) => CreateTableMode::Create, // Can't clone closure, default to Create
        }
    }
}

impl std::fmt::Debug for CreateTableMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateTableMode::Create => write!(f, "Create"),
            CreateTableMode::Overwrite => write!(f, "Overwrite"),
            CreateTableMode::CreateIfNotExists => write!(f, "CreateIfNotExists"),
            CreateTableMode::ExistOk(_) => write!(f, "ExistOk(callback)"),
        }
    }
}

/// Create table data
pub enum CreateTableData {
    Empty,
    Data(Box<dyn arrow_array::RecordBatchReader + Send>),
    StreamingData(Box<dyn futures::Stream<Item = std::result::Result<arrow_array::RecordBatch, arrow_schema::ArrowError>> + Send + Unpin>),
}

impl std::fmt::Debug for CreateTableData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateTableData::Empty => write!(f, "Empty"),
            CreateTableData::Data(_) => write!(f, "Data(RecordBatchReader)"),
            CreateTableData::StreamingData(_) => write!(f, "StreamingData(Stream)"),
        }
    }
}

impl Clone for CreateTableData {
    fn clone(&self) -> Self {
        // Can't clone readers/streams, so we just return Empty
        CreateTableData::Empty
    }
}

impl CreateTableData {
    pub fn arrow_schema(&self) -> arrow_schema::SchemaRef {
        match self {
            CreateTableData::Empty => Arc::new(arrow_schema::Schema::empty()),
            CreateTableData::Data(reader) => reader.schema(),
            CreateTableData::StreamingData(_) => Arc::new(arrow_schema::Schema::empty()),
        }
    }
}

impl lance_datafusion::utils::StreamingWriteSource for CreateTableData {
    fn arrow_schema(&self) -> arrow_schema::SchemaRef {
        self.arrow_schema()
    }
    
    fn into_stream(self) -> std::pin::Pin<Box<dyn datafusion_physical_plan::RecordBatchStream + Send>> {
        // For now, just return a simple implementation
        // The actual implementation would need proper stream conversion
        todo!("StreamingWriteSource implementation")
    }
}

/// Open table request
#[derive(Debug, Clone)]
pub struct OpenTableRequest {
    pub name: String,
    pub namespace: Vec<String>,
    pub index_cache_size: Option<u64>,
    pub lance_read_params: Option<lance::dataset::ReadParams>,
}

/// Table names request
#[derive(Debug, Clone, Default)]
pub struct TableNamesRequest {
    pub namespace: Option<String>,
    pub start_after: Option<String>,
    pub limit: Option<u32>,
}

/// List namespaces request
#[derive(Debug, Clone, Default)]
pub struct ListNamespacesRequest {
    pub start_after: Option<String>,
    pub limit: Option<u32>,
}

/// Create namespace request
#[derive(Debug, Clone)]
pub struct CreateNamespaceRequest {
    pub name: String,
}

/// Drop namespace request  
#[derive(Debug, Clone)]
pub struct DropNamespaceRequest {
    pub name: String,
}
