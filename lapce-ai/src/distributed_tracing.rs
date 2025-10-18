/// Distributed Tracing with Jaeger - Day 36 AM
use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace, Resource},
    trace::{Span, Tracer, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::time::Duration;

pub fn init_tracing(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317")
                .with_timeout(Duration::from_secs(3))
        )
        .with_trace_config(
            trace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", service_name.to_string()),
            ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    tracing_subscriber::registry()
        .with(telemetry)
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    Ok(())
}

pub struct TracedOperation {
    span: tracing::Span,
}

impl TracedOperation {
    pub fn new(name: &str) -> Self {
        let span = tracing::info_span!(
            "operation",
            name = name,
            start_time = %std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        Self { span }
    }
    
    pub fn record_event(&self, event: &str) {
        self.span.in_scope(|| {
            tracing::info!(event = event);
        });
    }
    
    pub fn record_error(&self, error: &str) {
        self.span.in_scope(|| {
            tracing::error!(error = error);
        });
    }
}

#[macro_export]
macro_rules! trace_operation {
    ($name:expr, $body:expr) => {{
        let _op = TracedOperation::new($name);
        let _guard = _op.span.enter();
        $body
    }};
}
