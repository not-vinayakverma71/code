/// IPC Test Client - Real client that connects to IPC server via shared memory
/// NO MOCKS - Uses actual shared memory and doorbells for cross-process communication

use std::time::Duration;
use std::sync::Arc;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use lapce_ai_rust::ipc::ipc_client::IpcClient;
use lapce_ai_rust::ipc::binary_codec::{MessageType, LspRequestPayload, Message, MessagePayload};
use bytes::Bytes;

/// Test client for IPC communication with LSP gateway
pub struct IpcTestClient {
    connection_id: String,
    shm_prefix: String,
    client: Arc<IpcClient>,
}

impl IpcTestClient {
    /// Connect to an existing IPC server
    pub async fn connect(shm_prefix: &str) -> Result<Self> {
        let connection_id = format!("test-client-{}", std::process::id());
        
        println!("Connecting IPC test client: {}", connection_id);
        println!("SHM prefix: {}", shm_prefix);
        
        // Connect to IPC server using real client
        let client = IpcClient::connect(shm_prefix).await
            .context("Failed to connect to IPC server")?;
        
        Ok(Self {
            connection_id,
            shm_prefix: shm_prefix.to_string(),
            client: Arc::new(client),
        })
    }
    
    /// Send an LSP request and wait for response
    pub async fn send_lsp_request(
        &mut self,
        method: &str,
        params: JsonValue,
    ) -> Result<JsonValue> {
        let request_id = format!("req-{}", rand::random::<u64>());
        
        println!("Sending LSP request: method={}, id={}", method, request_id);
        
        // Create LSP request payload
        let lsp_request = LspRequestPayload {
            id: request_id.clone(),
            method: method.to_string(),
            params_json: serde_json::to_string(&params)
                .context("Failed to serialize params")?,
        };
        
        // Serialize request
        let request_bytes = serde_json::to_vec(&lsp_request)
            .context("Failed to serialize LSP request")?;
        
        // Send request and wait for response with timeout
        let response_bytes = self.client.send_with_timeout(
            MessageType::LspRequest,
            &request_bytes,
            Duration::from_secs(5),
        ).await
            .context("Failed to send LSP request or receive response")?;
        
        // Deserialize response
        let response_json: JsonValue = serde_json::from_slice(&response_bytes)
            .context("Failed to deserialize response")?;
        
        // Extract result from LSP response format
        if let Some(result) = response_json.get("result") {
            Ok(result.clone())
        } else if let Some(error) = response_json.get("error") {
            Err(anyhow!("LSP error: {:?}", error))
        } else {
            Ok(response_json)
        }
    }
    
    /// Send didOpen notification
    pub async fn did_open(
        &mut self,
        uri: &str,
        language_id: &str,
        version: i32,
        text: &str,
    ) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": version,
                "text": text,
            }
        });
        
        self.send_lsp_request("textDocument/didOpen", params).await?;
        
        Ok(())
    }
    
    /// Send didChange notification
    pub async fn did_change(
        &mut self,
        uri: &str,
        version: i32,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "version": version,
            },
            "contentChanges": content_changes,
        });
        
        self.send_lsp_request("textDocument/didChange", params).await?;
        
        Ok(())
    }
    
    /// Send didClose notification
    pub async fn did_close(&mut self, uri: &str) -> Result<()> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            }
        });
        
        self.send_lsp_request("textDocument/didClose", params).await?;
        
        Ok(())
    }
    
    /// Request documentSymbol
    pub async fn document_symbol(&mut self, uri: &str) -> Result<Vec<DocumentSymbol>> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            }
        });
        
        let response = self.send_lsp_request("textDocument/documentSymbol", params).await?;
        
        // Parse response
        let symbols: Vec<DocumentSymbol> = serde_json::from_value(response)
            .context("Failed to parse documentSymbol response")?;
        
        Ok(symbols)
    }
    
    /// Request hover
    pub async fn hover(&mut self, uri: &str, line: u32, character: u32) -> Result<Option<Hover>> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            },
            "position": {
                "line": line,
                "character": character,
            }
        });
        
        let response = self.send_lsp_request("textDocument/hover", params).await?;
        
        if response.is_null() {
            return Ok(None);
        }
        
        let hover: Hover = serde_json::from_value(response)
            .context("Failed to parse hover response")?;
        
        Ok(Some(hover))
    }
    
    /// Request definition
    pub async fn definition(&mut self, uri: &str, line: u32, character: u32) -> Result<Vec<Location>> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            },
            "position": {
                "line": line,
                "character": character,
            }
        });
        
        let response = self.send_lsp_request("textDocument/definition", params).await?;
        
        let locations: Vec<Location> = serde_json::from_value(response)
            .context("Failed to parse definition response")?;
        
        Ok(locations)
    }
    
    /// Request references
    pub async fn references(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Vec<Location>> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            },
            "position": {
                "line": line,
                "character": character,
            },
            "context": {
                "includeDeclaration": include_declaration,
            }
        });
        
        let response = self.send_lsp_request("textDocument/references", params).await?;
        
        let locations: Vec<Location> = serde_json::from_value(response)
            .context("Failed to parse references response")?;
        
        Ok(locations)
    }
    
    /// Request folding ranges
    pub async fn folding_range(&mut self, uri: &str) -> Result<Vec<FoldingRange>> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            }
        });
        
        let response = self.send_lsp_request("textDocument/foldingRange", params).await?;
        
        let ranges: Vec<FoldingRange> = serde_json::from_value(response)
            .context("Failed to parse foldingRange response")?;
        
        Ok(ranges)
    }
    
    /// Request semantic tokens
    pub async fn semantic_tokens_full(&mut self, uri: &str) -> Result<SemanticTokens> {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
            }
        });
        
        let response = self.send_lsp_request("textDocument/semanticTokens/full", params).await?;
        
        let tokens: SemanticTokens = serde_json::from_value(response)
            .context("Failed to parse semanticTokens response")?;
        
        Ok(tokens)
    }
    
    /// Wait for diagnostics notification
    pub async fn wait_for_diagnostics(&mut self, uri: &str, timeout: Duration) -> Result<Vec<Diagnostic>> {
        // TODO: Poll for diagnostics from notification channel
        
        tokio::time::sleep(timeout).await;
        
        Ok(Vec::new())
    }
    
    /// Disconnect from server
    pub async fn disconnect(&mut self) -> Result<()> {
        println!("Disconnecting IPC test client: {}", self.connection_id);
        
        // TODO: Close shared memory connection
        
        Ok(())
    }
}

// LSP type definitions (simplified for testing)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: u32,
    pub range: Range,
    pub selection_range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DocumentSymbol>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    pub contents: MarkupContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoldingRange {
    pub start_line: u32,
    pub end_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokens {
    pub data: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    pub text: String,
}

impl Drop for IpcTestClient {
    fn drop(&mut self) {
        println!("Dropping IPC test client: {}", self.connection_id);
        // Cleanup happens automatically
    }
}
