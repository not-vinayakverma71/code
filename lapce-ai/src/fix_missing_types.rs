// Temporary fixes for missing types
pub type ChatCompletionStream = futures::stream::BoxStream<'static, Result<String, anyhow::Error>>;
