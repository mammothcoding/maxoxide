use reqwest::Client;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::debug;

use crate::errors::{MaxError, Result};
use crate::types::*;

const BASE_URL: &str = "https://platform-api.max.ru";

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

impl Bot {
    /// Create a new bot with the given token.
    pub fn new(token: impl Into<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self::with_client(token, client)
    }

    /// Create a new bot with a custom HTTP client.
    pub fn with_client(token: impl Into<String>, client: Client) -> Self {
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
    /// Note: live MAX tests currently show successful API responses for
    /// `"typing_on"`, but the client-side typing indicator is not reliably
    /// visible. Treat the visual effect as a current MAX platform gap.
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

    /// GET /updates — Poll for raw JSON updates once.
    pub async fn get_updates_raw(
        &self,
        marker: Option<i64>,
        timeout: Option<u32>,
        limit: Option<u32>,
    ) -> Result<RawUpdatesResponse> {
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

#[cfg(test)]
mod tests {
    use super::{MessageRecipientQuery, parse_success_payload};
    use crate::types::Message;

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
