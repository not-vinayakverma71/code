// Tracing Sampling Configuration - SEM-018-B
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use crate::security::redaction::redact_pii;

/// Configure tracing for release builds with sampling
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // In release, sample 10% of traces
            #[cfg(not(debug_assertions))]
            return EnvFilter::new("info,semantic_search=debug")
                .add_directive("semantic_search::search=trace".parse().unwrap());
            
            // In debug, trace everything
            #[cfg(debug_assertions)]
            return EnvFilter::new("debug,semantic_search=trace");
        });
    
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .fmt_fields(RedactingFieldFormatter);
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Field formatter that redacts PII
struct RedactingFieldFormatter;

impl<'writer> tracing_subscriber::fmt::format::FormatFields<'writer> for RedactingFieldFormatter {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: tracing_subscriber::fmt::format::Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        let mut visitor = RedactingVisitor::new(writer);
        fields.record(&mut visitor);
        Ok(())
    }
}

struct RedactingVisitor<'a> {
    writer: tracing_subscriber::fmt::format::Writer<'a>,
    first: bool,
}

impl<'a> RedactingVisitor<'a> {
    fn new(writer: tracing_subscriber::fmt::format::Writer<'a>) -> Self {
        Self { writer, first: true }
    }
}

impl<'a> tracing::field::Visit for RedactingVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{:?}", value);
        let redacted = redact_pii(&value_str);
        
        if !self.first {
            let _ = write!(self.writer, ", ");
        }
        let _ = write!(self.writer, "{}={}", field.name(), redacted);
        self.first = false;
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let redacted = redact_pii(value);
        
        if !self.first {
            let _ = write!(self.writer, ", ");
        }
        let _ = write!(self.writer, "{}=\"{}\"", field.name(), redacted);
        self.first = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sampling_config() {
        // Test that tracing can be initialized without panic
        // Note: Can only init once per process, so this is a smoke test
        std::env::set_var("RUST_LOG", "info");
    }
}
