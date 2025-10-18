/// Native LSP Gateway Implementation
/// 
/// Provides tree-sitter based LSP features without external language servers

mod document_sync;
mod symbols;
mod hover;
mod definition;
mod references;
mod folding;
mod semantic_tokens;
mod diagnostics;
mod index;
mod file_watcher;
mod metrics;
mod security;
mod observability;
mod cancellation;
mod memory;
mod backpressure;
mod streaming;
mod concurrency;
mod spec_compliance;
mod recovery;
mod plugin_detection;

pub use document_sync::DocumentSync;
pub use symbols::SymbolExtractor;
pub use hover::HoverProvider;
pub use definition::DefinitionProvider;
pub use references::ReferencesProvider;
pub use folding::FoldingProvider;
pub use semantic_tokens::SemanticTokensProvider;
pub use diagnostics::DiagnosticsProvider;
pub use index::SymbolIndex;
pub use file_watcher::FileSystemWatcher;
pub use metrics::{LspMetrics, RequestTimer, ParseTimer};
pub use security::{SecurityConfig, SecurityValidator, RateLimiter, redact_pii};
pub use observability::{CorrelationId, ErrorCode, LspError, RequestContext, ObservabilityConfig, init_tracing};
pub use cancellation::{CancellationToken, CancellationRegistry, CancellationError, TimeoutConfig};
pub use memory::{MemoryManager, MemoryConfig, MemoryUsage};
pub use backpressure::{CircuitBreaker, CircuitBreakerConfig, CircuitState, RequestQueue, QueueConfig, QueuedRequest, RequestPriority};
pub use streaming::{ProgressToken, ProgressNotification, ProgressKind, ProgressReporter, DiagnosticChunk, DiagnosticsChunker};
pub use concurrency::{ConcurrentDocumentStore, DocumentData, ParseTreeCache, CachedTree, LspParserPool, TaskQueue, ConcurrentSymbolIndex, SymbolLocation};
pub use spec_compliance::{UriPathConverter, PositionValidator, RangeValidator, LocationValidator, OptionalFieldHelper};
pub use recovery::{RecoveryManager, DocumentSnapshot, DiagnosticsSnapshot, GatewaySnapshot, IpcReconnectionHandler, DocumentRehydrationManager};
pub use plugin_detection::{PluginConflictDetector, LspSource, LspRegistration, LanguageConflict, ConflictReport};

use std::sync::Arc;
use anyhow::Result;
use bytes::Bytes;

use crate::ipc::binary_codec::{
    BinaryCodec, Message, MessageType, MessagePayload,
    LspRequestPayload, LspResponsePayload, LspNotificationPayload,
    LspDiagnosticsPayload, LspProgressPayload,
};
use crate::ipc::errors::IpcResult;

/// Main LSP gateway coordinator
pub struct LspGateway {
    // Codec is wrapped in Mutex for interior mutability
    codec: std::sync::Mutex<BinaryCodec>,
    // Document synchronization (shared across providers)
    doc_sync: Arc<parking_lot::Mutex<DocumentSync>>,
    // Symbol extraction
    symbol_extractor: Arc<SymbolExtractor>,
    // Hover provider
    hover_provider: Arc<HoverProvider>,
    // Symbol index for definition/references
    symbol_index: Arc<parking_lot::Mutex<SymbolIndex>>,
    // Definition provider
    definition_provider: Arc<DefinitionProvider>,
    // References provider
    references_provider: Arc<ReferencesProvider>,
    // Folding provider
    folding_provider: Arc<FoldingProvider>,
    // Semantic tokens provider
    semantic_tokens_provider: Arc<SemanticTokensProvider>,
    // Diagnostics provider (needs mutability for caching)
    diagnostics_provider: Arc<parking_lot::Mutex<DiagnosticsProvider>>,
}

impl LspGateway {
    /// Create new LSP gateway
    pub fn new() -> Self {
        let doc_sync = Arc::new(parking_lot::Mutex::new(DocumentSync::new()));
        let symbol_index = Arc::new(parking_lot::Mutex::new(SymbolIndex::new()));
        
        Self {
            codec: std::sync::Mutex::new(BinaryCodec::new()),
            doc_sync: doc_sync.clone(),
            symbol_extractor: Arc::new(SymbolExtractor::new(doc_sync.clone())),
            hover_provider: Arc::new(HoverProvider::new(doc_sync.clone())),
            definition_provider: Arc::new(DefinitionProvider::new(doc_sync.clone(), symbol_index.clone())),
            references_provider: Arc::new(ReferencesProvider::new(doc_sync.clone(), symbol_index.clone())),
            folding_provider: Arc::new(FoldingProvider::new(doc_sync.clone())),
            semantic_tokens_provider: Arc::new(SemanticTokensProvider::new(doc_sync.clone())),
            diagnostics_provider: Arc::new(parking_lot::Mutex::new(DiagnosticsProvider::new(doc_sync.clone()))),
            symbol_index,
        }
    }

    /// Handle LSP request
    pub async fn handle_request(&self, request_data: Bytes) -> IpcResult<Bytes> {
        // Decode request
        let msg = self.codec.lock().unwrap().decode(&request_data)?;
        
        let response_payload = match &msg.payload {
            MessagePayload::LspRequest(req) => {
                // Route to appropriate handler based on method
                self.route_lsp_request(req).await?
            }
            _ => {
                return Err(crate::ipc::errors::IpcError::invalid_message(
                    "Expected LspRequest payload"
                ));
            }
        };

        // Encode response
        let response = Message {
            id: msg.id,
            msg_type: MessageType::LspResponse,
            payload: response_payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let encoded = self.codec.lock().unwrap().encode(&response)?;
        Ok(Bytes::from(encoded.to_vec()))
    }

    /// Route LSP request to specific handler
    async fn route_lsp_request(
        &self,
        req: &LspRequestPayload,
    ) -> Result<MessagePayload, crate::ipc::errors::IpcError> {
        // Stub implementation - will be filled in LSP-006 through LSP-015
        let result_json = match req.method.as_str() {
            "textDocument/documentSymbol" => {
                // LSP-008: Document symbols
                self.handle_document_symbol(req).await?
            }
            "textDocument/hover" => {
                // LSP-009: Hover information
                self.handle_hover(req).await?
            }
            "textDocument/definition" => {
                // LSP-010: Go to definition
                self.handle_definition(req).await?
            }
            "textDocument/references" => {
                // LSP-011: Find references
                self.handle_references(req).await?
            }
            "textDocument/foldingRange" => {
                // LSP-012: Folding ranges
                self.handle_folding_range(req).await?
            }
            "textDocument/semanticTokens/full" => {
                // LSP-013: Semantic tokens
                self.handle_semantic_tokens(req).await?
            }
            "workspace/symbol" => {
                // LSP-015: Workspace symbols
                self.handle_workspace_symbol(req).await?
            }
            _ => {
                // Method not supported yet
                return Ok(MessagePayload::LspResponse(LspResponsePayload {
                    id: req.id.clone(),
                    ok: false,
                    result_json: String::new(),
                    error: Some(format!("Method not implemented: {}", req.method)),
                    error_code: Some(-32601), // LSP MethodNotFound
                }));
            }
        };

        Ok(MessagePayload::LspResponse(LspResponsePayload {
            id: req.id.clone(),
            ok: true,
            result_json,
            error: None,
            error_code: None,
        }))
    }

    /// Handle textDocument/documentSymbol
    async fn handle_document_symbol(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        // Extract URI from params
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing URI"))?;
        
        // Get language_id
        let language_id = req.language_id.as_str();
        
        self.symbol_extractor.extract_symbols(uri, language_id).await
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("Symbol extraction failed: {}", e)))
    }

    /// Handle textDocument/hover
    async fn handle_hover(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        // Extract position from params
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing URI"))?;
        
        let line = params["position"]["line"].as_u64()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing line"))? as u32;
        
        let character = params["position"]["character"].as_u64()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing character"))? as u32;
        
        self.hover_provider.get_hover(uri, line, character).await
            .map(|opt| opt.unwrap_or_else(|| "null".to_string()))
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("Hover failed: {}", e)))
    }

    /// Handle textDocument/definition
    async fn handle_definition(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing URI"))?;
        
        let line = params["position"]["line"].as_u64()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing line"))? as u32;
        
        let character = params["position"]["character"].as_u64()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing character"))? as u32;
        
        self.definition_provider.find_definition(uri, line, character).await
            .map(|opt| opt.unwrap_or_else(|| "null".to_string()))
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("Definition lookup failed: {}", e)))
    }

    /// Handle textDocument/references
    async fn handle_references(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing URI"))?;
        
        let line = params["position"]["line"].as_u64()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing line"))? as u32;
        
        let character = params["position"]["character"].as_u64()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing character"))? as u32;
        
        let include_declaration = params["context"]["includeDeclaration"]
            .as_bool()
            .unwrap_or(true);
        
        self.references_provider.find_references(uri, line, character, include_declaration).await
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("References lookup failed: {}", e)))
    }

    /// Handle textDocument/foldingRange
    async fn handle_folding_range(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing URI"))?;
        
        self.folding_provider.get_folding_ranges(uri).await
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("Folding ranges failed: {}", e)))
    }

    /// Handle textDocument/semanticTokens/full
    async fn handle_semantic_tokens(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let uri = params["textDocument"]["uri"].as_str()
            .ok_or_else(|| crate::ipc::errors::IpcError::invalid_message("Missing URI"))?;
        
        let language_id = req.language_id.as_str();
        
        self.semantic_tokens_provider.compute_semantic_tokens(uri, language_id).await
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("Semantic tokens failed: {}", e)))
    }

    /// Handle workspace/symbol
    async fn handle_workspace_symbol(&self, req: &LspRequestPayload) -> Result<String, crate::ipc::errors::IpcError> {
        let params: serde_json::Value = serde_json::from_str(&req.params_json)
            .map_err(|e| crate::ipc::errors::IpcError::invalid_message(format!("Invalid params: {}", e)))?;
        
        let query = params["query"].as_str().unwrap_or("");
        let limit = 100; // Default limit
        
        let index = self.symbol_index.lock();
        let results = index.search_symbols(query, limit);
        
        // Convert to LSP SymbolInformation format
        let symbols: Vec<WorkspaceSymbol> = results.into_iter().map(|(name, location)| {
            WorkspaceSymbol {
                name,
                kind: 1, // Generic symbol kind
                location: WorkspaceSymbolLocation {
                    uri: location.uri,
                    range: WorkspaceSymbolRange {
                        start: WorkspaceSymbolPosition {
                            line: location.line,
                            character: location.character,
                        },
                        end: WorkspaceSymbolPosition {
                            line: location.end_line,
                            character: location.end_character,
                        },
                    },
                },
            }
        }).collect();
        
        serde_json::to_string(&symbols)
            .map_err(|e| crate::ipc::errors::IpcError::internal(format!("Failed to serialize symbols: {}", e)))
    }

    /// Handle LSP notification (didOpen, didChange, didClose)
    pub async fn handle_notification(&self, notification_data: Bytes) -> IpcResult<()> {
        let msg = self.codec.lock().unwrap().decode(&notification_data)?;
        
        if let MessagePayload::LspNotification(notif) = &msg.payload {
            let mut doc_sync = self.doc_sync.lock();
            
            match notif.method.as_str() {
                "textDocument/didOpen" => {
                    // Parse params to extract text and language
                    if let Ok(params) = serde_json::from_str::<serde_json::Value>(&notif.params_json) {
                        if let (Some(text), Some(lang_id)) = (
                            params["textDocument"]["text"].as_str(),
                            params["textDocument"]["languageId"].as_str(),
                        ) {
                            if let Err(e) = doc_sync.did_open(&notif.uri, lang_id, text).await {
                                tracing::error!("Failed to open document {}: {}", notif.uri, e);
                            } else {
                                tracing::info!("Document opened: {}", notif.uri);
                            }
                        }
                    }
                }
                "textDocument/didChange" => {
                    // Parse changes from params
                    if let Ok(params) = serde_json::from_str::<serde_json::Value>(&notif.params_json) {
                        if let Some(changes) = params["contentChanges"].as_array() {
                            let changes_json = serde_json::to_string(changes).unwrap_or_default();
                            if let Err(e) = doc_sync.did_change(&notif.uri, &changes_json).await {
                                tracing::error!("Failed to apply changes to {}: {}", notif.uri, e);
                            }
                        }
                    }
                }
                "textDocument/didClose" => {
                    if let Err(e) = doc_sync.did_close(&notif.uri).await {
                        tracing::error!("Failed to close document {}: {}", notif.uri, e);
                    } else {
                        tracing::info!("Document closed: {}", notif.uri);
                    }
                }
                _ => {
                    tracing::warn!("Unknown LSP notification: {}", notif.method);
                }
            }
        }

        Ok(())
    }
}

impl Default for LspGateway {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Workspace Symbol Types
// ============================================================================

#[derive(Debug, serde::Serialize)]
struct WorkspaceSymbol {
    name: String,
    kind: u32,
    location: WorkspaceSymbolLocation,
}

#[derive(Debug, serde::Serialize)]
struct WorkspaceSymbolLocation {
    uri: String,
    range: WorkspaceSymbolRange,
}

#[derive(Debug, serde::Serialize)]
struct WorkspaceSymbolRange {
    start: WorkspaceSymbolPosition,
    end: WorkspaceSymbolPosition,
}

#[derive(Debug, serde::Serialize)]
struct WorkspaceSymbolPosition {
    line: u32,
    character: u32,
}
