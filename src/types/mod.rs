use serde::{Deserialize, Deserializer, Serialize};

// ────────────────────────────────────────────────
// User / Bot info
// ────────────────────────────────────────────────

/// Represents a Max user or bot.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    /// Global MAX user identifier.
    ///
    /// Do not confuse this with `chat_id`: one user can appear in different
    /// private dialogs or group chats, each with its own `chat_id`.
    pub user_id: i64,
    pub name: String,
    pub username: Option<String>,
    pub is_bot: Option<bool>,
    pub last_activity_time: Option<i64>,
    pub avatar_url: Option<String>,
    pub full_avatar_url: Option<String>,
}

// ────────────────────────────────────────────────
// Chat
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatType {
    Dialog,
    Chat,
    Channel,
}

/// Represents a Max chat (dialog or group).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chat {
    /// Identifier of a concrete dialog, group, or channel.
    ///
    /// Do not confuse this with a user's global `user_id`.
    pub chat_id: i64,
    pub r#type: ChatType,
    pub status: Option<String>,
    pub title: Option<String>,
    pub icon: Option<Image>,
    pub last_event_time: Option<i64>,
    pub participants_count: Option<i32>,
    pub owner_id: Option<i64>,
    pub is_public: Option<bool>,
    pub link: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
}

/// Response from GET /chats
#[derive(Debug, Clone, Deserialize)]
pub struct ChatList {
    pub chats: Vec<Chat>,
    pub marker: Option<i64>,
}

/// Body for PATCH /chats/{chatId}
#[derive(Debug, Clone, Serialize, Default)]
pub struct EditChatBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

// ────────────────────────────────────────────────
// Message
// ────────────────────────────────────────────────

/// Text format for outgoing messages.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageFormat {
    #[default]
    Markdown,
    Html,
    Plain,
}

/// Represents a received message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Message {
    pub sender: Option<User>,
    pub recipient: Recipient,
    pub timestamp: i64,
    pub link: Option<LinkedMessage>,
    pub body: MessageBody,
    pub stat: Option<MessageStat>,
    pub url: Option<String>,
    pub constructor: Option<serde_json::Value>,
}

impl Message {
    /// Shortcut: get the `chat_id` this message was sent in.
    ///
    /// This is the dialog/group/channel identifier, not the sender's global
    /// MAX `user_id`.
    pub fn chat_id(&self) -> i64 {
        self.recipient.chat_id
    }

    /// Shortcut: get the message_id.
    pub fn message_id(&self) -> &str {
        &self.body.mid
    }

    /// Shortcut: get text content of the message.
    pub fn text(&self) -> Option<&str> {
        self.body.text.as_deref()
    }
}

/// Target chat metadata attached to a received message.
///
/// In private dialogs, `chat_id` is the ID of the dialog itself, while
/// `user_id` can carry the global MAX user ID of the peer.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Recipient {
    /// ID of the concrete dialog/group/channel that received the message.
    pub chat_id: i64,
    pub chat_type: ChatType,
    /// Optional global MAX user ID for dialog recipients.
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageBody {
    pub mid: String,
    pub seq: i64,
    pub text: Option<String>,
    #[serde(default, deserialize_with = "deserialize_attachments_lossy")]
    pub attachments: Option<Vec<Attachment>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageStat {
    pub views: Option<i32>,
}

fn deserialize_attachments_lossy<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<Attachment>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Vec<serde_json::Value>>::deserialize(deserializer)?;

    Ok(raw.map(|items| {
        items
            .into_iter()
            .map(|value| serde_json::from_value::<Attachment>(value).unwrap_or(Attachment::Unknown))
            .collect()
    }))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkedMessage {
    pub r#type: String,
    pub sender: Option<User>,
    pub chat_id: Option<i64>,
    pub message: Option<MessageBody>,
}

/// Response from GET /messages
#[derive(Debug, Clone, Deserialize)]
pub struct MessageList {
    pub messages: Vec<Message>,
}

// ────────────────────────────────────────────────
// Attachments
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Attachment {
    Image {
        payload: MediaPayload,
    },
    Video {
        payload: MediaPayload,
    },
    Audio {
        payload: MediaPayload,
    },
    File {
        payload: FilePayload,
    },
    Sticker {
        payload: StickerPayload,
    },
    InlineKeyboard {
        payload: KeyboardPayload,
    },
    Location {
        payload: LocationPayload,
    },
    Contact {
        payload: ContactPayload,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaPayload {
    pub url: Option<String>,
    pub token: Option<String>,
    pub photo_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilePayload {
    pub url: Option<String>,
    pub token: Option<String>,
    pub filename: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StickerPayload {
    pub code: String,
    pub url: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocationPayload {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContactPayload {
    pub name: Option<String>,
    pub contact_id: Option<i64>,
    pub vcf_info: Option<String>,
    pub vcf_phone: Option<String>,
}

// ────────────────────────────────────────────────
// Keyboard
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct KeyboardPayload {
    /// Rows of buttons (max 30 rows, max 7 buttons per row).
    pub buttons: Vec<Vec<Button>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Button {
    /// Sends a callback event to the bot.
    Callback {
        text: String,
        payload: String,
        intent: Option<ButtonIntent>,
    },
    /// Opens a URL.
    Link {
        text: String,
        url: String,
        intent: Option<ButtonIntent>,
    },
    /// Sends a text message as the user.
    Message {
        text: String,
        intent: Option<ButtonIntent>,
    },
    /// Requests the user's contact card.
    ///
    /// MAX documents this button, but live tests have observed contact updates
    /// with empty `contact_id` and `vcf_phone`, so phone delivery is not
    /// currently guaranteed on the MAX side.
    RequestContact { text: String },
    /// Requests the user's geo location.
    ///
    /// MAX documents this button, but live tests have observed the client-side
    /// location card without a matching bot update, so end-to-end delivery is
    /// not currently guaranteed on the MAX side.
    RequestGeoLocation { text: String, quick: Option<bool> },
}

/// Visual style of a button.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ButtonIntent {
    #[default]
    Default,
    Positive,
    Negative,
}

impl Button {
    pub fn callback(text: impl Into<String>, payload: impl Into<String>) -> Self {
        Button::Callback {
            text: text.into(),
            payload: payload.into(),
            intent: None,
        }
    }

    pub fn link(text: impl Into<String>, url: impl Into<String>) -> Self {
        Button::Link {
            text: text.into(),
            url: url.into(),
            intent: None,
        }
    }
}

// ────────────────────────────────────────────────
// New message body (outgoing)
// ────────────────────────────────────────────────

/// Body for POST /messages.
#[derive(Debug, Clone, Serialize, Default)]
pub struct NewMessageBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<NewAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<NewMessageLink>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<MessageFormat>,
}

impl NewMessageBody {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn with_keyboard(mut self, keyboard: KeyboardPayload) -> Self {
        let attachments = self.attachments.get_or_insert_with(Vec::new);
        attachments.push(NewAttachment::InlineKeyboard { payload: keyboard });
        self
    }

    pub fn with_format(mut self, format: MessageFormat) -> Self {
        self.format = Some(format);
        self
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NewAttachment {
    InlineKeyboard { payload: KeyboardPayload },
    Image { payload: UploadedToken },
    Video { payload: UploadedToken },
    Audio { payload: UploadedToken },
    File { payload: UploadedToken },
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadedToken {
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewMessageLink {
    pub r#type: LinkType,
    pub mid: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Forward,
    Reply,
}

// ────────────────────────────────────────────────
// Updates / Events (long polling & webhook)
// ────────────────────────────────────────────────

/// Container returned by GET /updates.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdatesResponse {
    pub updates: Vec<Update>,
    pub marker: Option<i64>,
}

/// A single update event from the Max platform.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "update_type", rename_all = "snake_case")]
pub enum Update {
    /// A new message was received.
    MessageCreated { timestamp: i64, message: Message },
    /// A message was edited.
    MessageEdited { timestamp: i64, message: Message },
    /// A message was deleted.
    MessageRemoved {
        timestamp: i64,
        message_id: String,
        chat_id: i64,
        user_id: i64,
    },
    /// A user pressed an inline button.
    MessageCallback {
        timestamp: i64,
        callback: Callback,
        message: Option<Message>,
        user_locale: Option<String>,
    },
    /// The bot was started in a private chat.
    BotStarted {
        timestamp: i64,
        chat_id: i64,
        user: User,
        payload: Option<String>,
        user_locale: Option<String>,
    },
    /// The bot was added to a chat.
    BotAdded {
        timestamp: i64,
        chat_id: i64,
        user: User,
        is_channel: Option<bool>,
    },
    /// The bot was removed from a chat.
    BotRemoved {
        timestamp: i64,
        chat_id: i64,
        user: User,
        is_channel: Option<bool>,
    },
    /// A user joined a chat where the bot is a member.
    UserAdded {
        timestamp: i64,
        chat_id: i64,
        user: User,
        inviter_id: Option<i64>,
        is_channel: Option<bool>,
    },
    /// A user left a chat where the bot is a member.
    UserRemoved {
        timestamp: i64,
        chat_id: i64,
        user: User,
        admin_id: Option<i64>,
        is_channel: Option<bool>,
    },
    /// The bot received a message with a chat title change.
    ChatTitleChanged {
        timestamp: i64,
        chat_id: i64,
        user: User,
        title: String,
    },
}

impl Update {
    /// Returns the timestamp of this update.
    pub fn timestamp(&self) -> i64 {
        match self {
            Update::MessageCreated { timestamp, .. } => *timestamp,
            Update::MessageEdited { timestamp, .. } => *timestamp,
            Update::MessageRemoved { timestamp, .. } => *timestamp,
            Update::MessageCallback { timestamp, .. } => *timestamp,
            Update::BotStarted { timestamp, .. } => *timestamp,
            Update::BotAdded { timestamp, .. } => *timestamp,
            Update::BotRemoved { timestamp, .. } => *timestamp,
            Update::UserAdded { timestamp, .. } => *timestamp,
            Update::UserRemoved { timestamp, .. } => *timestamp,
            Update::ChatTitleChanged { timestamp, .. } => *timestamp,
        }
    }
}

/// An inline button callback.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Callback {
    pub callback_id: String,
    pub user: User,
    pub payload: Option<String>,
    pub timestamp: i64,
}

// ────────────────────────────────────────────────
// Subscriptions (webhook)
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subscription {
    pub url: String,
    pub time: i64,
    pub update_types: Option<Vec<String>>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionList {
    pub subscriptions: Vec<Subscription>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscribeBody {
    /// HTTPS URL of your bot endpoint (must be port 443, no self-signed certs).
    pub url: String,
    /// Optional list of update types to receive (e.g. `["message_created", "bot_started"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Optional secret (5–256 chars, `[A-Za-z0-9_-]`).
    /// Sent in the `X-Max-Bot-Api-Secret` header on every webhook request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

/// Response from a multipart file upload (image / file).
/// For video/audio the token is returned by `POST /uploads` before the upload.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadResponse {
    /// Ready-to-use attachment token (for image and file types).
    pub token: Option<String>,
}

// ────────────────────────────────────────────────
// Answer on callback
// ────────────────────────────────────────────────

/// Body for POST /answers.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AnswerCallbackBody {
    pub callback_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<NewMessageBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<String>,
}

// ────────────────────────────────────────────────
// Upload
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadType {
    /// Still images (JPG, JPEG, PNG, GIF, TIFF, BMP, HEIC).
    /// NOTE: `photo` was removed from the API — always use `image`.
    Image,
    /// Video files (MP4, MOV, MKV, WEBM).
    Video,
    /// Audio files (MP3, WAV, M4A, …).
    Audio,
    /// Any other file type (max 4 GB).
    File,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadEndpoint {
    pub url: String,
    pub token: Option<String>,
}

/// Generic simple JSON result `{"success": true}`.
#[derive(Debug, Clone, Deserialize)]
pub struct SimpleResult {
    pub success: bool,
    pub message: Option<String>,
}

// ────────────────────────────────────────────────
// Chat members
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChatMember {
    pub user_id: i64,
    pub name: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub is_owner: Option<bool>,
    pub is_admin: Option<bool>,
    pub join_time: Option<i64>,
    pub permissions: Option<Vec<String>>,
    pub last_access_time: Option<i64>,
    pub is_bot: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMembersList {
    pub members: Vec<ChatMember>,
    pub marker: Option<i64>,
}

/// Body for POST /chats/{chatId}/members
#[derive(Debug, Clone, Serialize)]
pub struct AddMembersBody {
    pub user_ids: Vec<i64>,
}

/// Body for DELETE /chats/{chatId}/members
#[derive(Debug, Clone, Serialize)]
pub struct RemoveMemberQuery {
    pub user_id: i64,
}

/// Pinned message info
#[derive(Debug, Clone, Deserialize)]
pub struct PinnedMessage {
    pub message: Message,
}

/// Body for PUT /chats/{chatId}/pin
#[derive(Debug, Clone, Serialize)]
pub struct PinMessageBody {
    pub message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

/// Bot command for setMyCommands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCommand {
    pub name: String,
    pub description: String,
}
