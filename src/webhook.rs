//! Framework-neutral webhook processing with optional Axum and Actix adapters.
//!
//! [`WebhookService`] owns the MAX-specific behavior: secret verification,
//! payload limits, parsing, backpressure, dispatch timeout, and handler errors.
//! Web frameworks only translate their request and response types.

use std::{sync::Arc, time::Duration};

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use tracing::{error, warn};

use crate::{
    dispatcher::{Dispatcher, DispatcherShutdown},
    errors::{MaxError, Result},
};

/// Header MAX sends when a webhook subscription has a shared secret.
pub const SECRET_HEADER: &str = "x-max-bot-api-secret";

/// Framework-independent webhook response classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebhookOutcome {
    /// The payload was accepted and all matching handlers succeeded.
    Accepted,
    /// The request body was not a valid MAX update object.
    BadRequest,
    /// The configured shared secret was absent or incorrect.
    Unauthorized,
    /// The request exceeded the configured body-size limit.
    PayloadTooLarge,
    /// The service reached its configured in-flight request limit.
    Busy,
    /// Dispatcher processing exceeded the configured timeout.
    DispatchTimedOut,
    /// A dispatcher handler returned an error.
    HandlerFailed,
}

impl WebhookOutcome {
    /// HTTP status code adapters should return for this outcome.
    pub const fn status_code(self) -> u16 {
        match self {
            Self::Accepted => 200,
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::PayloadTooLarge => 413,
            Self::Busy => 503,
            Self::DispatchTimedOut => 503,
            Self::HandlerFailed => 500,
        }
    }
}

/// Shared MAX webhook processor used by every framework adapter.
#[derive(Clone)]
pub struct WebhookService {
    dispatcher: Arc<Dispatcher>,
    secret_digest: Option<[u8; 32]>,
    max_body_size: usize,
    dispatch_timeout: Duration,
    in_flight: Arc<Semaphore>,
}

impl WebhookService {
    /// Creates a service with a 1 MiB body limit, 25-second dispatch timeout,
    /// and at most 64 concurrently processed requests.
    pub fn new(dispatcher: Dispatcher) -> Self {
        Self::from_shared(Arc::new(dispatcher))
    }

    /// Creates a service around an already shared dispatcher.
    pub fn from_shared(dispatcher: Arc<Dispatcher>) -> Self {
        Self {
            dispatcher,
            secret_digest: None,
            max_body_size: 1024 * 1024,
            dispatch_timeout: Duration::from_secs(25),
            in_flight: Arc::new(Semaphore::new(64)),
        }
    }

    /// Requires the subscription secret without storing it in plaintext.
    pub fn secret(mut self, secret: impl AsRef<[u8]>) -> Result<Self> {
        let secret = secret.as_ref();
        if secret.is_empty() {
            return Err(MaxError::Configuration(
                "webhook secret must not be empty".into(),
            ));
        }
        self.secret_digest = Some(Sha256::digest(secret).into());
        Ok(self)
    }

    /// Replaces the maximum accepted JSON request size.
    pub fn max_body_size(mut self, bytes: usize) -> Result<Self> {
        if bytes == 0 {
            return Err(MaxError::Configuration(
                "webhook body limit must be greater than zero".into(),
            ));
        }
        self.max_body_size = bytes;
        Ok(self)
    }

    /// Replaces the time allowed for dispatcher processing.
    pub fn dispatch_timeout(mut self, timeout: Duration) -> Result<Self> {
        if timeout.is_zero() {
            return Err(MaxError::Configuration(
                "webhook dispatch timeout must be greater than zero".into(),
            ));
        }
        self.dispatch_timeout = timeout;
        Ok(self)
    }

    /// Replaces the maximum number of in-flight webhook requests.
    pub fn max_in_flight(mut self, maximum: usize) -> Result<Self> {
        if maximum == 0 {
            return Err(MaxError::Configuration(
                "webhook in-flight limit must be greater than zero".into(),
            ));
        }
        self.in_flight = Arc::new(Semaphore::new(maximum));
        Ok(self)
    }

    /// Returns the configured request body limit.
    pub const fn body_limit(&self) -> usize {
        self.max_body_size
    }

    /// Returns a handle that can stop polling/scheduled dispatcher work.
    pub fn shutdown_handle(&self) -> DispatcherShutdown {
        self.dispatcher.shutdown_handle()
    }

    /// Validates and dispatches one raw webhook body.
    pub async fn handle(&self, provided_secret: Option<&str>, body: &[u8]) -> WebhookOutcome {
        if body.len() > self.max_body_size {
            return WebhookOutcome::PayloadTooLarge;
        }
        if !self.secret_matches(provided_secret) {
            warn!("MAX webhook secret validation failed");
            return WebhookOutcome::Unauthorized;
        }

        let update = match serde_json::from_slice::<serde_json::Value>(body) {
            Ok(update) if update.is_object() => update,
            Ok(_) => return WebhookOutcome::BadRequest,
            Err(error) => {
                warn!("Invalid MAX webhook JSON: {error}");
                return WebhookOutcome::BadRequest;
            }
        };
        let Ok(_permit) = self.in_flight.try_acquire() else {
            return WebhookOutcome::Busy;
        };

        match tokio::time::timeout(self.dispatch_timeout, self.dispatcher.dispatch_raw(update))
            .await
        {
            Ok(Ok(())) => WebhookOutcome::Accepted,
            Ok(Err(error)) => {
                error!("MAX webhook handler failed: {error}");
                WebhookOutcome::HandlerFailed
            }
            Err(_) => {
                warn!("MAX webhook dispatch timed out");
                WebhookOutcome::DispatchTimedOut
            }
        }
    }

    fn secret_matches(&self, provided_secret: Option<&str>) -> bool {
        match (self.secret_digest, provided_secret) {
            (None, _) => true,
            (Some(expected), Some(provided)) => {
                let provided: [u8; 32] = Sha256::digest(provided.as_bytes()).into();
                bool::from(expected.ct_eq(&provided))
            }
            (Some(_), None) => false,
        }
    }
}

fn validate_path(path: &str) -> Result<()> {
    if !path.starts_with('/') || path.contains('?') || path.contains('#') {
        return Err(MaxError::Configuration(
            "webhook path must be an absolute path without a query or fragment".into(),
        ));
    }
    Ok(())
}

/// Axum adapter for [`WebhookService`].
#[cfg(feature = "webhook-axum")]
pub mod axum_adapter {
    use axum::{
        Router,
        body::Bytes,
        extract::{DefaultBodyLimit, State},
        http::{HeaderMap, StatusCode},
        routing::post,
    };

    use super::{Result, SECRET_HEADER, WebhookService, validate_path};

    /// Builds a ready-to-merge Axum router for one webhook path.
    pub fn router(service: WebhookService, path: &str) -> Result<Router> {
        validate_path(path)?;
        let body_limit = service.body_limit();
        Ok(Router::new()
            .route(path, post(handle))
            .layer(DefaultBodyLimit::max(body_limit))
            .with_state(service))
    }

    async fn handle(
        State(service): State<WebhookService>,
        headers: HeaderMap,
        body: Bytes,
    ) -> StatusCode {
        let secret = headers
            .get(SECRET_HEADER)
            .and_then(|value| value.to_str().ok());
        StatusCode::from_u16(service.handle(secret, &body).await.status_code())
            .expect("webhook outcomes always contain valid HTTP status codes")
    }
}

/// Actix Web adapter for [`WebhookService`].
#[cfg(feature = "webhook-actix")]
pub mod actix_adapter {
    use actix_web::{
        HttpRequest, HttpResponse, Scope,
        web::{self, Bytes},
    };

    use super::{Result, SECRET_HEADER, WebhookService, validate_path};

    /// Builds a ready-to-register Actix scope for one webhook path.
    pub fn scope(service: WebhookService, path: &str) -> Result<Scope> {
        validate_path(path)?;
        let body_limit = service.body_limit();
        Ok(web::scope(path)
            .app_data(web::Data::new(service))
            .app_data(web::PayloadConfig::new(body_limit))
            .route("", web::post().to(handle)))
    }

    async fn handle(
        service: web::Data<WebhookService>,
        request: HttpRequest,
        body: Bytes,
    ) -> HttpResponse {
        let secret = request
            .headers()
            .get(SECRET_HEADER)
            .and_then(|value| value.to_str().ok());
        let status = service.handle(secret, &body).await.status_code();
        HttpResponse::build(
            actix_web::http::StatusCode::from_u16(status)
                .expect("webhook outcomes always contain valid HTTP status codes"),
        )
        .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{WebhookOutcome, WebhookService};
    use crate::{Bot, Dispatcher};

    fn test_service() -> WebhookService {
        WebhookService::new(Dispatcher::new(Bot::new("token").unwrap()))
    }

    #[tokio::test]
    async fn rejects_bad_secret_before_json_parsing() {
        let service = test_service().secret("correct").unwrap();
        assert_eq!(
            service.handle(Some("wrong"), b"not json").await,
            WebhookOutcome::Unauthorized
        );
    }

    #[tokio::test]
    async fn rejects_oversized_and_non_object_payloads() {
        let service = test_service().max_body_size(2).unwrap();
        assert_eq!(service.handle(None, b"{}").await, WebhookOutcome::Accepted);
        assert_eq!(
            service.handle(None, b"{  }").await,
            WebhookOutcome::PayloadTooLarge
        );
        let service = test_service();
        assert_eq!(
            service.handle(None, b"[]").await,
            WebhookOutcome::BadRequest
        );
    }
}
