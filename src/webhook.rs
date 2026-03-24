//! Axum-based Webhook server — production-grade alternative to long polling.
//!
//! Enabled with `features = ["webhook"]`.
//!
//! ## How it works
//!
//! 1. Your bot registers a webhook via [`Bot::subscribe`].
//! 2. Max sends `POST /` (or any path you choose) with a single [`Update`] JSON body
//!    and an optional `X-Max-Bot-Api-Secret` header.
//! 3. [`WebhookServer`] verifies the secret, parses the update and passes it to
//!    [`Dispatcher`].
//!
//! ## Requirements from Max API
//!
//! * Endpoint must be reachable over **HTTPS on port 443**.
//! * No self-signed certificates.
//! * Must return **HTTP 200** within 30 seconds.
//!
//! ## Example
//!
//! ```no_run
//! use maxoxide::{Bot, Dispatcher, Context};
//! use maxoxide::types::{Update, SubscribeBody};
//! use maxoxide::webhook::WebhookServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let bot = Bot::from_env();
//!     let mut dp = Dispatcher::new(bot.clone());
//!
//!     dp.on_message(|ctx: Context| async move {
//!         if let Update::MessageCreated { message, .. } = &ctx.update {
//!             ctx.bot.send_text(message.chat_id(), message.text().unwrap_or("")).await?;
//!         }
//!         Ok(())
//!     });
//!
//!     // Register webhook with Max
//!     bot.subscribe(SubscribeBody {
//!         url: "https://your-domain.com/webhook".into(),
//!         update_types: None,
//!         version: None,
//!         secret: Some("my_secret_123".into()),
//!     })
//!     .await
//!     .unwrap();
//!
//!     // Start the server (listens on 0.0.0.0:443 or behind a TLS-terminating proxy)
//!     WebhookServer::new(dp)
//!         .secret("my_secret_123")
//!         .path("/webhook")
//!         .serve("0.0.0.0:8443")
//!         .await;
//! }
//! ```

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
};
use bytes::Bytes;
use tracing::{error, info, warn};

use crate::{dispatcher::Dispatcher, types::Update};

// ────────────────────────────────────────────────
// WebhookServer
// ────────────────────────────────────────────────

/// An axum-based HTTPS webhook receiver for the Max Bot API.
pub struct WebhookServer {
    dispatcher: Arc<Dispatcher>,
    secret: Option<String>,
    path: String,
}

impl WebhookServer {
    /// Create a new webhook server backed by the given dispatcher.
    pub fn new(dispatcher: Dispatcher) -> Self {
        Self {
            dispatcher: Arc::new(dispatcher),
            secret: None,
            path: "/".into(),
        }
    }

    /// Set the shared secret used to verify `X-Max-Bot-Api-Secret` headers.
    /// Strongly recommended — rejects any request that doesn't match.
    pub fn secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Set the URL path to listen on (default: `/`).
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Start listening on the given address (e.g. `"0.0.0.0:8443"`).
    ///
    /// This function runs forever (or until the process exits).
    ///
    /// In production, put a TLS-terminating reverse proxy (nginx, Caddy, …) in
    /// front of this and expose it on port 443 as required by the Max API.
    pub async fn serve(self, addr: impl Into<String>) {
        let addr: SocketAddr = addr
            .into()
            .parse()
            .expect("Invalid socket address for webhook server");

        let state = Arc::new(WebhookState {
            dispatcher: self.dispatcher,
            secret: self.secret,
        });

        let app = Router::new()
            .route(&self.path, post(handle_update))
            .with_state(state);

        info!("Webhook server listening on {addr}");
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

// ────────────────────────────────────────────────
// Internal state + handler
// ────────────────────────────────────────────────

struct WebhookState {
    dispatcher: Arc<Dispatcher>,
    secret: Option<String>,
}

async fn handle_update(
    State(state): State<Arc<WebhookState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // 1. Verify the optional secret header.
    if let Some(expected) = &state.secret {
        let provided = headers
            .get("x-max-bot-api-secret")
            .and_then(|v| v.to_str().ok());

        match provided {
            Some(val) if val == expected => {}
            Some(val) => {
                warn!("Webhook secret mismatch (got '{val}')");
                return StatusCode::UNAUTHORIZED;
            }
            None => {
                warn!("Missing X-Max-Bot-Api-Secret header");
                return StatusCode::UNAUTHORIZED;
            }
        }
    }

    // 2. Parse the single Update object from the request body.
    let update: Update = match serde_json::from_slice(&body) {
        Ok(u) => u,
        Err(e) => {
            error!("Failed to parse webhook update: {e}");
            // Return 200 so Max doesn't retry a malformed payload forever.
            return StatusCode::OK;
        }
    };

    // 3. Dispatch — must not block longer than 30 s (Max's timeout).
    state.dispatcher.dispatch(update).await;

    StatusCode::OK
}
