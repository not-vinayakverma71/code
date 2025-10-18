/// Native LSP Gateway - Tree-sitter based language intelligence
/// 
/// Provides LSP features across ~69 languages using CST-tree-sitter:
/// - Document sync (didOpen/didChange/didClose)
/// - Document symbols
/// - Hover information
/// - Go to definition
/// - Find references
/// - Folding ranges
/// - Semantic tokens
/// - Diagnostics
/// - Workspace symbols

pub mod native;

pub use native::LspGateway;
