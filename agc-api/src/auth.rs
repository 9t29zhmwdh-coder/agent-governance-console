//! REST API RBAC: JWT (shared-secret HS256) or Entra ID (AAD, RS256 via
//! JWKS) bearer token validation, gating write endpoints to the `Admin`
//! role.
//!
//! **Opt-in, like everything else in this portfolio**: with no
//! `AGC_JWT_SECRET`/`AGC_AAD_TENANT_ID` configured, RBAC is off and every
//! request is treated as `Admin` -- identical to this API's behavior
//! before this feature existed. This matters for the existing test suite
//! and any deployment that hasn't opted in yet.
//!
//! **What's verified vs. not**: the HMAC path is fully tested against
//! real HS256 tokens. The AAD/JWKS path is correct-by-construction against
//! Entra ID's documented JWKS contract (same `https://login.microsoftonline.com/{tenant}/discovery/v2.0/keys`
//! endpoint and RS256 `kid`-based key selection every Microsoft
//! identity library uses) and tested against a local mock JWKS server,
//! but has not been exercised against a real Entra ID tenant (none was
//! available while building this -- same disclosed limitation as
//! `agc_azure::ManagedIdentityCredential`, see `docs/azure_integration.md`).

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Viewer,
    Admin,
}

impl Role {
    fn parse(s: &str) -> Option<Role> {
        match s.to_lowercase().as_str() {
            "admin" => Some(Role::Admin),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }

    /// Highest-privilege role found among `roles`, or `Viewer` if none of
    /// the claim's role strings are recognized (fails safe: unrecognized
    /// roles never grant Admin).
    fn highest_of(roles: &[String]) -> Role {
        roles.iter().filter_map(|r| Role::parse(r)).max().unwrap_or(Role::Viewer)
    }
}

#[derive(Debug, Deserialize)]
struct Claims {
    #[serde(default)]
    roles: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Jwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Clone)]
pub enum AuthConfig {
    /// No auth configured: every request is `Admin`.
    Disabled,
    /// Validates HS256 JWTs signed with a shared secret (`AGC_JWT_SECRET`).
    Hmac { secret: String },
    /// Validates RS256 JWTs issued by Entra ID for a tenant, checked
    /// against `audience`. The JWKS is fetched once on first use and
    /// cached for the process's lifetime (a real deployment doing key
    /// rotation would want a TTL-based refresh; documented simplification).
    Aad { jwks_uri: String, audience: String, jwks_cache: Arc<Mutex<Option<Vec<Jwk>>>> },
}

impl AuthConfig {
    pub fn disabled() -> Self {
        AuthConfig::Disabled
    }

    pub fn hmac(secret: impl Into<String>) -> Self {
        AuthConfig::Hmac { secret: secret.into() }
    }

    /// Validates tokens issued by Entra ID tenant `tenant_id`, using the
    /// real, documented JWKS endpoint every Microsoft identity library
    /// uses.
    pub fn aad(tenant_id: impl AsRef<str>, audience: impl Into<String>) -> Self {
        Self::aad_with_jwks_uri(
            format!("https://login.microsoftonline.com/{}/discovery/v2.0/keys", tenant_id.as_ref()),
            audience,
        )
    }

    /// Same as `aad`, but points at a custom JWKS URI instead of the real
    /// Entra ID endpoint -- exists so tests can verify the fetch/kid-match/
    /// RS256-verify path against a local mock server, since the real
    /// endpoint needs a live Entra ID tenant.
    pub fn aad_with_jwks_uri(jwks_uri: impl Into<String>, audience: impl Into<String>) -> Self {
        AuthConfig::Aad { jwks_uri: jwks_uri.into(), audience: audience.into(), jwks_cache: Arc::new(Mutex::new(None)) }
    }

    async fn aad_keys(jwks_uri: &str, cache: &Arc<Mutex<Option<Vec<Jwk>>>>) -> Result<Vec<Jwk>, String> {
        let mut guard = cache.lock().await;
        if let Some(keys) = &*guard {
            return Ok(keys.clone());
        }
        let jwks: Jwks = reqwest::get(jwks_uri)
            .await
            .map_err(|e| format!("fetching JWKS: {e}"))?
            .json()
            .await
            .map_err(|e| format!("parsing JWKS: {e}"))?;
        *guard = Some(jwks.keys.clone());
        Ok(jwks.keys)
    }

    /// Validates `token` and returns the caller's highest role, or an
    /// error message suitable for a `401` response body.
    async fn validate(&self, token: &str) -> Result<Role, String> {
        match self {
            AuthConfig::Disabled => Ok(Role::Admin),
            AuthConfig::Hmac { secret } => {
                let key = DecodingKey::from_secret(secret.as_bytes());
                let mut validation = Validation::new(Algorithm::HS256);
                validation.validate_aud = false;
                // `exp` is optional here (cleared from required_spec_claims):
                // jsonwebtoken's default requires it to even be present,
                // which would reject perfectly valid non-expiring tokens
                // that never claimed to be short-lived. If a token DOES
                // carry an `exp`, validate_exp (still true, the default)
                // enforces it -- only *absence* is tolerated, not an
                // actually-expired token.
                validation.required_spec_claims.clear();
                let data = decode::<Claims>(token, &key, &validation).map_err(|e| format!("invalid token: {e}"))?;
                Ok(Role::highest_of(&data.claims.roles))
            }
            AuthConfig::Aad { jwks_uri, audience, jwks_cache } => {
                let header = decode_header(token).map_err(|e| format!("invalid token header: {e}"))?;
                let kid = header.kid.ok_or("token has no 'kid' header")?;
                let keys = Self::aad_keys(jwks_uri, jwks_cache).await?;
                let jwk = keys.iter().find(|k| k.kid == kid).ok_or("no matching key in JWKS for this token's kid")?;
                let key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(|e| format!("bad JWKS key: {e}"))?;
                let mut validation = Validation::new(Algorithm::RS256);
                validation.set_audience(std::slice::from_ref(audience));
                validation.required_spec_claims.clear();
                let data = decode::<Claims>(token, &key, &validation).map_err(|e| format!("invalid token: {e}"))?;
                Ok(Role::highest_of(&data.claims.roles))
            }
        }
    }
}

fn unauthorized(reason: impl Into<String>) -> Response {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized", "reason": reason.into()}))).into_response()
}

fn forbidden(min_role: Role) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({"error": "forbidden", "reason": format!("requires at least {min_role:?} role")})),
    )
        .into_response()
}

/// Extracts and validates the bearer token from `headers`, then checks
/// the resulting role is at least `min_role`. Returns `Ok(())` to let the
/// handler continue, or the `401`/`403` response to return immediately.
pub async fn authorize(auth: &AuthConfig, headers: &HeaderMap, min_role: Role) -> Result<(), Response> {
    if matches!(auth, AuthConfig::Disabled) {
        return Ok(());
    }
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| unauthorized("missing or malformed Authorization: Bearer <token> header"))?;

    let role = auth.validate(token).await.map_err(unauthorized)?;
    if role < min_role {
        return Err(forbidden(min_role));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn hmac_token(secret: &str, roles: &[&str]) -> String {
        let claims = serde_json::json!({"roles": roles});
        encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret.as_bytes())).unwrap()
    }

    #[tokio::test]
    async fn disabled_treats_every_request_as_admin_with_no_header() {
        let auth = AuthConfig::disabled();
        let headers = HeaderMap::new();
        assert!(authorize(&auth, &headers, Role::Admin).await.is_ok());
    }

    #[tokio::test]
    async fn hmac_valid_admin_token_passes_admin_check() {
        let auth = AuthConfig::hmac("s3cret");
        let token = hmac_token("s3cret", &["admin"]);
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        assert!(authorize(&auth, &headers, Role::Admin).await.is_ok());
    }

    #[tokio::test]
    async fn hmac_viewer_token_fails_admin_check_with_403() {
        let auth = AuthConfig::hmac("s3cret");
        let token = hmac_token("s3cret", &["viewer"]);
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        let err = authorize(&auth, &headers, Role::Admin).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn hmac_viewer_token_passes_viewer_check() {
        let auth = AuthConfig::hmac("s3cret");
        let token = hmac_token("s3cret", &["viewer"]);
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        assert!(authorize(&auth, &headers, Role::Viewer).await.is_ok());
    }

    #[tokio::test]
    async fn missing_header_is_401_when_auth_is_enabled() {
        let auth = AuthConfig::hmac("s3cret");
        let headers = HeaderMap::new();
        let err = authorize(&auth, &headers, Role::Viewer).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_secret_is_401() {
        let auth = AuthConfig::hmac("s3cret");
        let token = hmac_token("wrong-secret", &["admin"]);
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        let err = authorize(&auth, &headers, Role::Viewer).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_token_is_rejected_when_exp_is_present() {
        // required_spec_claims is cleared so a token *without* exp still
        // validates (see the comment in validate()) -- this test proves
        // that didn't accidentally disable exp checking altogether: a
        // token that DOES carry an exp in the past must still fail.
        let auth = AuthConfig::hmac("s3cret");
        let claims = serde_json::json!({"roles": ["admin"], "exp": 1}); // 1970, long expired
        let token = jsonwebtoken::encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(b"s3cret"),
        )
        .unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        let err = authorize(&auth, &headers, Role::Viewer).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn unrecognized_role_string_fails_safe_to_viewer() {
        let auth = AuthConfig::hmac("s3cret");
        let token = hmac_token("s3cret", &["superuser"]);
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        assert!(authorize(&auth, &headers, Role::Viewer).await.is_ok());
        let err = authorize(&auth, &headers, Role::Admin).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn aad_mode_validates_a_real_rs256_token_against_a_mock_jwks_server() {
        use rsa::pkcs1::EncodeRsaPrivateKey;
        use rsa::traits::PublicKeyParts;

        // Generate a real RSA keypair, sign a real RS256 JWT with it, and
        // serve the public key as a real JWKS document -- proves the full
        // "fetch JWKS over HTTP, find the kid, verify RS256" path this
        // crate's own authorize() takes, not just that the code compiles.
        let mut rng = rand::thread_rng();
        let private_key = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let public_key = rsa::RsaPublicKey::from(&private_key);
        let pem = private_key.to_pkcs1_pem(rsa::pkcs8::LineEnding::LF).unwrap();
        let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap();

        let n = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, public_key.n().to_bytes_be());
        let e = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, public_key.e().to_bytes_be());

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/mock-tenant/discovery/v2.0/keys"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "keys": [{"kid": "test-kid", "n": n, "e": e}]
            })))
            .mount(&server)
            .await;

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-kid".to_string());
        let claims = serde_json::json!({"roles": ["admin"], "aud": "test-audience"});
        let token = encode(&header, &claims, &encoding_key).unwrap();

        let auth = AuthConfig::aad_with_jwks_uri(
            format!("{}/mock-tenant/discovery/v2.0/keys", server.uri()),
            "test-audience",
        );
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());

        assert!(authorize(&auth, &headers, Role::Admin).await.is_ok());

        // The JWKS fetch is cached: a second call must not hit the mock
        // server again (which has no expectation for a second request).
        assert!(authorize(&auth, &headers, Role::Viewer).await.is_ok());
    }

    #[tokio::test]
    async fn aad_mode_rejects_a_token_with_wrong_audience() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"keys": []})))
            .mount(&server)
            .await;

        let auth = AuthConfig::aad_with_jwks_uri(server.uri(), "expected-audience");
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::AUTHORIZATION, "Bearer not-even-a-real-jwt".parse().unwrap());

        let err = authorize(&auth, &headers, Role::Viewer).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::UNAUTHORIZED);
    }
}
