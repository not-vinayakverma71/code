/// Tantivy Full-Text Search - Day 33 PM
use tantivy::{schema::*, Index, IndexWriter};
use tantivy::query::QueryParser;
use tantivy::collector::TopDocs;
use anyhow::Result;
use std::path::Path;

pub struct TantivySearchEngine {
    index: Index,
    schema: Schema,
    writer: IndexWriter,
}

impl TantivySearchEngine {
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("content", TEXT | STORED);
        schema_builder.add_text_field("tags", TEXT | STORED | FAST);
        schema_builder.add_f64_field("score", STORED | FAST);
        schema_builder.add_date_field("created_at", STORED | FAST);
        
        let schema = schema_builder.build();
        
        // Check if it's a valid index directory by looking for meta.json
        let meta_path = index_path.as_ref().join("meta.json");
        let index = if meta_path.exists() {
            Index::open_in_dir(index_path)?
        } else {
            std::fs::create_dir_all(&index_path)?;
            Index::create_in_dir(index_path, schema.clone())?
        };
        
        let writer = index.writer(50_000_000)?;
        
        Ok(Self {
            index,
            schema,
            writer,
        })
    }
    
    pub fn add_document(&mut self, id: &str, title: &str, content: &str, tags: Vec<&str>) -> Result<()> {
        let id_field = self.schema.get_field("id")?;
        let title_field = self.schema.get_field("title")?;
        let content_field = self.schema.get_field("content")?;
        let tags_field = self.schema.get_field("tags")?;
        let score_field = self.schema.get_field("score")?;
        
        let mut doc = tantivy::TantivyDocument::new();
        doc.add_text(id_field, id);
        doc.add_text(title_field, title);
        doc.add_text(content_field, content);
        doc.add_text(tags_field, &tags.join(" "));
        doc.add_f64(score_field, 1.0);
        
        self.writer.add_document(doc)?;
        Ok(())
    }
    
    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit()?;
        Ok(())
    }
    
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        
        let content_field = self.schema.get_field("content")?;
        let title_field = self.schema.get_field("title")?;
        
        let query_parser = QueryParser::for_index(&self.index, vec![title_field, content_field]);
        let query = query_parser.parse_query(query_str)?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            
            let id = retrieved_doc.get_first(self.schema.get_field("id")?)
                .and_then(|v| match v {
                    tantivy::schema::OwnedValue::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("")
                .to_string();
            
            let title = retrieved_doc.get_first(self.schema.get_field("title")?)
                .and_then(|v| match v {
                    tantivy::schema::OwnedValue::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("")
                .to_string();
            
            let content = retrieved_doc.get_first(self.schema.get_field("content")?)
                .and_then(|v| match v {
                    tantivy::schema::OwnedValue::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("")
                .to_string();
                
            results.push(SearchResult {
                id,
                title,
                content,
                score: _score,
            });
        }
        
        Ok(results)
    }
    
    pub fn delete_document(&mut self, id: &str) -> Result<()> {
        let id_field = self.schema.get_field("id")?;
        let term = tantivy::Term::from_field_text(id_field, id);
        self.writer.delete_term(term);
        Ok(())
    }
}

pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_tantivy_search() {
        let temp_dir = TempDir::new().unwrap();
        let mut engine = TantivySearchEngine::new(temp_dir.path()).unwrap();
        
        // Add documents
        engine.add_document("1", "Rust Programming", "Rust is a systems programming language", vec!["rust", "programming"]).unwrap();
        engine.add_document("2", "Python Guide", "Python is great for data science", vec!["python", "data"]).unwrap();
        engine.add_document("3", "Rust Safety", "Memory safety without garbage collection", vec!["rust", "memory"]).unwrap();
        engine.commit().unwrap();
        
        // Search
        let results = engine.search("rust memory", 10).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].content.contains("safety"));
        
        // Delete
        engine.delete_document("2").unwrap();
        engine.commit().unwrap();
        
        let results = engine.search("python", 10).unwrap();
        assert!(results.is_empty());
    }
}
