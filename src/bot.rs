use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::debug;

use crate::errors::{MaxError, Result};
use crate::types::*;

const BASE_URL: &str = "https://platform-api.max.ru";

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
}

impl Bot {
    /// Create a new bot with the given token.
    pub fn new(token: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Bot {
            inner: Arc::new(BotInner {
                token: token.into(),
                client,
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

    /// Returns a reference to the raw HTTP client.
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

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.get_with_query::<T, [(&str, &str); 0]>(path, []).await
    }

    async fn get_with_query<T, Q>(&self, path: &str, query: Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: serde::Serialize,
    {
        debug!("GET {path}");
        let resp = self
            .inner
            .client
            .get(self.url(path))
            .header("Authorization", self.auth())
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
        let resp = self
            .inner
            .client
            .post(self.url(path))
            .header("Authorization", self.auth())
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
        let resp = self
            .inner
            .client
            .put(self.url(path))
            .header("Authorization", self.auth())
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
        let resp = self
            .inner
            .client
            .patch(self.url(path))
            .header("Authorization", self.auth())
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
        let resp = self
            .inner
            .client
            .delete(self.url(path))
            .header("Authorization", self.auth())
            .query(&query)
            .send()
            .await?;
        self.parse(resp).await
    }

    async fn parse<T: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let text = resp.text().await?;
        debug!("Response {status}: {text}");

        if status.is_success() {
            serde_json::from_str(&text).map_err(MaxError::Json)
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

    // ────────────────────────────────────────────────
    // Messages
    // ────────────────────────────────────────────────

    /// POST /messages — Send a message to a chat.
    pub async fn send_message(&self, chat_id: i64, body: NewMessageBody) -> Result<Message> {
        self.post_with_query("/messages", &body, [("chat_id", chat_id.to_string())])
            .await
    }

    /// Convenience: send a plain-text message.
    pub async fn send_text(&self, chat_id: i64, text: impl Into<String>) -> Result<Message> {
        self.send_message(chat_id, NewMessageBody::text(text)).await
    }

    /// Convenience: send a Markdown-formatted message.
    pub async fn send_markdown(&self, chat_id: i64, text: impl Into<String>) -> Result<Message> {
        self.send_message(
            chat_id,
            NewMessageBody::text(text).with_format(MessageFormat::Markdown),
        )
        .await
    }

    /// PUT /messages — Edit an existing message.
    pub async fn edit_message(&self, message_id: &str, body: NewMessageBody) -> Result<Message> {
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

    /// POST /answers — Respond to an inline button callback.
    pub async fn answer_callback(&self, body: AnswerCallbackBody) -> Result<SimpleResult> {
        self.post("/answers", &body).await
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

    /// PATCH /chats/{chatId} — Edit chat title/description.
    pub async fn edit_chat(&self, chat_id: i64, body: EditChatBody) -> Result<Chat> {
        self.patch(&format!("/chats/{chat_id}"), &body).await
    }

    /// DELETE /chats/{chatId} — Delete a chat.
    pub async fn delete_chat(&self, chat_id: i64) -> Result<SimpleResult> {
        self.delete(&format!("/chats/{chat_id}")).await
    }

    /// POST /chats/{chatId}/actions — Send a typing indicator.
    pub async fn send_action(&self, chat_id: i64, action: &str) -> Result<SimpleResult> {
        #[derive(serde::Serialize)]
        struct ActionBody<'a> {
            action: &'a str,
        }
        self.post(&format!("/chats/{chat_id}/actions"), &ActionBody { action })
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
        self.delete_with_query(
            &format!("/chats/{chat_id}/members"),
            [("user_id", user_id.to_string())],
        )
        .await
    }

    /// GET /chats/{chatId}/members/admins — Get administrators of a chat.
    pub async fn get_admins(&self, chat_id: i64) -> Result<ChatMembersList> {
        self.get(&format!("/chats/{chat_id}/members/admins")).await
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
        if let Some(m) = marker {
            params.push(("marker", m.to_string()));
        }
        if let Some(t) = timeout {
            params.push(("timeout", t.to_string()));
        }
        if let Some(l) = limit {
            params.push(("limit", l.to_string()));
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
            [("type", format!("{:?}", upload_type).to_lowercase())],
        )
        .await
    }

    // ────────────────────────────────────────────────
    // Bot commands
    // ────────────────────────────────────────────────

    /// Set the list of commands shown to users.
    ///
    /// This is a convenience that POSTs to the commands endpoint.
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
        let resp = self
            .inner
            .client
            .put(self.url(path))
            .header("Authorization", self.auth())
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
