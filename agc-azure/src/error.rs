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

#[cfg(test)]
mod tests {
    use super::*;

    /// Haelt die Fehlertexte fest. Sie gehen in Logs und in Antworten der API,
    /// sind also nach aussen sichtbar. Ein Versionssprung von `thiserror` darf
    /// die Formatierung nicht stillschweigend veraendern.
    #[test]
    fn fehlertexte_bleiben_wie_sie_sind() {
        let status = AzureError::Status {
            status: reqwest::StatusCode::FORBIDDEN,
            body: "insufficient privileges".into(),
        };
        assert_eq!(
            status.to_string(),
            "unexpected status 403 Forbidden: insufficient privileges"
        );

        let otlp = AzureError::Otlp("exporter shut down".into());
        assert_eq!(otlp.to_string(), "OTLP exporter error: exporter shut down");
    }
}
