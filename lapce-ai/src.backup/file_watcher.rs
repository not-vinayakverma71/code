// Day 18: Real-time File Watcher & Indexing
use notify::{RecommendedWatcher, RecursiveMode, Event, EventKind, Watcher};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use std::collections::HashMap;
use std::time::Instant;

pub struct FileIndexer {
    index: Arc<RwLock<HashMap<String, FileInfo>>>,
    watcher_tx: mpsc::Sender<IndexCommand>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub content: String,
    pub language: Language,
    pub tokens: Vec<Token>,
    pub last_modified: Instant,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    Go,
    TypeScript,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub enum TokenKind {
    Function,
    Class,
    Variable,
    Import,
    Comment,
    Keyword,
    Literal,
}

#[derive(Debug)]
enum IndexCommand {
    Index(String),
    Remove(String),
    Reindex,
}

impl FileIndexer {
    pub async fn new(watch_paths: Vec<String>) -> Self {
        let (tx, mut rx) = mpsc::channel::<IndexCommand>(1000);
        let index = Arc::new(RwLock::new(HashMap::new()));
        
        // Start indexing worker
        let index_clone = index.clone();
        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    IndexCommand::Index(path) => {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let info = FileInfo {
                                path: path.clone(),
                                content: content.clone(),
                                language: detect_language(&path),
                                tokens: tokenize(&content, &detect_language(&path)),
                                last_modified: Instant::now(),
                                embedding: generate_embedding(&content),
                            };
                            
                            let mut idx = index_clone.write().await;
                            idx.insert(path, info);
                        }
                    }
                    IndexCommand::Remove(path) => {
                        let mut idx = index_clone.write().await;
                        idx.remove(&path);
                    }
                    IndexCommand::Reindex => {
                        // Reindex all files
                        let paths: Vec<String> = {
                            let idx = index_clone.read().await;
                            idx.keys().cloned().collect()
                        };
                        
                        for path in paths {
                            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                                let info = FileInfo {
                                    path: path.clone(),
                                    content: content.clone(),
                                    language: detect_language(&path),
                                    tokens: tokenize(&content, &detect_language(&path)),
                                    last_modified: Instant::now(),
                                    embedding: generate_embedding(&content),
                                };
                                
                                let mut idx = index_clone.write().await;
                                idx.insert(path, info);
                            }
                        }
                    }
                }
            }
        });
        
        // Start file watcher
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let (watcher_tx, mut watcher_rx) = mpsc::channel(100);
            
            let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = watcher_tx.blocking_send(event);
                }
            }).unwrap();
            
            for path in watch_paths {
                watcher.watch(Path::new(&path), RecursiveMode::Recursive).unwrap();
            }
            
            while let Some(event) = watcher_rx.recv().await {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if let Some(path_str) = path.to_str() {
                                if is_code_file(path_str) {
                                    let _ = tx_clone.send(IndexCommand::Index(path_str.to_string())).await;
                                }
                            }
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            if let Some(path_str) = path.to_str() {
                                let _ = tx_clone.send(IndexCommand::Remove(path_str.to_string())).await;
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        
        Self {
            index,
            watcher_tx: tx,
        }
    }
    
    pub async fn search(&self, query: &str) -> Vec<SearchResult> {
        let idx = self.index.read().await;
        let mut results = Vec::new();
        
        for (path, info) in idx.iter() {
            // Simple text search
            if info.content.contains(query) {
                let score = calculate_relevance(&info.content, query);
                results.push(SearchResult {
                    path: path.clone(),
                    snippet: extract_snippet(&info.content, query),
                    score,
                    language: info.language.clone(),
                });
            }
            
            // Token search
            for token in &info.tokens {
                if token.text.contains(query) {
                    results.push(SearchResult {
                        path: path.clone(),
                        snippet: format!("{}:{}: {}", token.line, token.column, token.text),
                        score: 0.8,
                        language: info.language.clone(),
                    });
                    break;
                }
            }
        }
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(50);
        results
    }
    
    pub async fn get_stats(&self) -> IndexStats {
        let idx = self.index.read().await;
        
        let mut stats = IndexStats {
            total_files: idx.len(),
            total_tokens: 0,
            languages: HashMap::new(),
        };
        
        for info in idx.values() {
            stats.total_tokens += info.tokens.len();
            *stats.languages.entry(format!("{:?}", info.language)).or_insert(0) += 1;
        }
        
        stats
    }
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: String,
    pub snippet: String,
    pub score: f32,
    pub language: Language,
}

#[derive(Debug)]
pub struct IndexStats {
    pub total_files: usize,
    pub total_tokens: usize,
    pub languages: HashMap<String, usize>,
}

fn detect_language(path: &str) -> Language {
    if path.ends_with(".rs") {
        Language::Rust
    } else if path.ends_with(".py") {
        Language::Python
    } else if path.ends_with(".js") || path.ends_with(".jsx") {
        Language::JavaScript
    } else if path.ends_with(".ts") || path.ends_with(".tsx") {
        Language::TypeScript
    } else if path.ends_with(".go") {
        Language::Go
    } else {
        Language::Unknown
    }
}

fn is_code_file(path: &str) -> bool {
    path.ends_with(".rs") || path.ends_with(".py") || path.ends_with(".js") || 
    path.ends_with(".ts") || path.ends_with(".go") || path.ends_with(".jsx") ||
    path.ends_with(".tsx")
}

fn tokenize(content: &str, language: &Language) -> Vec<Token> {
    let mut tokens = Vec::new();
    
    for (line_no, line) in content.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        
        for (col, word) in words.iter().enumerate() {
            let kind = match *word {
                "fn" | "def" | "function" | "func" => TokenKind::Function,
                "class" | "struct" | "impl" => TokenKind::Class,
                "let" | "var" | "const" => TokenKind::Variable,
                "use" | "import" | "from" | "require" => TokenKind::Import,
                _ if word.starts_with("//") || word.starts_with("#") => TokenKind::Comment,
                _ => TokenKind::Literal,
            };
            
            tokens.push(Token {
                text: word.to_string(),
                kind,
                line: line_no + 1,
                column: col + 1,
            });
        }
    }
    
    tokens
}

fn generate_embedding(content: &str) -> Vec<f32> {
    // Simple mock embedding
    let mut embedding = vec![0.0; 384];
    for (i, ch) in content.chars().take(384).enumerate() {
        embedding[i] = (ch as u32 as f32) / 128.0;
    }
    embedding
}

fn calculate_relevance(content: &str, query: &str) -> f32 {
    let count = content.matches(query).count();
    (count as f32 / content.len() as f32) * 100.0
}

fn extract_snippet(content: &str, query: &str) -> String {
    if let Some(pos) = content.find(query) {
        let start = pos.saturating_sub(50);
        let end = (pos + query.len() + 50).min(content.len());
        format!("...{}...", &content[start..end])
    } else {
        content.chars().take(100).collect()
    }
}
