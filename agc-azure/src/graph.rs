use crate::error::AzureError;
use std::time::Duration;

const DEFAULT_GRAPH_BASE_URL: &str = "https://graph.microsoft.com/v1.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// An Entra ID (Azure AD) app registration, as returned by Microsoft Graph.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AppRegistration {
    pub id: String,
    #[serde(rename = "appId")]
    pub app_id: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GraphListResponse<T> {
    value: Vec<T>,
}

/// Reads agent app registrations from Microsoft Graph: app registrations
/// tagged `agc-agent` in Entra ID are treated as AGC agent identities. See
/// `docs/azure_integration.md` for how to tag one.
pub struct GraphClient {
    client: reqwest::Client,
    base_url: String,
}

impl GraphClient {
    /// Uses the real Microsoft Graph v1.0 endpoint.
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_GRAPH_BASE_URL)
    }

    /// Points at a custom base URL, for testing against a mock server.
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("building the Graph HTTP client");
        Self { client, base_url: base_url.into() }
    }

    /// Lists app registrations tagged `agc-agent`, authenticated with
    /// `token` (an AAD bearer token scoped to
    /// `https://graph.microsoft.com/.default`, requiring the
    /// `Application.Read.All` permission).
    pub async fn list_agent_app_registrations(
        &self,
        token: &str,
    ) -> Result<Vec<AppRegistration>, AzureError> {
        let url = format!("{}/applications", self.base_url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .query(&[("$filter", "tags/any(t:t eq 'agc-agent')")])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AzureError::Status { status, body });
        }
        let parsed: GraphListResponse<AppRegistration> = response.json().await?;
        Ok(parsed.value)
    }
}

impl Default for GraphClient {
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
    async fn list_agent_app_registrations_filters_by_tag() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/applications"))
            .and(query_param("$filter", "tags/any(t:t eq 'agc-agent')"))
            .and(header("Authorization", "Bearer fake-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": [
                    {"id": "obj-1", "appId": "app-1", "displayName": "Trading Agent", "tags": ["agc-agent"]}
                ]
            })))
            .mount(&server)
            .await;

        let client = GraphClient::with_base_url(server.uri());
        let apps = client.list_agent_app_registrations("fake-token").await.unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].app_id, "app-1");
        assert_eq!(apps[0].display_name.as_deref(), Some("Trading Agent"));
    }

    #[tokio::test]
    async fn list_agent_app_registrations_handles_empty_result() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"value": []})))
            .mount(&server)
            .await;

        let client = GraphClient::with_base_url(server.uri());
        let apps = client.list_agent_app_registrations("fake-token").await.unwrap();
        assert!(apps.is_empty());
    }

    #[tokio::test]
    async fn list_agent_app_registrations_surfaces_non_success_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
            .mount(&server)
            .await;

        let client = GraphClient::with_base_url(server.uri());
        let err = client.list_agent_app_registrations("bad-token").await.unwrap_err();
        assert!(matches!(err, AzureError::Status { .. }));
    }
}
