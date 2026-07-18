/// Errors from any Azure integration point in this crate.
#[derive(Debug, thiserror::Error)]
pub enum AzureError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("unexpected status {status}: {body}")]
    Status { status: reqwest::StatusCode, body: String },
    #[error("OTLP exporter error: {0}")]
    Otlp(String),
}
