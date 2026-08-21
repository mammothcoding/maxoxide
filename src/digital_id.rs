//! Experimental, feature-gated partner client for MAX Digital ID age verification.
//!
//! The partner request and response schema is issued during Digital ID
//! onboarding, so this client intentionally accepts caller-owned serializable
//! types instead of freezing an undocumented public wire model.
//!
//! # Experimental status
//!
//! This integration has not been verified against the live partner service. Before
//! production use, validate the endpoint, authorization, and caller-owned models
//! against the current onboarding documentation issued to your organization by MAX.

use std::time::Duration;

use reqwest::{Client, StatusCode, header::HeaderValue};
use serde::{Serialize, de::DeserializeOwned};
use url::Url;

use crate::{ApiError, MaxError, Result, bot::RussianTlsExt};

/// Production MAX Digital ID age-verification endpoint.
pub const AGE_VERIFICATION_ENDPOINT: &str =
    "https://ext-api2.max.ru/v2/business/pos/age-verification";

/// Experimental independent client authenticated with a MAX Digital ID partner token.
///
/// The private partner contract and live service behavior must be validated during
/// onboarding before production use.
#[derive(Clone)]
pub struct DigitalIdClient {
    token: HeaderValue,
    client: Client,
    endpoint: Url,
    timeout: Duration,
}

impl DigitalIdClient {
    /// Creates a verified-TLS client for the production endpoint.
    pub fn new(token: impl AsRef<str>) -> Result<Self> {
        let client = Client::builder().russian_tls()?.build()?;
        Self::with_client(token, client)
    }

    /// Creates a Digital ID client with caller-owned HTTP settings.
    pub fn with_client(token: impl AsRef<str>, client: Client) -> Result<Self> {
        let token = token.as_ref();
        if token.is_empty() {
            return Err(MaxError::Configuration(
                "Digital ID token must not be empty".into(),
            ));
        }
        let token = HeaderValue::from_str(token).map_err(|error| {
            MaxError::Configuration(format!("invalid Digital ID token: {error}"))
        })?;
        let endpoint = Url::parse(AGE_VERIFICATION_ENDPOINT)
            .map_err(|error| MaxError::Configuration(error.to_string()))?;
        Ok(Self {
            token,
            client,
            endpoint,
            timeout: Duration::from_secs(30),
        })
    }

    /// Overrides the endpoint, primarily for a private stand or mock server.
    pub fn endpoint(mut self, endpoint: impl AsRef<str>) -> Result<Self> {
        let endpoint = Url::parse(endpoint.as_ref())
            .map_err(|error| MaxError::Configuration(format!("invalid endpoint: {error}")))?;
        if endpoint.scheme() != "https" && !endpoint.host_str().is_some_and(is_loopback_host) {
            return Err(MaxError::Configuration(
                "Digital ID endpoint must use HTTPS unless it targets localhost".into(),
            ));
        }
        self.endpoint = endpoint;
        Ok(self)
    }

    /// Overrides the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() {
            return Err(MaxError::Configuration(
                "Digital ID timeout must be greater than zero".into(),
            ));
        }
        self.timeout = timeout;
        Ok(self)
    }

    /// Submits a partner-defined age-verification request and decodes its response.
    pub async fn verify_age<Request, Response>(&self, request: &Request) -> Result<Response>
    where
        Request: Serialize + ?Sized,
        Response: DeserializeOwned,
    {
        let response = self
            .client
            .post(self.endpoint.clone())
            .header("Authorization", self.token.clone())
            .timeout(self.timeout)
            .json(request)
            .send()
            .await?;
        parse_response(response).await
    }

    /// Submits and returns arbitrary JSON when an onboarding schema is dynamic.
    pub async fn verify_age_raw(&self, request: &serde_json::Value) -> Result<serde_json::Value> {
        self.verify_age(request).await
    }
}

async fn parse_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status();
    let body = response.text().await?;
    if status.is_success() {
        Ok(serde_json::from_str(&body)?)
    } else {
        Err(api_error(status, body).into())
    }
}

fn api_error(status: StatusCode, body: String) -> ApiError {
    let value = serde_json::from_str::<serde_json::Value>(&body).ok();
    let code = value
        .as_ref()
        .and_then(|value| value.get("code"))
        .and_then(serde_json::Value::as_str)
        .map(String::from);
    let message = value
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("Digital ID request failed")
        })
        .to_string();
    ApiError::new(status.as_u16(), code, message).with_raw_response(body)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

#[cfg(test)]
mod tests {
    use super::{AGE_VERIFICATION_ENDPOINT, DigitalIdClient};

    #[test]
    fn validates_configuration_without_exposing_token() {
        assert!(DigitalIdClient::with_client("", reqwest::Client::new()).is_err());
        let client = DigitalIdClient::with_client("secret", reqwest::Client::new()).unwrap();
        assert_eq!(client.endpoint.as_str(), AGE_VERIFICATION_ENDPOINT);
        assert!(client.clone().endpoint("http://example.com/test").is_err());
        assert!(client.endpoint("http://127.0.0.1:8080/test").is_ok());
    }
}
