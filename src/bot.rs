use reqwest::{Certificate, Client};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::{debug, warn};

use crate::errors::{MaxError, Result};
use crate::types::*;

const BASE_URL: &str = "https://platform-api2.max.ru";
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 30;
const CA_FETCH_TIMEOUT_SECS: u64 = 10;
const RUSSIAN_TRUSTED_ROOT_CA_URL: &str =
    "https://gu-st.ru/content/lending/russian_trusted_root_ca_pem.crt";
const RUSSIAN_TRUSTED_ROOT_CA_PEM: &[u8] = include_bytes!("certs/russian_trusted_root_ca.pem");

fn default_client_builder() -> reqwest::ClientBuilder {
    Client::builder().timeout(Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS))
}

fn certificates_from_bytes(bytes: &[u8]) -> std::result::Result<Vec<Certificate>, reqwest::Error> {
    match Certificate::from_pem_bundle(bytes) {
        Ok(certs) if !certs.is_empty() => Ok(certs),
        Ok(_) | Err(_) => Certificate::from_der(bytes).map(|cert| vec![cert]),
    }
}

fn embedded_russian_trusted_root_ca() -> std::result::Result<Vec<Certificate>, reqwest::Error> {
    Certificate::from_pem_bundle(RUSSIAN_TRUSTED_ROOT_CA_PEM)
}

fn build_client_with_certs(certs: Vec<Certificate>) -> std::result::Result<Client, reqwest::Error> {
    default_client_builder().tls_certs_merge(certs).build()
}

fn build_client_with_embedded_ca() -> std::result::Result<Client, reqwest::Error> {
    build_client_with_certs(embedded_russian_trusted_root_ca()?)
}

async fn download_russian_trusted_root_ca() -> std::result::Result<Vec<Certificate>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(CA_FETCH_TIMEOUT_SECS))
        .build()
        .map_err(|err| format!("failed to build CA download client: {err}"))?;
    let response = client
        .get(RUSSIAN_TRUSTED_ROOT_CA_URL)
        .send()
        .await
        .map_err(|err| format!("failed to download Russian Trusted Root CA: {err}"))?;
    let status = response.status();

    if !status.is_success() {
        return Err(format!(
            "Russian Trusted Root CA download returned HTTP {status}"
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| format!("failed to read Russian Trusted Root CA body: {err}"))?;

    certificates_from_bytes(&bytes)
        .map_err(|err| format!("failed to parse Russian Trusted Root CA: {err}"))
}

async fn build_auto_client() -> Result<Client> {
    match download_russian_trusted_root_ca().await {
        Ok(certs) => match build_client_with_certs(certs) {
            Ok(client) => {
                debug!("Loaded Russian Trusted Root CA from {RUSSIAN_TRUSTED_ROOT_CA_URL}");
                Ok(client)
            }
            Err(err) => {
                warn!(
                    "Failed to build client with downloaded Russian Trusted Root CA: {err}; using embedded fallback"
                );
                build_client_with_embedded_ca().map_err(MaxError::Http)
            }
        },
        Err(err) => {
            warn!("{err}; using embedded Russian Trusted Root CA fallback");
            build_client_with_embedded_ca().map_err(MaxError::Http)
        }
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
/// async fn main() {
///     let bot = Bot::from_env();
///     let me = bot.get_me().await.unwrap();
///     println!("Running as @{}", me.username.unwrap_or_default());
/// }
/// ```
#[derive(Clone)]
pub struct Bot {
    inner: Arc<BotInner>,
}

struct BotInner {
    token: String,
    client: Client,
    auto_client: Option<OnceCell<Client>>,
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

impl Bot {
    /// Create a new bot with the given token.
    pub fn new(token: impl Into<String>) -> Self {
        let client = build_client_with_embedded_ca().expect("Failed to build HTTP client");

        Bot {
            inner: Arc::new(BotInner {
                token: token.into(),
                client,
                auto_client: Some(OnceCell::new()),
            }),
        }
    }

    /// Create a new bot with a custom HTTP client.
    ///
    /// The provided client is used as-is. Automatic Russian Trusted Root CA
    /// refresh is only applied to clients created by [`Bot::new`] and
    /// [`Bot::from_env`].
    pub fn with_client(token: impl Into<String>, client: Client) -> Self {
        Bot {
            inner: Arc::new(BotInner {
                token: token.into(),
                client,
                auto_client: None,
            }),
        }
    }

    /// Create a bot reading the token from the `MAX_BOT_TOKEN` environment variable.
    ///
    /// # Panics
    /// Panics if the environment variable is not set.
    pub fn from_env() -> Self {
        let token =
            std::env::var("MAX_BOT_TOKEN").expect("MAX_BOT_TOKEN environment variable is not set");
        Self::new(token)
    }

    /// Returns a reference to the initially built raw HTTP client.
    ///
    /// For bots created by [`Bot::new`] or [`Bot::from_env`], API methods may
    /// use an internally refreshed client after automatic CA loading succeeds.
    pub fn client(&self) -> &Client {
        &self.inner.client
    }

    /// Returns the bot token.
    pub fn token(&self) -> &str {
        &self.inner.token
    }

    // ────────────────────────────────────────────────
    // Internal helpers
    // ────────────────────────────────────────────────

    fn url(&self, path: &str) -> String {
        format!("{BASE_URL}{path}")
    }

    fn auth(&self) -> String {
        self.inner.token.clone()
    }

    pub(crate) async fn api_client(&self) -> Result<&Client> {
        match &self.inner.auto_client {
            Some(auto_client) => {
                auto_client
                    .get_or_try_init(|| async { build_auto_client().await })
                    .await
            }
            None => Ok(&self.inner.client),
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with_query::<T, [(&str, &str); 0]>(path, []).await
    }

    async fn get_with_query<T, Q>(&self, path: &str, query: Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: serde::Serialize,
    {
        debug!("GET {path}");
        let url = self.url(path);
        let auth = self.auth();
        let resp = self
            .api_client()
            .await?
            .get(url)
            .header("Authorization", auth)
            .query(&query)
            .send()
            .await?;
        self.parse(resp).await
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
        debug!("POST {path}");
        let url = self.url(path);
        let auth = self.auth();
        let resp = self
            .api_client()
            .await?
            .post(url)
            .header("Authorization", auth)
            .query(&query)
            .json(body)
            .send()
            .await?;
        self.parse(resp).await
    }

    async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        debug!("PUT {path}");
        let url = self.url(path);
        let auth = self.auth();
        let resp = self
            .api_client()
            .await?
            .put(url)
            .header("Authorization", auth)
            .json(body)
            .send()
            .await?;
        self.parse(resp).await
    }

    async fn patch<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        debug!("PATCH {path}");
        let url = self.url(path);
        let auth = self.auth();
        let resp = self
            .api_client()
            .await?
            .patch(url)
            .header("Authorization", auth)
            .json(body)
            .send()
            .await?;
        self.parse(resp).await
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
        debug!("DELETE {path}");
        let url = self.url(path);
        let auth = self.auth();
        let resp = self
            .api_client()
            .await?
            .delete(url)
            .header("Authorization", auth)
            .query(&query)
            .send()
            .await?;
        self.parse(resp).await
    }

    async fn parse<T: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let bytes = resp.bytes().await?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        debug!("Response {status}: {text}");

        if status.is_success() {
            parse_success_payload(&text).map_err(MaxError::Json)
        } else {
            // Try to extract an error message from the JSON body.
            let message = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v.get("message")
                        .or_else(|| v.get("error"))
                        .and_then(|m| m.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| text.clone());

            Err(MaxError::Api {
                code: status.as_u16(),
                message,
            })
        }
    }

    // ────────────────────────────────────────────────
    // Bots
    // ────────────────────────────────────────────────

    /// GET /me — Returns info about the current bot.
    pub async fn get_me(&self) -> Result<User> {
        self.get("/me").await
    }

    /// PATCH /me — Edit the current bot's profile, commands, or avatar.
    pub async fn edit_my_info(&self, body: EditMyInfoBody) -> Result<User> {
        self.patch("/me", &body).await
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

    /// PUT /messages — Edit an existing message.
    pub async fn edit_message(
        &self,
        message_id: &str,
        body: NewMessageBody,
    ) -> Result<SimpleResult> {
        self.put_with_query("/messages", &body, [("message_id", message_id)])
            .await
    }

    /// DELETE /messages — Delete a message.
    pub async fn delete_message(&self, message_id: &str) -> Result<SimpleResult> {
        self.delete_with_query("/messages", [("message_id", message_id)])
            .await
    }

    /// GET /messages/{messageId} — Get a single message by ID.
    pub async fn get_message(&self, message_id: &str) -> Result<Message> {
        self.get(&format!("/messages/{message_id}")).await
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
        #[derive(serde::Serialize)]
        struct AnswerBody {
            #[serde(skip_serializing_if = "Option::is_none")]
            message: Option<NewMessageBody>,
            #[serde(skip_serializing_if = "Option::is_none")]
            notification: Option<String>,
        }

        self.post_with_query(
            "/answers",
            &AnswerBody {
                message: body.message,
                notification: body.notification,
            },
            [("callback_id", body.callback_id)],
        )
        .await
    }

    // ────────────────────────────────────────────────
    // Chats
    // ────────────────────────────────────────────────

    /// GET /chats — Get all group chats the bot is a member of.
    pub async fn get_chats(&self, count: Option<u32>, marker: Option<i64>) -> Result<ChatList> {
        let mut params: Vec<(&str, String)> = vec![];
        if let Some(c) = count {
            params.push(("count", c.to_string()));
        }
        if let Some(m) = marker {
            params.push(("marker", m.to_string()));
        }
        self.get_with_query("/chats", &params).await
    }

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
            return Err(MaxError::Api {
                code: 0,
                message: "chat_link is empty".into(),
            });
        }

        let tried = candidates.join(", ");
        let mut last_error = None;

        for candidate in &candidates {
            let encoded = percent_encode_path_segment(candidate);
            match self.get(&format!("/chats/{encoded}")).await {
                Ok(chat) => return Ok(chat),
                Err(err) => {
                    let should_try_next = matches!(err, MaxError::Api { code: 404, .. });
                    if !should_try_next {
                        return Err(err);
                    }
                    last_error = Some(err);
                }
            }
        }

        match last_error {
            Some(MaxError::Api { code: 404, message }) => Err(MaxError::Api {
                code: 404,
                message: format!("{message}. Tried variants: {tried}"),
            }),
            Some(err) => Err(err),
            None => Err(MaxError::Api {
                code: 0,
                message: "chat_link is empty".into(),
            }),
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

    /// Attempt to set the list of commands shown to users.
    ///
    /// The public MAX REST docs expose bot commands in `GET /me`, but do not
    /// currently document a write endpoint for updating that menu.
    /// Live requests to `POST /me/commands` currently return
    /// `404 Path /me/commands is not recognized`.
    ///
    /// This helper is kept for experimentation and future MAX support.
    pub async fn set_my_commands(&self, commands: Vec<BotCommand>) -> Result<SimpleResult> {
        #[derive(serde::Serialize)]
        struct CommandsBody {
            commands: Vec<BotCommand>,
        }
        self.post("/me/commands", &CommandsBody { commands }).await
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
        debug!("PUT {path}");
        let url = self.url(path);
        let auth = self.auth();
        let resp = self
            .api_client()
            .await?
            .put(url)
            .header("Authorization", auth)
            .query(&query)
            .json(body)
            .send()
            .await?;
        self.parse(resp).await
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
        let bot = super::Bot::new("token");

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
