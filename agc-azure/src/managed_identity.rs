use crate::error::AzureError;
use std::time::Duration;

/// Azure's real Instance Metadata Service endpoint. Only reachable from
/// inside an Azure-hosted compute resource (VM, App Service, Container
/// Instance, AKS pod) that has a managed identity assigned; unreachable
/// from anywhere else, including this crate's own tests, which is why
/// `ManagedIdentityCredential::with_endpoint` exists.
pub const DEFAULT_IMDS_ENDPOINT: &str =
    "http://169.254.169.254/metadata/identity/oauth2/token";

const IMDS_API_VERSION: &str = "2018-02-01";

/// IMDS responds within milliseconds when it's actually there; off Azure,
/// the link-local address 169.254.169.254 is often silently unroutable
/// rather than actively refused, so without a timeout a connection
/// attempt can hang far longer than reqwest's unset default. Matches the
/// short probe timeout other Azure SDKs use for IMDS.
const IMDS_TIMEOUT: Duration = Duration::from_secs(2);

/// An AAD access token as returned by IMDS.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AccessToken {
    pub access_token: String,
    /// Unix timestamp (seconds) as a string; IMDS does not return a number.
    pub expires_on: String,
    pub resource: String,
    pub token_type: String,
}

impl AccessToken {
    /// Parses `expires_on` into a real timestamp. `None` if IMDS ever
    /// returns something unparseable (defensive; the real service always
    /// returns a valid unix timestamp string).
    pub fn expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.expires_on.parse::<i64>().ok().and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
    }
}

/// Fetches AAD access tokens via Managed Identity (IMDS), so AGC never
/// needs a client secret configured to call Azure Monitor or Microsoft
/// Graph. Supports both system-assigned (no `client_id`) and user-assigned
/// (`with_client_id`) managed identities.
pub struct ManagedIdentityCredential {
    client: reqwest::Client,
    imds_endpoint: String,
    client_id: Option<String>,
}

impl ManagedIdentityCredential {
    /// Uses the real IMDS endpoint; only works when actually running on
    /// Azure with a managed identity assigned.
    pub fn new() -> Self {
        Self::with_endpoint(DEFAULT_IMDS_ENDPOINT)
    }

    /// Points at a custom endpoint instead of the real IMDS URL. Exists so
    /// tests (and this crate's own test suite) can verify request
    /// construction against a local mock server, since the real endpoint
    /// is unreachable outside Azure.
    pub fn with_endpoint(imds_endpoint: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(IMDS_TIMEOUT)
            .build()
            .expect("building the IMDS HTTP client");
        Self { client, imds_endpoint: imds_endpoint.into(), client_id: None }
    }

    /// Scopes token requests to a specific user-assigned managed identity.
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = Some(client_id.into());
        self
    }

    /// Requests a token scoped to `resource` (e.g.
    /// `https://monitor.azure.com/` or `https://graph.microsoft.com/`).
    pub async fn get_token(&self, resource: &str) -> Result<AccessToken, AzureError> {
        let mut query = vec![("api-version", IMDS_API_VERSION), ("resource", resource)];
        if let Some(client_id) = &self.client_id {
            query.push(("client_id", client_id));
        }

        let response = self
            .client
            .get(&self.imds_endpoint)
            .header("Metadata", "true")
            .query(&query)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AzureError::Status { status, body });
        }
        Ok(response.json::<AccessToken>().await?)
    }
}

impl Default for ManagedIdentityCredential {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn get_token_sends_metadata_header_and_resource_query() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metadata/identity/oauth2/token"))
            .and(header("Metadata", "true"))
            .and(query_param("resource", "https://monitor.azure.com/"))
            .and(query_param("api-version", "2018-02-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "fake-token",
                "expires_on": "1900000000",
                "resource": "https://monitor.azure.com/",
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;

        let cred = ManagedIdentityCredential::with_endpoint(format!(
            "{}/metadata/identity/oauth2/token",
            server.uri()
        ));
        let token = cred.get_token("https://monitor.azure.com/").await.unwrap();
        assert_eq!(token.access_token, "fake-token");
        assert!(token.expires_at().is_some());
    }

    #[tokio::test]
    async fn get_token_includes_client_id_for_user_assigned_identity() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("client_id", "abc-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "fake-token",
                "expires_on": "1900000000",
                "resource": "https://graph.microsoft.com/",
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;

        let cred = ManagedIdentityCredential::with_endpoint(server.uri()).with_client_id("abc-123");
        let token = cred.get_token("https://graph.microsoft.com/").await.unwrap();
        assert_eq!(token.access_token, "fake-token");
    }

    #[tokio::test]
    async fn get_token_times_out_instead_of_hanging_forever() {
        // Regression test for a real hang found during development:
        // reqwest::Client::new() has no timeout at all by default, so a
        // request to an address that accepts the TCP connection but never
        // responds (exactly what 169.254.169.254 does off Azure) hung the
        // whole CLI indefinitely instead of failing fast.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            // Accept and hold the connection open without ever writing a
            // response, simulating an unresponsive/black-holed endpoint.
            let _ = listener.accept().await;
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        });

        let cred = ManagedIdentityCredential::with_endpoint(format!("http://{addr}/token"));
        let started = std::time::Instant::now();
        let result = tokio::time::timeout(std::time::Duration::from_secs(5), cred.get_token("https://monitor.azure.com/")).await;

        let outer = result.expect("get_token itself must return within 5s (its own 2s timeout), not hang the test's 5s guard");
        assert!(outer.is_err(), "a request to an unresponsive endpoint must fail, not succeed");
        assert!(started.elapsed() < std::time::Duration::from_secs(4), "took {:?}, expected ~2s client timeout", started.elapsed());
    }

    #[tokio::test]
    async fn get_token_surfaces_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
            .mount(&server)
            .await;

        let cred = ManagedIdentityCredential::with_endpoint(server.uri());
        let err = cred.get_token("https://monitor.azure.com/").await.unwrap_err();
        assert!(matches!(err, AzureError::Status { .. }));
    }
}
