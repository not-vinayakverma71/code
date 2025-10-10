// Complete GraphQL Server Implementation
use async_graphql::{Context, EmptySubscription, Object, Schema, SimpleObject};
use async_graphql_axum::GraphQL;
use axum::{
    extract::Extension,
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

#[derive(SimpleObject, Clone, Debug)]
struct Document {
    id: String,
    content: String,
    score: f32,
}

#[derive(Clone)]
struct Storage {
    documents: Arc<RwLock<HashMap<String, Document>>>,
}

struct Query;

#[Object]
impl Query {
    async fn document(&self, ctx: &Context<'_>, id: String) -> Option<Document> {
        let storage = ctx.data::<Storage>().unwrap();
        let docs = storage.documents.read().await;
        docs.get(&id).cloned()
    }
    
    async fn search(&self, ctx: &Context<'_>, query: String, limit: i32) -> Vec<Document> {
        let storage = ctx.data::<Storage>().unwrap();
        let docs = storage.documents.read().await;
        
        let mut results: Vec<Document> = docs.values()
            .filter(|d| d.content.contains(&query))
            .take(limit as usize)
            .cloned()
            .collect();
            
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }
    
    async fn health(&self) -> String {
        "GraphQL API is healthy".to_string()
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn create_document(&self, ctx: &Context<'_>, id: String, content: String) -> Document {
        let storage = ctx.data::<Storage>().unwrap();
        let doc = Document {
            id: id.clone(),
            content,
            score: 0.95,
        };
        
        let mut docs = storage.documents.write().await;
        docs.insert(id, doc.clone());
        doc
    }
    
    async fn delete_document(&self, ctx: &Context<'_>, id: String) -> bool {
        let storage = ctx.data::<Storage>().unwrap();
        let mut docs = storage.documents.write().await;
        docs.remove(&id).is_some()
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 Starting GraphQL Server on port 8081");
    
    let storage = Storage {
        documents: Arc::new(RwLock::new(HashMap::new())),
    };
    
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(storage)
        .finish();
    
    let app = Router::new()
        .route("/graphql", axum::routing::get(graphql_playground).post_service(GraphQL::new(schema.clone())))
        .layer(Extension(schema));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081")
        .await
        .unwrap();
    
    println!("✅ GraphQL API listening on http://0.0.0.0:8081/graphql");
    
    axum::serve(listener, app).await.unwrap();
}

async fn graphql_playground() -> impl axum::response::IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql")
            .subscription_endpoint("/graphql/ws")
    ))
}
