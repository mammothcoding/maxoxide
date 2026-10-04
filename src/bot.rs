use reqwest::{Certificate, Client, Method, Proxy};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use url::Url;

use crate::errors::{ApiError, MaxError, Result, ValidationError};
use crate::rate_limit::{MessageOperation, RateLimitConfig, RateLimitKey, RateLimiters};
use crate::types::*;

/// Default MAX Bot API endpoint.
pub const DEFAULT_BASE_URL: &str = "https://platform-api2.max.ru";
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 30;
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
const DEFAULT_UPLOAD_TIMEOUT_SECS: u64 = 30 * 60;
const RUSSIAN_TRUSTED_ROOT_CA_PEM: &[u8] = include_bytes!("certs/russian_trusted_root_ca.pem");

fn embedded_russian_trusted_root_ca() -> std::result::Result<Vec<Certificate>, reqwest::Error> {
    Certificate::from_pem_bundle(RUSSIAN_TRUSTED_ROOT_CA_PEM)
}

/// Extension methods for building custom MAX-compatible `reqwest` clients.
///
/// Use this when you need custom `reqwest::ClientBuilder` settings such as
/// `timeout(...)` or `no_proxy()` together with the Russian Trusted Root CA
/// required by the current `platform-api2.max.ru` TLS chain.
pub trait RussianTlsExt {
    /// Merge the embedded Russian Trusted Root CA into this builder.
    ///
    /// This keeps certificate verification enabled and preserves all settings
    /// already applied to the builder.
    fn russian_tls(self) -> std::result::Result<reqwest::ClientBuilder, reqwest::Error>;
}

impl RussianTlsExt for reqwest::ClientBuilder {
    fn russian_tls(self) -> std::result::Result<reqwest::ClientBuilder, reqwest::Error> {
        Ok(self.tls_certs_merge(embedded_russian_trusted_root_ca()?))
    }
}

fn parse_success_payload<T: DeserializeOwned>(
    text: &str,
) -> std::result::Result<T, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(text)?;

    match serde_json::from_value::<T>(value.clone()) {
        Ok(parsed) => Ok(parsed),
        Err(original_error) => {
            let nested_message = value.get("message").cloned();

            match nested_message {
                Some(message) => match serde_json::from_value::<T>(message) {
                    Ok(parsed) => Ok(parsed),
                    Err(_) => Err(original_error),
                },
                None => Err(original_error),
            }
        }
    }
}

/// Retry policy for temporary MAX API and transport failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Total number of attempts, including the first request.
    pub max_attempts: u32,
    /// Delay before the first retry.
    pub initial_delay: Duration,
    /// Maximum exponential-backoff delay.
    pub max_delay: Duration,
}

impl RetryPolicy {
    /// Disables automatic retries.
    pub const fn disabled() -> Self {
        Self {
            max_attempts: 1,
            initial_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
        }
    }

    fn delay(self, retry_index: u32) -> Duration {
        let multiplier = 1_u32.checked_shl(retry_index.min(31)).unwrap_or(u32::MAX);
        self.initial_delay
            .saturating_mul(multiplier)
            .min(self.max_delay)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// Builder for a production MAX Bot API client.
pub struct BotBuilder {
    token: String,
    base_url: String,
    request_timeout: Duration,
    connect_timeout: Duration,
    upload_timeout: Duration,
    proxy: Option<Proxy>,
    proxy_disabled: bool,
    client: Option<Client>,
    rate_limits: RateLimitConfig,
    retry_policy: RetryPolicy,
}

impl BotBuilder {
    /// Creates a builder with secure TLS and documented MAX rate limits.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            request_timeout: Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS),
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS),
            upload_timeout: Duration::from_secs(DEFAULT_UPLOAD_TIMEOUT_SECS),
            proxy: None,
            proxy_disabled: false,
            client: None,
            rate_limits: RateLimitConfig::default(),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Overrides the MAX API base URL, for example for a test server or gateway.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the timeout for normal Bot API requests.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets the TCP connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the timeout for each upload request or resumable chunk.
    pub fn upload_timeout(mut self, timeout: Duration) -> Self {
        self.upload_timeout = timeout;
        self
    }

    /// Routes all requests through a configured reqwest proxy.
    ///
    /// Use the re-exported [`crate::reqwest::Proxy`] to configure credentials,
    /// custom authorization, or destination exclusions before passing it here.
    pub fn proxy(mut self, proxy: Proxy) -> Self {
        self.proxy = Some(proxy);
        self.proxy_disabled = false;
        self
    }

    /// Configures an HTTP, HTTPS, or feature-gated SOCKS proxy URL.
    ///
    /// For credentials or destination exclusions, construct a
    /// [`crate::reqwest::Proxy`] and use [`Self::proxy`] instead.
    pub fn proxy_url(mut self, proxy_url: &str) -> Result<Self> {
        self.proxy = Some(Proxy::all(proxy_url)?);
        self.proxy_disabled = false;
        Ok(self)
    }

    /// Disables automatic system proxies and any previously configured proxy.
    ///
    /// This applies to every request made by the bot client, including uploads.
    /// A later call to [`Self::proxy`] or [`Self::proxy_url`] replaces this setting.
    ///
    /// ```
    /// # fn main() -> maxoxide::Result<()> {
    /// let bot = maxoxide::Bot::builder("token")
    ///     .no_proxy()
    ///     .build()?;
    /// # let _ = bot;
    /// # Ok(())
    /// # }
    /// ```
    pub fn no_proxy(mut self) -> Self {
        self.proxy = None;
        self.proxy_disabled = true;
        self
    }

    /// Uses a prebuilt HTTP client as-is.
    ///
    /// The custom client is responsible for TLS roots, proxies, and transport
    /// timeouts. Request-level upload timeouts are still applied by maxoxide.
    pub fn http_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Configures global and recipient-specific limits.
    pub fn rate_limits(mut self, rate_limits: RateLimitConfig) -> Self {
        self.rate_limits = rate_limits;
        self
    }

    /// Configures retries for temporary failures.
    pub fn retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Builds the bot client.
    pub fn build(self) -> Result<Bot> {
        if self.token.trim().is_empty() {
            return Err(MaxError::Configuration("bot token is empty".into()));
        }
        reqwest::header::HeaderValue::from_str(&self.token).map_err(|error| {
            MaxError::Configuration(format!(
                "bot token cannot be used as an HTTP header: {error}"
            ))
        })?;
        if self.request_timeout.is_zero()
            || self.connect_timeout.is_zero()
            || self.upload_timeout.is_zero()
        {
            return Err(MaxError::Configuration(
                "request, connect, and upload timeouts must be greater than zero".into(),
            ));
        }
        if self.retry_policy.max_attempts == 0 {
            return Err(MaxError::Configuration(
                "retry policy must allow at least one attempt".into(),
            ));
        }

        let mut base_url = Url::parse(&self.base_url)
            .map_err(|error| MaxError::Configuration(format!("invalid base URL: {error}")))?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(MaxError::Configuration(
                "base URL must use http or https".into(),
            ));
        }
        if base_url.scheme() == "http" && !base_url.host_str().is_some_and(is_loopback_host) {
            return Err(MaxError::Configuration(
                "base URL must use HTTPS unless it targets localhost".into(),
            ));
        }
        if !base_url.username().is_empty()
            || base_url.password().is_some()
            || base_url.query().is_some()
            || base_url.fragment().is_some()
        {
            return Err(MaxError::Configuration(
                "base URL must not contain credentials, a query, or a fragment".into(),
            ));
        }
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }

        if self.client.is_some() && (self.proxy.is_some() || self.proxy_disabled) {
            return Err(MaxError::Configuration(
                "proxy configuration cannot be combined with a prebuilt HTTP client".into(),
            ));
        }

        let client = match self.client {
            Some(client) => client,
            None => {
                let mut builder = Client::builder()
                    .timeout(self.request_timeout)
                    .connect_timeout(self.connect_timeout)
                    .russian_tls()?;
                if self.proxy_disabled {
                    builder = builder.no_proxy();
                } else if let Some(proxy) = self.proxy {
                    builder = builder.proxy(proxy);
                }
                builder.build()?
            }
        };

        Ok(Bot {
            inner: Arc::new(BotInner {
                token: self.token,
                client,
                base_url,
                upload_timeout: self.upload_timeout,
                rate_limiters: RateLimiters::new(self.rate_limits),
                retry_policy: self.retry_policy,
            }),
        })
    }
}

/// The main entry point for the Max Bot API.
///
/// Holds an HTTP client and your bot token. All API methods are async and
/// return `Result<T, MaxError>`.
///
/// # Example
/// ```no_run
/// use maxoxide::Bot;
///
/// #[tokio::main]
/// async fn main() -> maxoxide::Result<()> {
///     let bot = Bot::from_env()?;
///     let me = bot.get_me().await?;
///     println!("Running as @{}", me.username.unwrap_or_default());
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Bot {
    inner: Arc<BotInner>,
}

struct BotInner {
    token: String,
    client: Client,
    base_url: Url,
    upload_timeout: Duration,
    rate_limiters: RateLimiters,
    retry_policy: RetryPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageRecipientQuery {
    ChatId(i64),
    UserId(i64),
}

impl MessageRecipientQuery {
    fn append_to(self, params: &mut Vec<(&'static str, String)>) {
        match self {
            Self::ChatId(chat_id) => params.push(("chat_id", chat_id.to_string())),
            Self::UserId(user_id) => params.push(("user_id", user_id.to_string())),
        }
    }

    fn into_query(self) -> Vec<(&'static str, String)> {
        let mut params = Vec::with_capacity(1);
        self.append_to(&mut params);
        params
    }

    fn rate_limit_key(self) -> RateLimitKey {
        match self {
            Self::ChatId(chat_id) => RateLimitKey::Chat(chat_id),
            Self::UserId(user_id) => RateLimitKey::User(user_id),
        }
    }
}

fn append_send_options(params: &mut Vec<(&'static str, String)>, options: SendMessageOptions) {
    if let Some(disable_link_preview) = options.disable_link_preview {
        params.push(("disable_link_preview", disable_link_preview.to_string()));
    }
}

fn append_update_query(
    params: &mut Vec<(&'static str, String)>,
    marker: Option<i64>,
    timeout: Option<u32>,
    limit: Option<u32>,
) {
    if let Some(m) = marker {
        params.push(("marker", m.to_string()));
    }
    if let Some(t) = timeout {
        params.push(("timeout", t.to_string()));
    }
    if let Some(l) = limit {
        params.push(("limit", l.to_string()));
    }
}

fn comma_join_strings(values: impl IntoIterator<Item = impl Into<String>>) -> String {
    values
        .into_iter()
        .map(Into::into)
        .collect::<Vec<String>>()
        .join(",")
}

fn comma_join_i64(values: impl IntoIterator<Item = i64>) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

fn split_message_text(text: &str, max_chars: usize) -> Vec<String> {
    debug_assert!(max_chars > 0);
    let mut chars = text.chars();
    let mut parts = Vec::new();

    loop {
        let part = chars.by_ref().take(max_chars).collect::<String>();
        if part.is_empty() {
            break;
        }
        parts.push(part);
    }

    if parts.is_empty() {
        parts.push(String::new());
    }
    parts
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();

    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'@' => {
                encoded.push(*byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

fn max_ru_link_last_segment(chat_link: &str) -> Option<&str> {
    let without_fragment = chat_link.split('#').next().unwrap_or(chat_link);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment)
        .trim_end_matches('/');
    let path = without_query
        .strip_prefix("https://max.ru/")
        .or_else(|| without_query.strip_prefix("http://max.ru/"))
        .or_else(|| without_query.strip_prefix("https://www.max.ru/"))
        .or_else(|| without_query.strip_prefix("http://www.max.ru/"))
        .or_else(|| without_query.strip_prefix("max.ru/"))
        .or_else(|| without_query.strip_prefix("www.max.ru/"))?;

    path.rsplit('/').find(|segment| !segment.is_empty())
}

fn push_chat_link_candidate(candidates: &mut Vec<String>, value: impl Into<String>) {
    let value = value.into();
    if !value.is_empty() && !candidates.iter().any(|candidate| candidate == &value) {
        candidates.push(value);
    }
}

fn push_chat_link_name_variants(candidates: &mut Vec<String>, value: &str) {
    let value = value.trim().trim_matches('/');
    if value.is_empty() {
        return;
    }

    push_chat_link_candidate(candidates, value);
    if let Some(without_at) = value.strip_prefix('@') {
        push_chat_link_candidate(candidates, without_at);
    } else if !value.contains("://") && !value.contains('/') {
        push_chat_link_candidate(candidates, format!("@{value}"));
    }
}

fn chat_link_candidates(chat_link: &str) -> Vec<String> {
    let trimmed = chat_link.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let direct = trimmed
        .split('#')
        .next()
        .unwrap_or(trimmed)
        .split('?')
        .next()
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    let mut candidates = Vec::new();
    push_chat_link_candidate(&mut candidates, direct);

    if let Some(username) = max_ru_link_last_segment(trimmed) {
        push_chat_link_name_variants(&mut candidates, username);
    } else {
        push_chat_link_name_variants(&mut candidates, direct);
    }

    candidates
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

impl Bot {
    /// Creates a new bot with secure defaults and documented MAX rate limits.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        Self::builder(token).build()
    }

    /// Starts configuring a bot client.
    pub fn builder(token: impl Into<String>) -> BotBuilder {
        BotBuilder::new(token)
    }

    /// Creates a bot with a custom HTTP client.
    ///
    /// The provided client is used as-is and must contain any required custom
    /// trust roots or proxy configuration.
    pub fn with_client(token: impl Into<String>, client: Client) -> Result<Self> {
        Self::builder(token).http_client(client).build()
    }

    /// Creates a bot using `MAX_BOT_TOKEN` from the environment.
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("MAX_BOT_TOKEN").map_err(|error| {
            MaxError::Configuration(format!("MAX_BOT_TOKEN is not available: {error}"))
        })?;
        Self::new(token)
    }

    /// Returns the underlying HTTP client.
    pub fn client(&self) -> &Client {
        &self.inner.client
    }

    /// Returns the bot token.
    pub fn token(&self) -> &str {
        &self.inner.token
    }

    /// Returns the configured Bot API base URL.
    pub fn base_url(&self) -> &Url {
        &self.inner.base_url
    }

    /// Executes an arbitrary relative MAX Bot API request.
    ///
    /// This escape hatch uses the configured authorization, rate limiter,
    /// structured errors, and retry policy. Prefer typed methods when one is
    /// available.
    pub async fn execute<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        self.request(method, path, query, body).await
    }

    // ────────────────────────────────────────────────
    // Internal helpers
    // ────────────────────────────────────────────────

    fn url(&self, path: &str) -> String {
        self.inner
            .base_url
            .join(path.trim_start_matches('/'))
            .expect("validated base URL accepts relative API paths")
            .to_string()
    }

    fn auth(&self) -> String {
        self.inner.token.clone()
    }

    pub(crate) async fn api_client(&self) -> Result<&Client> {
        Ok(&self.inner.client)
    }

    pub(crate) fn upload_timeout(&self) -> Duration {
        self.inner.upload_timeout
    }

    pub(crate) async fn acquire_message_limit(
        &self,
        operation: MessageOperation,
        key: RateLimitKey,
    ) {
        self.inner
            .rate_limiters
            .acquire_recipient(operation, key)
            .await;
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with_query::<T, [(&str, &str); 0]>(path, []).await
    }

    async fn get_with_query<T, Q>(&self, path: &str, query: Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: serde::Serialize,
    {
        self.request(Method::GET, path, &query, None).await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.post_with_query::<T, B, [(&str, &str); 0]>(path, body, [])
            .await
    }

    async fn post_with_query<T, B, Q>(&self, path: &str, body: &B, query: Q) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
        Q: serde::Serialize,
    {
        self.request(
            Method::POST,
            path,
            &query,
            Some(serde_json::to_value(body)?),
        )
        .await
    }

    async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request::<T, _>(
            Method::PUT,
            path,
            &[] as &[(&str, &str)],
            Some(serde_json::to_value(body)?),
        )
        .await
    }

    async fn patch<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request::<T, _>(
            Method::PATCH,
            path,
            &[] as &[(&str, &str)],
            Some(serde_json::to_value(body)?),
        )
        .await
    }

    async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.delete_with_query::<T, [(&str, &str); 0]>(path, [])
            .await
    }

    async fn delete_with_query<T, Q>(&self, path: &str, query: Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: serde::Serialize,
    {
        self.request(Method::DELETE, path, &query, None).await
    }

    async fn request<T, Q>(
        &self,
        method: Method,
        path: &str,
        query: &Q,
        body: Option<serde_json::Value>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        Q: serde::Serialize + ?Sized,
    {
        if path.contains("://") || path.starts_with("//") {
            return Err(MaxError::Configuration(
                "Bot API paths must be relative to the configured base URL".into(),
            ));
        }

        let attempts = self.inner.retry_policy.max_attempts.max(1);
        for attempt in 0..attempts {
            self.inner.rate_limiters.acquire_global().await;
            debug!(method = %method, path, attempt = attempt + 1, "MAX API request");

            let mut request = self
                .api_client()
                .await?
                .request(method.clone(), self.url(path))
                .header("Authorization", self.auth())
                .query(query);
            if let Some(body) = &body {
                request = request.json(body);
            }

            let result = match request.send().await {
                Ok(response) => Self::parse_response(response).await,
                Err(error) => Err(MaxError::Http(error)),
            };

            match result {
                Ok(value) => return Ok(value),
                Err(error)
                    if attempt + 1 < attempts && Self::is_retryable(&method, path, &error) =>
                {
                    let delay = error
                        .api_error()
                        .and_then(|error| error.retry_after)
                        .unwrap_or_else(|| self.inner.retry_policy.delay(attempt))
                        .min(self.inner.retry_policy.max_delay);
                    debug!(method = %method, path, ?delay, "retrying temporary MAX API failure");
                    tokio::time::sleep(delay).await;
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("request loop always returns on its final attempt")
    }

    fn is_retryable(method: &Method, path: &str, error: &MaxError) -> bool {
        match error {
            MaxError::Api(error) if error.is_attachment_not_ready() => {
                method == Method::POST && path == "/messages"
            }
            MaxError::Api(error) if error.is_rate_limited() => true,
            MaxError::Api(error) if error.is_server_error() => method != Method::POST,
            MaxError::Http(_) => method != Method::POST,
            _ => false,
        }
    }

    async fn parse_response<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs);
        let bytes = resp.bytes().await?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        debug!(status = status.as_u16(), "MAX API response");

        if status.is_success() {
            parse_success_payload(&text).map_err(MaxError::Json)
        } else {
            let value = serde_json::from_str::<serde_json::Value>(&text).ok();
            let code = value
                .as_ref()
                .and_then(|value| value.get("code"))
                .and_then(serde_json::Value::as_str)
                .map(String::from);
            let message = value
                .as_ref()
                .and_then(|value| {
                    value
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| value.get("error").and_then(serde_json::Value::as_str))
                        .or_else(|| {
                            value
                                .get("error")
                                .and_then(|error| error.get("message"))
                                .and_then(serde_json::Value::as_str)
                        })
                })
                .filter(|message| !message.is_empty())
                .map(String::from)
                .unwrap_or_else(|| {
                    status
                        .canonical_reason()
                        .unwrap_or("MAX API request failed")
                        .to_string()
                });

            Err(ApiError::new(status.as_u16(), code, message)
                .with_raw_response(text)
                .with_retry_after(retry_after)
                .into())
        }
    }

    // ────────────────────────────────────────────────
    // Bots
    // ────────────────────────────────────────────────

    /// GET /me — Returns info about the current bot.
    pub async fn get_me(&self) -> Result<BotInfo> {
        self.get("/me").await
    }

    // ────────────────────────────────────────────────
    // Messages
    // ────────────────────────────────────────────────

    async fn send_message_to_recipient(
        &self,
        recipient: MessageRecipientQuery,
        body: NewMessageBody,
    ) -> Result<Message> {
        self.send_message_to_recipient_with_options(recipient, body, SendMessageOptions::default())
            .await
    }

    async fn send_message_to_recipient_with_options(
        &self,
        recipient: MessageRecipientQuery,
        body: NewMessageBody,
        options: SendMessageOptions,
    ) -> Result<Message> {
        body.validate()?;
        self.acquire_message_limit(MessageOperation::Send, recipient.rate_limit_key())
            .await;
        let mut params = recipient.into_query();
        append_send_options(&mut params, options);
        self.post_with_query("/messages", &body, &params).await
    }

    /// POST /messages — Send a message to a chat/dialog by `chat_id`.
    ///
    /// `chat_id` identifies a concrete dialog, group, or channel.
    /// It is not the same as a user's global MAX `user_id`.
    pub async fn send_message_to_chat(
        &self,
        chat_id: i64,
        body: NewMessageBody,
    ) -> Result<Message> {
        self.send_message_to_recipient(MessageRecipientQuery::ChatId(chat_id), body)
            .await
    }

    /// POST /messages — Send a message to a chat/dialog by `chat_id` with query options.
    pub async fn send_message_to_chat_with_options(
        &self,
        chat_id: i64,
        body: NewMessageBody,
        options: SendMessageOptions,
    ) -> Result<Message> {
        self.send_message_to_recipient_with_options(
            MessageRecipientQuery::ChatId(chat_id),
            body,
            options,
        )
        .await
    }

    /// POST /messages — Send a message to a user by global MAX `user_id`.
    ///
    /// Use this when you know the user's stable MAX identifier, but do not want
    /// to address a specific dialog `chat_id`.
    pub async fn send_message_to_user(
        &self,
        user_id: i64,
        body: NewMessageBody,
    ) -> Result<Message> {
        self.send_message_to_recipient(MessageRecipientQuery::UserId(user_id), body)
            .await
    }

    /// POST /messages — Send a message to a user by global MAX `user_id` with query options.
    pub async fn send_message_to_user_with_options(
        &self,
        user_id: i64,
        body: NewMessageBody,
        options: SendMessageOptions,
    ) -> Result<Message> {
        self.send_message_to_recipient_with_options(
            MessageRecipientQuery::UserId(user_id),
            body,
            options,
        )
        .await
    }

    /// Convenience: send a plain-text message to a chat/dialog by `chat_id`.
    pub async fn send_text_to_chat(
        &self,
        chat_id: i64,
        text: impl Into<String>,
    ) -> Result<Message> {
        self.send_message_to_chat(chat_id, NewMessageBody::text(text))
            .await
    }

    /// Convenience: send a plain-text message to a user by global MAX `user_id`.
    pub async fn send_text_to_user(
        &self,
        user_id: i64,
        text: impl Into<String>,
    ) -> Result<Message> {
        self.send_message_to_user(user_id, NewMessageBody::text(text))
            .await
    }

    /// Convenience: send a Markdown-formatted message to a chat/dialog by `chat_id`.
    pub async fn send_markdown_to_chat(
        &self,
        chat_id: i64,
        text: impl Into<String>,
    ) -> Result<Message> {
        self.send_message_to_chat(
            chat_id,
            NewMessageBody::text(text).with_format(MessageFormat::Markdown),
        )
        .await
    }

    /// Convenience: send a Markdown-formatted message to a user by global MAX `user_id`.
    pub async fn send_markdown_to_user(
        &self,
        user_id: i64,
        text: impl Into<String>,
    ) -> Result<Message> {
        self.send_message_to_user(
            user_id,
            NewMessageBody::text(text).with_format(MessageFormat::Markdown),
        )
        .await
    }

    /// Convenience: send an HTML-formatted message to a chat/dialog.
    pub async fn send_html_to_chat(
        &self,
        chat_id: i64,
        text: impl Into<String>,
    ) -> Result<Message> {
        self.send_message_to_chat(
            chat_id,
            NewMessageBody::text(text).with_format(MessageFormat::Html),
        )
        .await
    }

    /// Convenience: send an HTML-formatted message to a user.
    pub async fn send_html_to_user(
        &self,
        user_id: i64,
        text: impl Into<String>,
    ) -> Result<Message> {
        self.send_message_to_user(
            user_id,
            NewMessageBody::text(text).with_format(MessageFormat::Html),
        )
        .await
    }

    /// Splits text at MAX's 4000-character boundary and sends every part.
    pub async fn send_long_text_to_chat(
        &self,
        chat_id: i64,
        text: impl AsRef<str>,
    ) -> Result<Vec<Message>> {
        self.send_long_text(MessageRecipientQuery::ChatId(chat_id), text.as_ref(), None)
            .await
    }

    /// Splits formatted text at MAX's 4000-character boundary and sends every part.
    pub async fn send_long_formatted_text_to_chat(
        &self,
        chat_id: i64,
        text: impl AsRef<str>,
        format: MessageFormat,
    ) -> Result<Vec<Message>> {
        self.send_long_text(
            MessageRecipientQuery::ChatId(chat_id),
            text.as_ref(),
            Some(format),
        )
        .await
    }

    async fn send_long_text(
        &self,
        recipient: MessageRecipientQuery,
        text: &str,
        format: Option<MessageFormat>,
    ) -> Result<Vec<Message>> {
        let mut messages = Vec::new();
        for part in split_message_text(text, 4000) {
            let mut body = NewMessageBody::text(part);
            body.format = format.clone();
            messages.push(self.send_message_to_recipient(recipient, body).await?);
        }
        Ok(messages)
    }

    /// PUT /messages — Edit an existing message.
    pub async fn edit_message(
        &self,
        message_id: &str,
        body: NewMessageBody,
    ) -> Result<SimpleResult> {
        body.validate()?;
        self.acquire_message_limit(
            MessageOperation::Edit,
            RateLimitKey::Custom(message_id.to_owned()),
        )
        .await;
        self.put_with_query("/messages", &body, [("message_id", message_id)])
            .await
    }

    /// DELETE /messages — Delete a message.
    pub async fn delete_message(&self, message_id: &str) -> Result<SimpleResult> {
        self.acquire_message_limit(
            MessageOperation::Delete,
            RateLimitKey::Custom(message_id.to_owned()),
        )
        .await;
        self.delete_with_query("/messages", [("message_id", message_id)])
            .await
    }

    /// GET /messages/{messageId} — Get a single message by ID.
    pub async fn get_message(&self, message_id: &str) -> Result<Message> {
        self.get(&format!(
            "/messages/{}",
            percent_encode_path_segment(message_id)
        ))
        .await
    }

    /// GET /messages — Get messages from a chat.
    pub async fn get_messages(
        &self,
        chat_id: i64,
        count: Option<u32>,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<MessageList> {
        let mut params: Vec<(&str, String)> = vec![("chat_id", chat_id.to_string())];
        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }
        if let Some(f) = from {
            params.push(("from", f.to_string()));
        }
        if let Some(t) = to {
            params.push(("to", t.to_string()));
        }
        self.get_with_query("/messages", &params).await
    }

    /// GET /messages — Get one or more messages by IDs.
    pub async fn get_messages_by_ids(
        &self,
        message_ids: impl IntoIterator<Item = impl Into<String>>,
        count: Option<u32>,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<MessageList> {
        let mut params: Vec<(&str, String)> =
            vec![("message_ids", comma_join_strings(message_ids))];
        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }
        if let Some(f) = from {
            params.push(("from", f.to_string()));
        }
        if let Some(t) = to {
            params.push(("to", t.to_string()));
        }
        self.get_with_query("/messages", &params).await
    }

    /// GET /videos/{videoToken} — Get video metadata and playback URLs.
    pub async fn get_video(&self, video_token: &str) -> Result<VideoInfo> {
        self.get(&format!("/videos/{video_token}")).await
    }

    /// POST /answers — Respond to an inline button callback.
    pub async fn answer_callback(&self, body: AnswerCallbackBody) -> Result<SimpleResult> {
        self.answer_callback_with_options(body, AnswerCallbackOptions::default())
            .await
    }

    /// Отвечает на callback с дополнительными query-параметрами.
    pub async fn answer_callback_with_options(
        &self,
        body: AnswerCallbackBody,
        options: AnswerCallbackOptions,
    ) -> Result<SimpleResult> {
        #[derive(serde::Serialize)]
        struct AnswerBody {
            #[serde(skip_serializing_if = "Option::is_none")]
            message: Option<NewMessageBody>,
            #[serde(skip_serializing_if = "Option::is_none")]
            notification: Option<String>,
        }

        if let Some(message) = &body.message {
            message.validate()?;
        }
        self.acquire_message_limit(
            MessageOperation::Callback,
            RateLimitKey::Custom(body.callback_id.clone()),
        )
        .await;
        let mut query = vec![("callback_id", body.callback_id)];
        if let Some(disable_link_preview) = options.disable_link_preview {
            query.push(("disable_link_preview", disable_link_preview.to_string()));
        }
        self.post_with_query(
            "/answers",
            &AnswerBody {
                message: body.message,
                notification: body.notification,
            },
            &query,
        )
        .await
    }

    // ────────────────────────────────────────────────
    // Comments
    // ────────────────────────────────────────────────

    /// GET /messages/{messageId}/comments — Get comments for a channel post.
    ///
    /// Бот должен быть администратором канала с правом `read_all_messages`.
    pub async fn get_comments(
        &self,
        message_id: &str,
        options: GetCommentsOptions,
    ) -> Result<CommentList> {
        if options
            .count
            .is_some_and(|count| !(1..=100).contains(&count))
        {
            return Err(ValidationError::new("count", "must be between 1 and 100").into());
        }
        if options.before.is_some_and(|timestamp| timestamp < 0)
            || options.after.is_some_and(|timestamp| timestamp < 0)
        {
            return Err(ValidationError::new("timestamp", "must not be negative").into());
        }

        let mut query = Vec::new();
        if let Some(comment_ids) = options.comment_ids {
            if comment_ids.iter().any(String::is_empty) {
                return Err(ValidationError::new("comment_ids", "IDs must not be empty").into());
            }
            query.push(("comment_ids", comment_ids.join(",")));
        }
        if let Some(before) = options.before {
            query.push(("before", before.to_string()));
        }
        if let Some(after) = options.after {
            query.push(("after", after.to_string()));
        }
        if let Some(count) = options.count {
            query.push(("count", count.to_string()));
        }

        self.get_with_query(
            &format!(
                "/messages/{}/comments",
                percent_encode_path_segment(message_id)
            ),
            &query,
        )
        .await
    }

    /// GET /messages/{messageId}/comments/{commentId} — Get one comment.
    pub async fn get_comment(&self, message_id: &str, comment_id: &str) -> Result<CommentMessage> {
        self.get(&format!(
            "/messages/{}/comments/{}",
            percent_encode_path_segment(message_id),
            percent_encode_path_segment(comment_id)
        ))
        .await
    }

    /// POST /messages/{messageId}/comments — Add a comment to a channel post.
    pub async fn create_comment(
        &self,
        message_id: &str,
        body: NewCommentBody,
        disable_link_preview: Option<bool>,
    ) -> Result<CommentMessage> {
        body.validate()?;
        self.acquire_message_limit(
            MessageOperation::Comment,
            RateLimitKey::Custom(message_id.to_owned()),
        )
        .await;
        let query = disable_link_preview
            .map(|value| vec![("disable_link_preview", value.to_string())])
            .unwrap_or_default();
        self.post_with_query(
            &format!(
                "/messages/{}/comments",
                percent_encode_path_segment(message_id)
            ),
            &body,
            &query,
        )
        .await
    }

    /// PUT /messages/{messageId}/comments — Edit a comment.
    pub async fn edit_comment(
        &self,
        message_id: &str,
        comment_id: &str,
        body: NewCommentBody,
    ) -> Result<SimpleResult> {
        body.validate()?;
        self.acquire_message_limit(
            MessageOperation::Comment,
            RateLimitKey::Custom(message_id.to_owned()),
        )
        .await;
        self.put_with_query(
            &format!(
                "/messages/{}/comments",
                percent_encode_path_segment(message_id)
            ),
            &body,
            [("comment_id", comment_id)],
        )
        .await
    }

    /// DELETE /messages/{messageId}/comments — Delete a comment.
    pub async fn delete_comment(&self, message_id: &str, comment_id: &str) -> Result<SimpleResult> {
        self.acquire_message_limit(
            MessageOperation::Comment,
            RateLimitKey::Custom(message_id.to_owned()),
        )
        .await;
        self.delete_with_query(
            &format!(
                "/messages/{}/comments",
                percent_encode_path_segment(message_id)
            ),
            [("comment_id", comment_id)],
        )
        .await
    }

    // ────────────────────────────────────────────────
    // Chats
    // ────────────────────────────────────────────────

    /// GET /chats/{chatId} — Get info about a specific chat.
    pub async fn get_chat(&self, chat_id: i64) -> Result<Chat> {
        self.get(&format!("/chats/{chat_id}")).await
    }

    /// GET /chats/{chatLink} — Get channel info by public link or username.
    ///
    /// The public MAX API documents this endpoint for channels only. You may
    /// pass a full `https://max.ru/...` URL, a channel public name, or a name
    /// with a leading `@`.
    pub async fn get_chat_by_link(&self, chat_link: &str) -> Result<Chat> {
        let candidates = chat_link_candidates(chat_link);
        if candidates.is_empty() {
            return Err(ValidationError::new("chat_link", "value is empty").into());
        }

        let tried = candidates.join(", ");
        let mut last_error = None;

        for candidate in &candidates {
            let encoded = percent_encode_path_segment(candidate);
            match self.get(&format!("/chats/{encoded}")).await {
                Ok(chat) => return Ok(chat),
                Err(err) => {
                    let should_try_next =
                        matches!(err, MaxError::Api(ref error) if error.status == 404);
                    if !should_try_next {
                        return Err(err);
                    }
                    last_error = Some(err);
                }
            }
        }

        match last_error {
            Some(MaxError::Api(error)) if error.status == 404 => Err(ApiError::new(
                404,
                error.code,
                format!("{}. Tried variants: {tried}", error.message),
            )
            .into()),
            Some(err) => Err(err),
            None => Err(ValidationError::new("chat_link", "value is empty").into()),
        }
    }

    /// PATCH /chats/{chatId} — Edit chat title/description.
    pub async fn edit_chat(&self, chat_id: i64, body: EditChatBody) -> Result<Chat> {
        self.patch(&format!("/chats/{chat_id}"), &body).await
    }

    /// DELETE /chats/{chatId} — Delete a chat.
    pub async fn delete_chat(&self, chat_id: i64) -> Result<SimpleResult> {
        self.delete(&format!("/chats/{chat_id}")).await
    }

    /// POST /chats/{chatId}/actions — Send a bot action to a group chat.
    ///
    /// The Max API expects values such as `"typing_on"`, `"sending_photo"`,
    /// `"sending_video"`, `"sending_audio"`, `"sending_file"` or `"mark_seen"`.
    ///
    /// Live MAX tests confirm that `"typing_on"` shows the typing indicator in
    /// group chats.
    pub async fn send_action(&self, chat_id: i64, action: &str) -> Result<SimpleResult> {
        #[derive(serde::Serialize)]
        struct ActionBody<'a> {
            action: &'a str,
        }
        self.post(&format!("/chats/{chat_id}/actions"), &ActionBody { action })
            .await
    }

    /// POST /chats/{chatId}/actions — Send a typed bot action to a group chat.
    pub async fn send_sender_action(
        &self,
        chat_id: i64,
        action: SenderAction,
    ) -> Result<SimpleResult> {
        self.send_action(chat_id, action.as_str()).await
    }

    /// Convenience: request a typing indicator.
    pub async fn send_typing_on(&self, chat_id: i64) -> Result<SimpleResult> {
        self.send_sender_action(chat_id, SenderAction::TypingOn)
            .await
    }

    /// Convenience: request an image upload indicator.
    pub async fn send_sending_image(&self, chat_id: i64) -> Result<SimpleResult> {
        self.send_sender_action(chat_id, SenderAction::SendingImage)
            .await
    }

    /// Convenience: request a video upload indicator.
    pub async fn send_sending_video(&self, chat_id: i64) -> Result<SimpleResult> {
        self.send_sender_action(chat_id, SenderAction::SendingVideo)
            .await
    }

    /// Convenience: request an audio upload indicator.
    pub async fn send_sending_audio(&self, chat_id: i64) -> Result<SimpleResult> {
        self.send_sender_action(chat_id, SenderAction::SendingAudio)
            .await
    }

    /// Convenience: request a file upload indicator.
    pub async fn send_sending_file(&self, chat_id: i64) -> Result<SimpleResult> {
        self.send_sender_action(chat_id, SenderAction::SendingFile)
            .await
    }

    /// Convenience: mark a group chat as seen.
    pub async fn mark_seen(&self, chat_id: i64) -> Result<SimpleResult> {
        self.send_sender_action(chat_id, SenderAction::MarkSeen)
            .await
    }

    // ────────────────────────────────────────────────
    // Pinned messages
    // ────────────────────────────────────────────────

    /// GET /chats/{chatId}/pin — Get the pinned message in a chat.
    pub async fn get_pinned_message(&self, chat_id: i64) -> Result<PinnedMessage> {
        self.get(&format!("/chats/{chat_id}/pin")).await
    }

    /// PUT /chats/{chatId}/pin — Pin a message.
    pub async fn pin_message(&self, chat_id: i64, body: PinMessageBody) -> Result<SimpleResult> {
        self.put(&format!("/chats/{chat_id}/pin"), &body).await
    }

    /// DELETE /chats/{chatId}/pin — Unpin the pinned message.
    pub async fn unpin_message(&self, chat_id: i64) -> Result<SimpleResult> {
        self.delete(&format!("/chats/{chat_id}/pin")).await
    }

    // ────────────────────────────────────────────────
    // Chat members
    // ────────────────────────────────────────────────

    /// GET /chats/{chatId}/members — Get members of a chat.
    pub async fn get_members(
        &self,
        chat_id: i64,
        count: Option<u32>,
        marker: Option<i64>,
    ) -> Result<ChatMembersList> {
        let mut params: Vec<(&str, String)> = vec![];
        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }
        if let Some(m) = marker {
            params.push(("marker", m.to_string()));
        }
        self.get_with_query(&format!("/chats/{chat_id}/members"), &params)
            .await
    }

    /// GET /chats/{chatId}/members — Get selected chat members by user IDs.
    pub async fn get_members_by_ids(
        &self,
        chat_id: i64,
        user_ids: impl IntoIterator<Item = i64>,
    ) -> Result<ChatMembersList> {
        self.get_with_query(
            &format!("/chats/{chat_id}/members"),
            [("user_ids", comma_join_i64(user_ids))],
        )
        .await
    }

    /// POST /chats/{chatId}/members — Add members to a chat.
    #[deprecated(
        note = "MAX ограничил POST /chats/{chatId}/members 9 сентября 2026 года и удалил 30 сентября 2026 года"
    )]
    pub async fn add_members(&self, chat_id: i64, user_ids: Vec<i64>) -> Result<SimpleResult> {
        self.post(
            &format!("/chats/{chat_id}/members"),
            &AddMembersBody { user_ids },
        )
        .await
    }

    /// DELETE /chats/{chatId}/members — Remove a member from a chat.
    pub async fn remove_member(&self, chat_id: i64, user_id: i64) -> Result<SimpleResult> {
        self.remove_member_with_options(chat_id, user_id, RemoveMemberOptions::default())
            .await
    }

    /// DELETE /chats/{chatId}/members — Remove a member with query options.
    pub async fn remove_member_with_options(
        &self,
        chat_id: i64,
        user_id: i64,
        options: RemoveMemberOptions,
    ) -> Result<SimpleResult> {
        let mut params = vec![("user_id", user_id.to_string())];
        if let Some(block) = options.block {
            params.push(("block", block.to_string()));
        }

        self.delete_with_query(&format!("/chats/{chat_id}/members"), &params)
            .await
    }

    /// GET /chats/{chatId}/members/admins — Get administrators of a chat.
    pub async fn get_admins(&self, chat_id: i64) -> Result<ChatMembersList> {
        self.get(&format!("/chats/{chat_id}/members/admins")).await
    }

    /// POST /chats/{chatId}/members/admins — Grant administrator rights.
    pub async fn add_admins(&self, chat_id: i64, admins: Vec<ChatAdmin>) -> Result<SimpleResult> {
        self.post(
            &format!("/chats/{chat_id}/members/admins"),
            &SetChatAdminsBody {
                admins,
                marker: None,
            },
        )
        .await
    }

    /// DELETE /chats/{chatId}/members/admins/{userId} — Revoke administrator rights.
    pub async fn remove_admin(&self, chat_id: i64, user_id: i64) -> Result<SimpleResult> {
        self.delete(&format!("/chats/{chat_id}/members/admins/{user_id}"))
            .await
    }

    /// GET /chats/{chatId}/members/me — Get the bot's membership info in a chat.
    pub async fn get_my_membership(&self, chat_id: i64) -> Result<ChatMember> {
        self.get(&format!("/chats/{chat_id}/members/me")).await
    }

    /// DELETE /chats/{chatId}/members/me — Leave a chat.
    pub async fn leave_chat(&self, chat_id: i64) -> Result<SimpleResult> {
        self.delete(&format!("/chats/{chat_id}/members/me")).await
    }

    // ────────────────────────────────────────────────
    // Subscriptions (Webhook)
    // ────────────────────────────────────────────────

    /// GET /subscriptions — List current webhook subscriptions.
    pub async fn get_subscriptions(&self) -> Result<SubscriptionList> {
        self.get("/subscriptions").await
    }

    /// POST /subscriptions — Subscribe to updates via webhook.
    pub async fn subscribe(&self, body: SubscribeBody) -> Result<SimpleResult> {
        self.post("/subscriptions", &body).await
    }

    /// DELETE /subscriptions — Unsubscribe from a webhook.
    pub async fn unsubscribe(&self, url: &str) -> Result<SimpleResult> {
        self.delete_with_query("/subscriptions", [("url", url)])
            .await
    }

    // ────────────────────────────────────────────────
    // Long Polling
    // ────────────────────────────────────────────────

    /// GET /updates — Poll for new updates once.
    ///
    /// `marker` is the offset from the previous call; pass `None` for the first call.
    /// `timeout` is the long-poll wait time in seconds (max 90).
    pub async fn get_updates(
        &self,
        marker: Option<i64>,
        timeout: Option<u32>,
        limit: Option<u32>,
    ) -> Result<UpdatesResponse> {
        let mut params: Vec<(&str, String)> = vec![];
        append_update_query(&mut params, marker, timeout, limit);
        self.get_with_query("/updates", &params).await
    }

    /// GET /updates — Poll for selected update types once.
    pub async fn get_updates_with_types(
        &self,
        marker: Option<i64>,
        timeout: Option<u32>,
        limit: Option<u32>,
        types: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<UpdatesResponse> {
        let mut params: Vec<(&str, String)> = vec![];
        append_update_query(&mut params, marker, timeout, limit);
        let types = comma_join_strings(types);
        if !types.is_empty() {
            params.push(("types", types));
        }
        self.get_with_query("/updates", &params).await
    }

    /// GET /updates — Poll for raw JSON updates once.
    pub async fn get_updates_raw(
        &self,
        marker: Option<i64>,
        timeout: Option<u32>,
        limit: Option<u32>,
    ) -> Result<RawUpdatesResponse> {
        let mut params: Vec<(&str, String)> = vec![];
        append_update_query(&mut params, marker, timeout, limit);
        self.get_with_query("/updates", &params).await
    }

    /// GET /updates — Poll raw JSON for selected update types once.
    pub async fn get_updates_raw_with_types(
        &self,
        marker: Option<i64>,
        timeout: Option<u32>,
        limit: Option<u32>,
        types: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<RawUpdatesResponse> {
        let mut params: Vec<(&str, String)> = vec![];
        append_update_query(&mut params, marker, timeout, limit);
        let types = comma_join_strings(types);
        if !types.is_empty() {
            params.push(("types", types));
        }
        self.get_with_query("/updates", &params).await
    }

    // ────────────────────────────────────────────────
    // Uploads
    // ────────────────────────────────────────────────

    /// POST /uploads — Get an upload URL for a given file type.
    pub async fn get_upload_url(&self, upload_type: UploadType) -> Result<UploadEndpoint> {
        self.post_with_query(
            "/uploads",
            &serde_json::Value::Null,
            [("type", upload_type.as_str())],
        )
        .await
    }

    // ────────────────────────────────────────────────
    // Bot commands
    // ────────────────────────────────────────────────

    /// PATCH /me/commands — Replace the list of commands shown to users.
    pub async fn set_my_commands(&self, commands: Vec<BotCommand>) -> Result<BotCommands> {
        if commands.len() > 32 {
            return Err(
                ValidationError::new("commands", "must not contain more than 32 items").into(),
            );
        }
        if commands.iter().any(|command| command.name.is_empty()) {
            return Err(ValidationError::new("commands.name", "value is empty").into());
        }
        #[derive(serde::Serialize)]
        struct CommandsBody {
            commands: Vec<BotCommand>,
        }
        self.patch("/me/commands", &CommandsBody { commands }).await
    }

    // ────────────────────────────────────────────────
    // Internal helper for PUT with query params
    // ────────────────────────────────────────────────

    async fn put_with_query<T, B, Q>(&self, path: &str, body: &B, query: Q) -> Result<T>
    where
        T: DeserializeOwned,
        B: serde::Serialize,
        Q: serde::Serialize,
    {
        self.request(Method::PUT, path, &query, Some(serde_json::to_value(body)?))
            .await
    }
}

impl std::fmt::Debug for Bot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bot").field("token", &"[REDACTED]").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MessageRecipientQuery, chat_link_candidates, parse_success_payload,
        percent_encode_path_segment,
    };
    use crate::types::Message;

    #[test]
    fn bot_uses_platform_api_v2_by_default() {
        let bot = super::Bot::new("token").unwrap();

        assert_eq!(bot.url("/me"), "https://platform-api2.max.ru/me");
    }

    #[test]
    fn chat_link_path_segment_is_percent_encoded() {
        assert_eq!(
            percent_encode_path_segment("https://max.ru/ru_3dnews"),
            "https%3A%2F%2Fmax.ru%2Fru_3dnews"
        );
        assert_eq!(percent_encode_path_segment("@ru_3dnews"), "@ru_3dnews");
    }

    #[test]
    fn chat_link_candidates_support_full_max_urls() {
        assert_eq!(
            chat_link_candidates("https://max.ru/ru_3dnews/"),
            [
                "https://max.ru/ru_3dnews".to_string(),
                "ru_3dnews".to_string(),
                "@ru_3dnews".to_string(),
            ]
        );
        assert_eq!(
            chat_link_candidates("@ru_3dnews"),
            ["@ru_3dnews", "ru_3dnews"]
        );
        assert_eq!(
            chat_link_candidates("ru_3dnews"),
            ["ru_3dnews", "@ru_3dnews"]
        );
    }

    #[test]
    fn parse_success_payload_supports_direct_message_response() {
        let json = r#"{
            "sender": {"user_id": 1, "name": "Alice"},
            "recipient": {"chat_id": 42, "chat_type": "dialog"},
            "timestamp": 1700000000,
            "body": {"mid": "mid_1", "seq": 1, "text": "hello"}
        }"#;

        let message: Message = parse_success_payload(json).expect("direct message response");
        assert_eq!(message.chat_id(), 42);
        assert_eq!(message.message_id(), "mid_1");
        assert_eq!(message.text(), Some("hello"));
    }

    #[test]
    fn parse_success_payload_supports_wrapped_message_response() {
        let json = r#"{
            "message": {
                "sender": {"user_id": 1, "name": "Alice"},
                "recipient": {"chat_id": 42, "chat_type": "dialog"},
                "timestamp": 1700000000,
                "body": {"mid": "mid_1", "seq": 1, "text": "hello"}
            }
        }"#;

        let message: Message = parse_success_payload(json).expect("wrapped message response");
        assert_eq!(message.chat_id(), 42);
        assert_eq!(message.message_id(), "mid_1");
        assert_eq!(message.text(), Some("hello"));
    }

    #[test]
    fn message_recipient_query_uses_chat_id_for_chat_targets() {
        assert_eq!(
            MessageRecipientQuery::ChatId(42).into_query(),
            [("chat_id", "42".to_string())]
        );
    }

    #[test]
    fn message_recipient_query_uses_user_id_for_user_targets() {
        assert_eq!(
            MessageRecipientQuery::UserId(5465382).into_query(),
            [("user_id", "5465382".to_string())]
        );
    }
}
