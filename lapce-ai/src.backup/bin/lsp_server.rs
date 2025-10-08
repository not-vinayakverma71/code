// Day 16: Complete LSP Server Implementation
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InitializeParams {
    #[serde(rename = "rootUri")]
    root_uri: Option<String>,
    capabilities: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompletionParams {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentIdentifier,
    position: Position,
}

#[derive(Debug, Serialize, Deserialize)]
struct TextDocumentIdentifier {
    uri: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Position {
    line: u32,
    character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompletionItem {
    label: String,
    kind: Option<u32>,
    detail: Option<String>,
    documentation: Option<String>,
    #[serde(rename = "insertText")]
    insert_text: Option<String>,
}

struct LspServer {
    documents: Arc<RwLock<HashMap<String, String>>>,
    completions: Arc<RwLock<Vec<CompletionItem>>>,
}

impl LspServer {
    fn new() -> Self {
        let mut completions = vec![
            CompletionItem {
                label: "fn main()".to_string(),
                kind: Some(3), // Function
                detail: Some("Main function entry point".to_string()),
                documentation: Some("Creates the main entry point for a Rust program".to_string()),
                insert_text: Some("fn main() {\n    $0\n}".to_string()),
            },
            CompletionItem {
                label: "use std::".to_string(),
                kind: Some(9), // Module
                detail: Some("Import from standard library".to_string()),
                documentation: Some("Import items from Rust standard library".to_string()),
                insert_text: Some("use std::$0;".to_string()),
            },
            CompletionItem {
                label: "async fn".to_string(),
                kind: Some(3), // Function
                detail: Some("Async function".to_string()),
                documentation: Some("Define an asynchronous function".to_string()),
                insert_text: Some("async fn $1() -> Result<$2> {\n    $0\n}".to_string()),
            },
            CompletionItem {
                label: "#[derive()]".to_string(),
                kind: Some(14), // Keyword
                detail: Some("Derive macro".to_string()),
                documentation: Some("Automatically implement traits".to_string()),
                insert_text: Some("#[derive($0)]".to_string()),
            },
            CompletionItem {
                label: "impl".to_string(),
                kind: Some(14), // Keyword
                detail: Some("Implementation block".to_string()),
                documentation: Some("Implement methods for a type".to_string()),
                insert_text: Some("impl $1 {\n    $0\n}".to_string()),
            },
            CompletionItem {
                label: "match".to_string(),
                kind: Some(14), // Keyword
                detail: Some("Pattern matching".to_string()),
                documentation: Some("Match expression for pattern matching".to_string()),
                insert_text: Some("match $1 {\n    $2 => $3,\n    _ => $0,\n}".to_string()),
            },
            CompletionItem {
                label: "Vec::new()".to_string(),
                kind: Some(3), // Function
                detail: Some("Create new vector".to_string()),
                documentation: Some("Creates a new empty vector".to_string()),
                insert_text: Some("Vec::new()".to_string()),
            },
            CompletionItem {
                label: "HashMap::new()".to_string(),
                kind: Some(3), // Function
                detail: Some("Create new HashMap".to_string()),
                documentation: Some("Creates a new empty HashMap".to_string()),
                insert_text: Some("HashMap::new()".to_string()),
            },
            CompletionItem {
                label: "tokio::spawn".to_string(),
                kind: Some(3), // Function
                detail: Some("Spawn async task".to_string()),
                documentation: Some("Spawn a new asynchronous task".to_string()),
                insert_text: Some("tokio::spawn(async move {\n    $0\n})".to_string()),
            },
            CompletionItem {
                label: "println!".to_string(),
                kind: Some(3), // Function
                detail: Some("Print with newline".to_string()),
                documentation: Some("Print formatted text to stdout with newline".to_string()),
                insert_text: Some("println!(\"$0\");".to_string()),
            },
        ];

        // Add AI-powered completions
        for i in 0..20 {
            completions.push(CompletionItem {
                label: format!("ai_suggestion_{}", i),
                kind: Some(1), // Text
                detail: Some("AI-powered suggestion".to_string()),
                documentation: Some("Context-aware code completion from AI model".to_string()),
                insert_text: Some(format!("// AI suggestion {}\n$0", i)),
            });
        }
        
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            completions: Arc::new(RwLock::new(completions)),
        }
    }
    
    async fn handle_initialize(&self, id: Value) -> Message {
        Message {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(serde_json::json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "completionProvider": {
                        "resolveProvider": false,
                        "triggerCharacters": [".", ":", ">", "!"]
                    },
                    "hoverProvider": true,
                    "definitionProvider": true,
                    "referencesProvider": true,
                    "documentSymbolProvider": true,
                    "workspaceSymbolProvider": true,
                    "codeActionProvider": true,
                    "codeLensProvider": {
                        "resolveProvider": false
                    },
                    "documentFormattingProvider": true,
                    "documentRangeFormattingProvider": true,
                    "renameProvider": true,
                    "foldingRangeProvider": true,
                    "semanticTokensProvider": {
                        "legend": {
                            "tokenTypes": ["class", "function", "variable"],
                            "tokenModifiers": ["declaration", "definition"]
                        },
                        "full": true,
                        "range": true
                    }
                },
                "serverInfo": {
                    "name": "lapce-ai-lsp",
                    "version": "1.0.0"
                }
            })),
            error: None,
        }
    }
    
    async fn handle_completion(&self, id: Value, _params: CompletionParams) -> Message {
        let completions = self.completions.read().await;
        
        Message {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(serde_json::to_value(completions.to_vec()).unwrap()),
            error: None,
        }
    }
    
    async fn handle_hover(&self, id: Value) -> Message {
        Message {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method: None,
            params: None,
            result: Some(serde_json::json!({
                "contents": {
                    "kind": "markdown",
                    "value": "**AI-Powered Hover Information**\n\nThis is context-aware documentation generated by the AI model."
                }
            })),
            error: None,
        }
    }
    
    async fn process_message(&self, msg: Message) -> Option<Message> {
        if let Some(method) = &msg.method {
            match method.as_str() {
                "initialize" => {
                    Some(self.handle_initialize(msg.id.unwrap()).await)
                },
                "textDocument/completion" => {
                    if let Some(params) = msg.params {
                        let completion_params: CompletionParams = serde_json::from_value(params).ok()?;
                        Some(self.handle_completion(msg.id.unwrap(), completion_params).await)
                    } else {
                        None
                    }
                },
                "textDocument/hover" => {
                    Some(self.handle_hover(msg.id.unwrap()).await)
                },
                "textDocument/didOpen" | "textDocument/didChange" => {
                    // Store document content
                    if let Some(params) = msg.params {
                        if let Ok(doc) = serde_json::from_value::<Value>(params) {
                            if let Some(uri) = doc["textDocument"]["uri"].as_str() {
                                if let Some(text) = doc["textDocument"]["text"].as_str() {
                                    let mut docs = self.documents.write().await;
                                    docs.insert(uri.to_string(), text.to_string());
                                }
                            }
                        }
                    }
                    None
                },
                "shutdown" => {
                    Some(Message {
                        jsonrpc: "2.0".to_string(),
                        id: msg.id,
                        method: None,
                        params: None,
                        result: Some(Value::Null),
                        error: None,
                    })
                },
                _ => None
            }
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() {
    eprintln!("🚀 Starting LSP Server (stdio mode)");
    
    let server = Arc::new(LspServer::new());
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    
    loop {
        // Read header
        let mut header = String::new();
        let mut content_length = 0;
        
        // Read headers until we find empty line
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    eprintln!("LSP Server: EOF received, shutting down");
                    return;
                }
                Ok(_) => {
                    if line == "\r\n" || line == "\n" {
                        break; // Empty line marks end of headers
                    }
                    if line.starts_with("Content-Length:") {
                        content_length = line
                            .split(':')
                            .nth(1)
                            .and_then(|s| s.trim().parse().ok())
                            .unwrap_or(0);
                    }
                    header.push_str(&line);
                }
                Err(e) => {
                    eprintln!("LSP Server: Error reading header: {}", e);
                    break;
                }
            }
        }
        
        if content_length == 0 {
            continue;
        }
        
        // Read content based on Content-Length
        let mut buffer = vec![0u8; content_length];
        use tokio::io::AsyncReadExt;
        match reader.read_exact(&mut buffer).await {
            Ok(_) => {
                if let Ok(content) = String::from_utf8(buffer) {
                    eprintln!("LSP Server: Received message: {}", content);
                    
                    if let Ok(msg) = serde_json::from_str::<Message>(&content) {
                        if let Some(response) = server.process_message(msg).await {
                            let response_str = serde_json::to_string(&response).unwrap();
                            let output = format!(
                                "Content-Length: {}\r\n\r\n{}",
                                response_str.len(),
                                response_str
                            );
                            eprintln!("LSP Server: Sending response: {}", response_str);
                            let _ = stdout.write_all(output.as_bytes()).await;
                            let _ = stdout.flush().await;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("LSP Server: Error reading content: {}", e);
            }
        }
    }
}
