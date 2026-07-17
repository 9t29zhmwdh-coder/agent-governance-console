use crate::error::AzureError;
use agc_core::AuditRecord;
use std::time::Duration;

const API_VERSION: &str = "2023-01-01";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Pushes AGC audit records to an Azure Monitor Data Collection Rule (DCR)
/// custom table via the Logs Ingestion API. See
/// `docs/azure_integration.md` for the required DCE/DCR/table setup
/// (`scripts/azure_setup.sh` provisions it) and `scripts/export_audit.sh`
/// for pulling records out of a running AGC instance first.
pub struct MonitorIngestClient {
    client: reqwest::Client,
    /// Data Collection Endpoint, e.g.
    /// `https://<name>.<region>-1.ingest.monitor.azure.com`.
    dce_endpoint: String,
    /// The DCR's immutable ID (`dcr-...`), not its display name.
    dcr_immutable_id: String,
    /// The custom table's stream name, e.g. `Custom-AGCAudit_CL`.
    stream_name: String,
}

impl MonitorIngestClient {
    pub fn new(
        dce_endpoint: impl Into<String>,
        dcr_immutable_id: impl Into<String>,
        stream_name: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("building the Monitor ingest HTTP client");
        Self {
            client,
            dce_endpoint: dce_endpoint.into(),
            dcr_immutable_id: dcr_immutable_id.into(),
            stream_name: stream_name.into(),
        }
    }

    fn ingest_url(&self) -> String {
        format!(
            "{}/dataCollectionRules/{}/streams/{}?api-version={API_VERSION}",
            self.dce_endpoint.trim_end_matches('/'),
            self.dcr_immutable_id,
            self.stream_name,
        )
    }

    /// Pushes a batch of audit records, authenticated with `token` (an AAD
    /// bearer token scoped to `https://monitor.azure.com/.default`, e.g.
    /// from [`crate::ManagedIdentityCredential`]). The Logs Ingestion API
    /// expects a JSON array, not NDJSON, despite the local export format.
    pub async fn push_records(&self, token: &str, records: &[AuditRecord]) -> Result<(), AzureError> {
        if records.is_empty() {
            return Ok(());
        }
        let response = self
            .client
            .post(self.ingest_url())
            .bearer_auth(token)
            .json(records)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AzureError::Status { status, body });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agc_core::AuditOutcome;
    use chrono::Utc;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_record() -> AuditRecord {
        AuditRecord {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            agent_id: "agent-1".into(),
            action: "tool_call".into(),
            outcome: AuditOutcome::Allowed,
            policy_id: None,
            details: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn push_records_posts_to_the_correct_dcr_stream_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/dataCollectionRules/dcr-abc123/streams/Custom-AGCAudit_CL"))
            .and(query_param("api-version", "2023-01-01"))
            .and(header("Authorization", "Bearer fake-token"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = MonitorIngestClient::new(server.uri(), "dcr-abc123", "Custom-AGCAudit_CL");
        client.push_records("fake-token", &[sample_record()]).await.unwrap();
    }

    #[tokio::test]
    async fn push_records_sends_a_json_array_body() {
        let server = MockServer::start().await;
        let record = sample_record();
        Mock::given(method("POST"))
            .and(body_json(serde_json::json!([record])))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = MonitorIngestClient::new(server.uri(), "dcr-abc123", "Custom-AGCAudit_CL");
        client.push_records("fake-token", &[record]).await.unwrap();
    }

    #[tokio::test]
    async fn push_records_is_a_noop_for_an_empty_batch() {
        // No mock registered at all: if this made a request, wiremock's
        // "no matcher" 404 fallback would turn this into an error.
        let server = MockServer::start().await;
        let client = MonitorIngestClient::new(server.uri(), "dcr-abc123", "Custom-AGCAudit_CL");
        client.push_records("fake-token", &[]).await.unwrap();
    }

    #[tokio::test]
    async fn push_records_surfaces_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&server)
            .await;

        let client = MonitorIngestClient::new(server.uri(), "dcr-abc123", "Custom-AGCAudit_CL");
        let err = client.push_records("fake-token", &[sample_record()]).await.unwrap_err();
        assert!(matches!(err, AzureError::Status { .. }));
    }
}
