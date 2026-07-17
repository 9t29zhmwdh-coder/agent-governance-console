use crate::error::AzureError;
use opentelemetry::trace::{Span, Tracer, TracerProvider as _};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider};

/// Real OTLP (HTTP) span exporter, for shipping AGC trace spans to any
/// OTLP-compatible collector — including Azure Monitor's native OTLP
/// ingestion endpoint (`https://<region>.otelcollector.azure.com/v1/traces`,
/// see `docs/azure_integration.md`) or a self-hosted OpenTelemetry
/// Collector in front of Application Insights. Opt-in only: nothing is
/// constructed unless [`TelemetryConfig`](agc_core::TelemetryConfig) has
/// `enabled = true` and an `endpoint` set (see `agc-core::telemetry`).
pub struct OtlpExporter {
    tracer: SdkTracer,
    // Kept alive for the exporter's lifetime; dropping it shuts down the
    // provider and stops exporting.
    _provider: SdkTracerProvider,
}

impl OtlpExporter {
    /// Builds an exporter sending spans to `endpoint` over OTLP/HTTP,
    /// tagged with `service_name`. `endpoint` must be the full traces
    /// endpoint URL, including the `/v1/traces` path (e.g.
    /// `https://<region>.otelcollector.azure.com/v1/traces`, exactly as
    /// `docs/azure_integration.md` documents it) — a programmatically set
    /// endpoint is used verbatim, not treated as a base URL to append to.
    pub fn new(endpoint: &str, service_name: &str) -> Result<Self, AzureError> {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()
            .map_err(|e| AzureError::Otlp(e.to_string()))?;

        // Batch, not simple: the batch processor runs its own dedicated OS
        // thread and only ever receives spans over a channel from
        // `record_span`, so the actual (blocking) HTTP export never runs on
        // whatever thread called `record_span`. A simple/synchronous
        // exporter did the HTTP call inline on the caller's thread instead,
        // which deadlocked when `record_span` was called from inside an
        // already-running Tokio runtime (e.g. an axum handler) — the
        // exporter's internal `block_on` had nowhere free to run.
        let provider = SdkTracerProvider::builder().with_batch_exporter(exporter).build();
        let tracer = provider.tracer(service_name.to_string());

        Ok(Self { tracer, _provider: provider })
    }

    /// Records a single completed span. Mirrors
    /// [`agc_core::TelemetrySink::record_span`]'s signature so it can be
    /// wired in as a drop-in real backend by callers (see `agc-api`).
    pub fn record_span(&self, operation: &str, duration_ms: u64) {
        let mut span = self.tracer.start(operation.to_string());
        span.set_attribute(KeyValue::new("duration_ms", duration_ms as i64));
        span.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn record_span_actually_posts_to_the_otlp_http_endpoint() {
        // Real network round-trip against a mock server, not just a unit
        // test of construction: proves record_span truly sends protobuf
        // over HTTP to <endpoint>/v1/traces, the OTLP/HTTP contract any
        // real collector (including Azure Monitor's OTLP endpoint) expects.
        // Wrapped in a timeout: if the batch processor's shutdown ever
        // stops flushing before exporting again, this fails fast with a
        // clear error instead of hanging the test suite forever.
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/v1/traces"))
                .respond_with(ResponseTemplate::new(200))
                .mount(&server)
                .await;

            let endpoint = format!("{}/v1/traces", server.uri());
            let exporter = OtlpExporter::new(&endpoint, "agc-test").unwrap();
            exporter.record_span("tool_call", 42);
            // Dropping the provider shuts down the batch processor, which
            // force-flushes any pending spans before the drop returns.
            drop(exporter);

            let requests = server.received_requests().await.unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].url.path(), "/v1/traces");
        })
        .await
        .expect("record_span/shutdown did not complete within 10s");
    }
}
