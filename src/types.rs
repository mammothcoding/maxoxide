use std::{collections::BTreeMap, fmt};

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{DeserializeOwned, Error as DeError},
    ser::SerializeStruct,
};

// ────────────────────────────────────────────────
// String enums with unknown-value preservation
// ────────────────────────────────────────────────

fn deserialize_string_enum<'de, D, T, F>(deserializer: D, from_str: F) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    F: FnOnce(String) -> T,
{
    let value = String::deserialize(deserializer)?;
    Ok(from_str(value))
}

fn serialize_string_enum<S>(serializer: S, value: &str) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value)
}

// ────────────────────────────────────────────────
// User / Bot info
// ────────────────────────────────────────────────

/// Represents a Max user or bot.
#[derive(Debug, Clone, Serialize)]
pub struct User {
    /// Global MAX user identifier.
    ///
    /// Do not confuse this with `chat_id`: one user can appear in different
    /// private dialogs or group chats, each with its own `chat_id`.
    pub user_id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub is_bot: Option<bool>,
    pub last_activity_time: Option<i64>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub full_avatar_url: Option<String>,
    pub commands: Option<Vec<BotCommand>>,
}

impl User {
    /// Returns a user-facing display name from first and last name.
    pub fn display_name(&self) -> String {
        match self.last_name.as_deref() {
            Some(last_name) if !last_name.is_empty() => {
                format!("{} {}", self.first_name, last_name)
            }
            _ => self.first_name.clone(),
        }
    }
}

impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireUser {
            user_id: i64,
            #[serde(default)]
            first_name: Option<String>,
            #[serde(default)]
            last_name: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            username: Option<String>,
            #[serde(default)]
            is_bot: Option<bool>,
            #[serde(default)]
            last_activity_time: Option<i64>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            avatar_url: Option<String>,
            #[serde(default)]
            full_avatar_url: Option<String>,
            #[serde(default)]
            commands: Option<Vec<BotCommand>>,
        }

        let wire = WireUser::deserialize(deserializer)?;
        let first_name = wire
            .first_name
            .or(wire.name)
            .ok_or_else(|| D::Error::missing_field("first_name"))?;

        Ok(Self {
            user_id: wire.user_id,
            first_name,
            last_name: wire.last_name,
            username: wire.username,
            is_bot: wire.is_bot,
            last_activity_time: wire.last_activity_time,
            description: wire.description,
            avatar_url: wire.avatar_url,
            full_avatar_url: wire.full_avatar_url,
            commands: wire.commands,
        })
    }
}

// ────────────────────────────────────────────────
// Chat
// ────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatType {
    Dialog,
    Chat,
    Channel,
    Unknown(String),
}

impl ChatType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Dialog => "dialog",
            Self::Chat => "chat",
            Self::Channel => "channel",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for ChatType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for ChatType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "dialog" => Self::Dialog,
            "chat" => Self::Chat,
            "channel" => Self::Channel,
            _ => Self::Unknown(value),
        })
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatStatus {
    Active,
    Removed,
    Left,
    Closed,
    Unknown(String),
}

impl ChatStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Removed => "removed",
            Self::Left => "left",
            Self::Closed => "closed",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for ChatStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for ChatStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "active" => Self::Active,
            "removed" => Self::Removed,
            "left" => Self::Left,
            "closed" => Self::Closed,
            _ => Self::Unknown(value),
        })
    }
}

/// Represents a Max chat (dialog or group).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Chat {
    /// Identifier of a concrete dialog, group, or channel.
    ///
    /// Do not confuse this with a user's global `user_id`.
    pub chat_id: i64,
    pub r#type: ChatType,
    pub status: Option<ChatStatus>,
    pub title: Option<String>,
    pub icon: Option<Image>,
    pub last_event_time: Option<i64>,
    pub participants_count: Option<i32>,
    pub owner_id: Option<i64>,
    pub is_public: Option<bool>,
    pub link: Option<String>,
    pub description: Option<String>,
    pub dialog_with_user: Option<User>,
    pub chat_message_id: Option<String>,
    pub pinned_message: Option<Box<Message>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    pub url: String,
}

/// Response from GET /chats.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatList {
    pub chats: Vec<Chat>,
    pub marker: Option<i64>,
}

/// Body for PATCH /chats/{chatId}.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EditChatBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<PhotoAttachmentPayload>,
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
///
/// Omit `format` for plain text.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MessageFormat {
    #[default]
    Markdown,
    Html,
    Unknown(String),
}

impl MessageFormat {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for MessageFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for MessageFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "markdown" => Self::Markdown,
            "html" => Self::Html,
            _ => Self::Unknown(value),
        })
    }
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

    /// Shortcut: get sender's global MAX user ID.
    pub fn sender_user_id(&self) -> Option<i64> {
        self.sender.as_ref().map(|sender| sender.user_id)
    }

    /// Returns true when this message contains at least one attachment.
    pub fn has_attachments(&self) -> bool {
        self.body
            .attachments
            .as_ref()
            .map(|attachments| !attachments.is_empty())
            .unwrap_or(false)
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
            .map(|value| {
                serde_json::from_value::<Attachment>(value.clone()).unwrap_or_else(|_| {
                    Attachment::Unknown {
                        r#type: value
                            .get("type")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        payload: value.get("payload").cloned(),
                        raw: value,
                    }
                })
            })
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

/// Response from GET /messages.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageList {
    pub messages: Vec<Message>,
}

// ────────────────────────────────────────────────
// Attachments
// ────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone)]
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
    Unknown {
        r#type: String,
        payload: Option<serde_json::Value>,
        raw: serde_json::Value,
    },
}

impl Attachment {
    pub fn kind(&self) -> AttachmentKind {
        match self {
            Self::Image { .. } => AttachmentKind::Image,
            Self::Video { .. } => AttachmentKind::Video,
            Self::Audio { .. } => AttachmentKind::Audio,
            Self::File { .. } => AttachmentKind::File,
            Self::Sticker { .. } => AttachmentKind::Sticker,
            Self::InlineKeyboard { .. } => AttachmentKind::InlineKeyboard,
            Self::Location { .. } => AttachmentKind::Location,
            Self::Contact { .. } => AttachmentKind::Contact,
            Self::Unknown { .. } => AttachmentKind::Unknown,
        }
    }
}

impl Serialize for Attachment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Image { payload } => serialize_attachment(serializer, "image", payload),
            Self::Video { payload } => serialize_attachment(serializer, "video", payload),
            Self::Audio { payload } => serialize_attachment(serializer, "audio", payload),
            Self::File { payload } => serialize_attachment(serializer, "file", payload),
            Self::Sticker { payload } => serialize_attachment(serializer, "sticker", payload),
            Self::InlineKeyboard { payload } => {
                serialize_attachment(serializer, "inline_keyboard", payload)
            }
            Self::Location { payload } => serialize_attachment(serializer, "location", payload),
            Self::Contact { payload } => serialize_attachment(serializer, "contact", payload),
            Self::Unknown { raw, .. } => raw.serialize(serializer),
        }
    }
}

fn serialize_attachment<S, T>(
    serializer: S,
    attachment_type: &str,
    payload: &T,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    let mut state = serializer.serialize_struct("Attachment", 2)?;
    state.serialize_field("type", attachment_type)?;
    state.serialize_field("payload", payload)?;
    state.end()
}

impl<'de> Deserialize<'de> for Attachment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let attachment_type = raw
            .get("type")
            .and_then(|value| value.as_str())
            .ok_or_else(|| D::Error::missing_field("type"))?;

        match attachment_type {
            "image" => Ok(Self::Image {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "video" => Ok(Self::Video {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "audio" => Ok(Self::Audio {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "file" => Ok(Self::File {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "sticker" => Ok(Self::Sticker {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "inline_keyboard" => Ok(Self::InlineKeyboard {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "location" => Ok(Self::Location {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "contact" => Ok(Self::Contact {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            _ => Ok(Self::Unknown {
                r#type: attachment_type.to_string(),
                payload: raw.get("payload").cloned(),
                raw,
            }),
        }
    }
}

fn deserialize_attachment_payload<T, E>(raw: &serde_json::Value) -> Result<T, E>
where
    T: DeserializeOwned,
    E: DeError,
{
    let value = raw.get("payload").cloned().unwrap_or_else(|| raw.clone());

    serde_json::from_value(value).map_err(E::custom)
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    Image,
    Video,
    Audio,
    File,
    Sticker,
    InlineKeyboard,
    Location,
    Contact,
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

#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Button {
    /// Sends a callback event to the bot.
    Callback {
        text: String,
        payload: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<ButtonIntent>,
    },
    /// Opens a URL.
    Link {
        text: String,
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<ButtonIntent>,
    },
    /// Sends a text message as the user.
    Message {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        intent: Option<ButtonIntent>,
    },
    /// Opens a MAX mini app.
    OpenApp {
        text: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        web_app: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        contact_id: Option<i64>,
    },
    /// Copies the payload to the clipboard.
    ///
    /// This button exists in the official Go SDK, but is not listed in the
    /// public REST documentation's button overview yet.
    Clipboard { text: String, payload: String },
    /// Requests the user's contact card.
    ///
    /// MAX documents this button, but live tests have observed contact updates
    /// with empty `contact_id` and `vcf_phone`, so phone delivery is not
    /// currently guaranteed on the MAX side.
    RequestContact { text: String },
    /// Requests the user's geo location.
    ///
    /// Live tests have observed MAX returning a `location` attachment with
    /// `latitude` and `longitude` directly on the attachment object.
    RequestGeoLocation {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        quick: Option<bool>,
    },
}

/// Visual style of a button.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ButtonIntent {
    #[default]
    Default,
    Positive,
    Negative,
    Unknown(String),
}

impl ButtonIntent {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for ButtonIntent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for ButtonIntent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "default" => Self::Default,
            "positive" => Self::Positive,
            "negative" => Self::Negative,
            _ => Self::Unknown(value),
        })
    }
}

impl Button {
    pub fn callback(text: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Callback {
            text: text.into(),
            payload: payload.into(),
            intent: None,
        }
    }

    pub fn link(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self::Link {
            text: text.into(),
            url: url.into(),
            intent: None,
        }
    }

    pub fn message(text: impl Into<String>) -> Self {
        Self::Message {
            text: text.into(),
            intent: None,
        }
    }

    pub fn open_app(text: impl Into<String>, web_app: impl Into<String>) -> Self {
        Self::OpenApp {
            text: text.into(),
            web_app: web_app.into(),
            payload: None,
            contact_id: None,
        }
    }

    pub fn open_app_with_payload(
        text: impl Into<String>,
        web_app: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self::OpenApp {
            text: text.into(),
            web_app: web_app.into(),
            payload: Some(payload.into()),
            contact_id: None,
        }
    }

    pub fn open_app_full(
        text: impl Into<String>,
        web_app: impl Into<String>,
        payload: Option<String>,
        contact_id: Option<i64>,
    ) -> Self {
        Self::OpenApp {
            text: text.into(),
            web_app: web_app.into(),
            payload,
            contact_id,
        }
    }

    pub fn clipboard(text: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Clipboard {
            text: text.into(),
            payload: payload.into(),
        }
    }

    pub fn request_contact(text: impl Into<String>) -> Self {
        Self::RequestContact { text: text.into() }
    }

    pub fn request_geo_location(text: impl Into<String>) -> Self {
        Self::RequestGeoLocation {
            text: text.into(),
            quick: None,
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
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn text_opt(text: Option<impl Into<String>>) -> Self {
        match text {
            Some(text) => Self::text(text),
            None => Self::empty(),
        }
    }

    pub fn with_attachment(mut self, attachment: NewAttachment) -> Self {
        self.attachments
            .get_or_insert_with(Vec::new)
            .push(attachment);
        self
    }

    pub fn with_attachments(
        mut self,
        attachments: impl IntoIterator<Item = NewAttachment>,
    ) -> Self {
        self.attachments
            .get_or_insert_with(Vec::new)
            .extend(attachments);
        self
    }

    pub fn with_keyboard(self, keyboard: KeyboardPayload) -> Self {
        self.with_attachment(NewAttachment::inline_keyboard(keyboard))
    }

    pub fn with_format(mut self, format: MessageFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_notify(mut self, notify: bool) -> Self {
        self.notify = Some(notify);
        self
    }

    pub fn with_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.link = Some(NewMessageLink {
            r#type: LinkType::Reply,
            mid: message_id.into(),
        });
        self
    }

    pub fn with_forward_from(mut self, message_id: impl Into<String>) -> Self {
        self.link = Some(NewMessageLink {
            r#type: LinkType::Forward,
            mid: message_id.into(),
        });
        self
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NewAttachment {
    InlineKeyboard { payload: KeyboardPayload },
    Image { payload: ImageAttachmentPayload },
    Video { payload: UploadedToken },
    Audio { payload: UploadedToken },
    File { payload: UploadedToken },
}

impl NewAttachment {
    pub fn inline_keyboard(keyboard: KeyboardPayload) -> Self {
        Self::InlineKeyboard { payload: keyboard }
    }

    pub fn image(token: impl Into<String>) -> Self {
        Self::Image {
            payload: ImageAttachmentPayload::token(token),
        }
    }

    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            payload: ImageAttachmentPayload::url(url),
        }
    }

    pub fn image_photos(photos: PhotoTokens) -> Self {
        Self::Image {
            payload: ImageAttachmentPayload::photos(photos),
        }
    }

    pub fn video(token: impl Into<String>) -> Self {
        Self::Video {
            payload: UploadedToken::new(token),
        }
    }

    pub fn audio(token: impl Into<String>) -> Self {
        Self::Audio {
            payload: UploadedToken::new(token),
        }
    }

    pub fn file(token: impl Into<String>) -> Self {
        Self::File {
            payload: UploadedToken::new(token),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhotoToken {
    pub token: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl PhotoToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            extra: BTreeMap::new(),
        }
    }
}

pub type PhotoTokens = BTreeMap<String, PhotoToken>;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageAttachmentPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<PhotoTokens>,
}

impl ImageAttachmentPayload {
    pub fn token(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            ..Default::default()
        }
    }

    pub fn url(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..Default::default()
        }
    }

    pub fn photos(photos: PhotoTokens) -> Self {
        Self {
            photos: Some(photos),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UploadedToken {
    pub token: String,
}

impl UploadedToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NewMessageLink {
    pub r#type: LinkType,
    pub mid: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkType {
    Forward,
    Reply,
    Unknown(String),
}

impl LinkType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Forward => "forward",
            Self::Reply => "reply",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for LinkType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for LinkType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "forward" => Self::Forward,
            "reply" => Self::Reply,
            _ => Self::Unknown(value),
        })
    }
}

/// Query options for POST /messages.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct SendMessageOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_link_preview: Option<bool>,
}

impl SendMessageOptions {
    pub fn disable_link_preview(disable: bool) -> Self {
        Self {
            disable_link_preview: Some(disable),
        }
    }
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

/// Raw container returned by GET /updates before typed update deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct RawUpdatesResponse {
    pub updates: Vec<serde_json::Value>,
    pub marker: Option<i64>,
}

/// A single update event from the Max platform.
///
/// The large `Message` payloads intentionally stay inline to keep public match
/// ergonomics simple for bot handlers.
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
#[derive(Debug, Clone)]
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
    /// A newer or currently unsupported update type.
    Unknown {
        update_type: Option<String>,
        timestamp: Option<i64>,
        raw: serde_json::Value,
    },
}

impl Update {
    /// Returns the timestamp of this update when one was present.
    pub fn timestamp(&self) -> Option<i64> {
        match self {
            Self::MessageCreated { timestamp, .. }
            | Self::MessageEdited { timestamp, .. }
            | Self::MessageRemoved { timestamp, .. }
            | Self::MessageCallback { timestamp, .. }
            | Self::BotStarted { timestamp, .. }
            | Self::BotAdded { timestamp, .. }
            | Self::BotRemoved { timestamp, .. }
            | Self::UserAdded { timestamp, .. }
            | Self::UserRemoved { timestamp, .. }
            | Self::ChatTitleChanged { timestamp, .. } => Some(*timestamp),
            Self::Unknown { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the timestamp or `0` when an unknown update did not include one.
    pub fn timestamp_or_default(&self) -> i64 {
        self.timestamp().unwrap_or_default()
    }

    pub fn update_type(&self) -> Option<&str> {
        match self {
            Self::MessageCreated { .. } => Some("message_created"),
            Self::MessageEdited { .. } => Some("message_edited"),
            Self::MessageRemoved { .. } => Some("message_removed"),
            Self::MessageCallback { .. } => Some("message_callback"),
            Self::BotStarted { .. } => Some("bot_started"),
            Self::BotAdded { .. } => Some("bot_added"),
            Self::BotRemoved { .. } => Some("bot_removed"),
            Self::UserAdded { .. } => Some("user_added"),
            Self::UserRemoved { .. } => Some("user_removed"),
            Self::ChatTitleChanged { .. } => Some("chat_title_changed"),
            Self::Unknown { update_type, .. } => update_type.as_deref(),
        }
    }

    pub fn raw(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Unknown { raw, .. } => Some(raw),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for Update {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let update_type = raw
            .get("update_type")
            .and_then(|value| value.as_str())
            .map(String::from);
        let timestamp = raw.get("timestamp").and_then(|value| value.as_i64());

        let Some(kind) = update_type.as_deref() else {
            return Ok(Self::Unknown {
                update_type,
                timestamp,
                raw,
            });
        };

        macro_rules! parse_update {
            ($wire:ty, $map:expr) => {
                match serde_json::from_value::<$wire>(raw.clone()) {
                    Ok(wire) => $map(wire),
                    Err(_) => Self::Unknown {
                        update_type,
                        timestamp,
                        raw,
                    },
                }
            };
        }

        #[derive(Deserialize)]
        struct MessageUpdate {
            timestamp: i64,
            message: Message,
        }

        #[derive(Deserialize)]
        struct MessageRemovedUpdate {
            timestamp: i64,
            message_id: String,
            chat_id: i64,
            user_id: i64,
        }

        #[derive(Deserialize)]
        struct MessageCallbackUpdate {
            timestamp: i64,
            callback: Callback,
            #[serde(default)]
            message: Option<Message>,
            #[serde(default)]
            user_locale: Option<String>,
        }

        #[derive(Deserialize)]
        struct BotStartedUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            #[serde(default)]
            payload: Option<String>,
            #[serde(default)]
            user_locale: Option<String>,
        }

        #[derive(Deserialize)]
        struct BotChatUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            #[serde(default)]
            is_channel: Option<bool>,
        }

        #[derive(Deserialize)]
        struct UserAddedUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            #[serde(default)]
            inviter_id: Option<i64>,
            #[serde(default)]
            is_channel: Option<bool>,
        }

        #[derive(Deserialize)]
        struct UserRemovedUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            #[serde(default)]
            admin_id: Option<i64>,
            #[serde(default)]
            is_channel: Option<bool>,
        }

        #[derive(Deserialize)]
        struct ChatTitleChangedUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            title: String,
        }

        Ok(match kind {
            "message_created" => parse_update!(MessageUpdate, |wire: MessageUpdate| {
                Self::MessageCreated {
                    timestamp: wire.timestamp,
                    message: wire.message,
                }
            }),
            "message_edited" => parse_update!(MessageUpdate, |wire: MessageUpdate| {
                Self::MessageEdited {
                    timestamp: wire.timestamp,
                    message: wire.message,
                }
            }),
            "message_removed" => {
                parse_update!(MessageRemovedUpdate, |wire: MessageRemovedUpdate| {
                    Self::MessageRemoved {
                        timestamp: wire.timestamp,
                        message_id: wire.message_id,
                        chat_id: wire.chat_id,
                        user_id: wire.user_id,
                    }
                })
            }
            "message_callback" => {
                parse_update!(MessageCallbackUpdate, |wire: MessageCallbackUpdate| {
                    Self::MessageCallback {
                        timestamp: wire.timestamp,
                        callback: wire.callback,
                        message: wire.message,
                        user_locale: wire.user_locale,
                    }
                })
            }
            "bot_started" => parse_update!(BotStartedUpdate, |wire: BotStartedUpdate| {
                Self::BotStarted {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    payload: wire.payload,
                    user_locale: wire.user_locale,
                }
            }),
            "bot_added" => parse_update!(BotChatUpdate, |wire: BotChatUpdate| {
                Self::BotAdded {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    is_channel: wire.is_channel,
                }
            }),
            "bot_removed" => parse_update!(BotChatUpdate, |wire: BotChatUpdate| {
                Self::BotRemoved {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    is_channel: wire.is_channel,
                }
            }),
            "user_added" => parse_update!(UserAddedUpdate, |wire: UserAddedUpdate| {
                Self::UserAdded {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    inviter_id: wire.inviter_id,
                    is_channel: wire.is_channel,
                }
            }),
            "user_removed" => parse_update!(UserRemovedUpdate, |wire: UserRemovedUpdate| {
                Self::UserRemoved {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    admin_id: wire.admin_id,
                    is_channel: wire.is_channel,
                }
            }),
            "chat_title_changed" => {
                parse_update!(ChatTitleChangedUpdate, |wire: ChatTitleChangedUpdate| {
                    Self::ChatTitleChanged {
                        timestamp: wire.timestamp,
                        chat_id: wire.chat_id,
                        user: wire.user,
                        title: wire.title,
                    }
                })
            }
            _ => Self::Unknown {
                update_type,
                timestamp,
                raw,
            },
        })
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
    /// Optional secret (5-256 chars, `[A-Za-z0-9_-]`).
    /// Sent in the `X-Max-Bot-Api-Secret` header on every webhook request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

// ────────────────────────────────────────────────
// Upload
// ────────────────────────────────────────────────

/// Response from a multipart file upload (image / file).
/// For video/audio the token is returned by `POST /uploads` before the upload.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadResponse {
    /// Ready-to-use attachment token (for image and file types).
    pub token: Option<String>,
    /// Photo tokens returned by MAX image uploads.
    pub photos: Option<PhotoTokens>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UploadType {
    /// Still images (JPG, JPEG, PNG, GIF, TIFF, BMP, HEIC).
    /// NOTE: `photo` was removed from the API — always use `image`.
    Image,
    /// Video files (MP4, MOV, MKV, WEBM, MATROSKA).
    Video,
    /// Audio files (MP3, WAV, M4A, ...).
    Audio,
    /// Any other file type (max 4 GB).
    File,
}

impl UploadType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::File => "file",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadEndpoint {
    pub url: String,
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
// Simple results and video metadata
// ────────────────────────────────────────────────

/// Generic simple JSON result `{"success": true}`.
#[derive(Debug, Clone, Deserialize)]
pub struct SimpleResult {
    pub success: bool,
    pub message: Option<String>,
    pub failed_user_ids: Option<Vec<i64>>,
    pub failed_user_details: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoInfo {
    pub token: String,
    pub urls: Option<VideoUrls>,
    pub thumbnail: Option<PhotoAttachmentPayload>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VideoUrls {
    #[serde(flatten)]
    pub values: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PhotoAttachmentPayload {
    pub url: Option<String>,
    pub token: Option<String>,
    pub photo_id: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

// ────────────────────────────────────────────────
// Chat members and admins
// ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ChatMember {
    pub user_id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
    pub full_avatar_url: Option<String>,
    pub description: Option<String>,
    pub is_owner: Option<bool>,
    pub is_admin: Option<bool>,
    pub join_time: Option<i64>,
    pub permissions: Option<Vec<ChatAdminPermission>>,
    pub last_activity_time: Option<i64>,
    pub last_access_time: Option<i64>,
    pub is_bot: Option<bool>,
    pub alias: Option<String>,
}

impl<'de> Deserialize<'de> for ChatMember {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireChatMember {
            user_id: i64,
            #[serde(default)]
            first_name: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            last_name: Option<String>,
            #[serde(default)]
            username: Option<String>,
            #[serde(default)]
            avatar_url: Option<String>,
            #[serde(default)]
            full_avatar_url: Option<String>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            is_owner: Option<bool>,
            #[serde(default)]
            is_admin: Option<bool>,
            #[serde(default)]
            join_time: Option<i64>,
            #[serde(default)]
            permissions: Option<Vec<ChatAdminPermission>>,
            #[serde(default)]
            last_activity_time: Option<i64>,
            #[serde(default)]
            last_access_time: Option<i64>,
            #[serde(default)]
            is_bot: Option<bool>,
            #[serde(default)]
            alias: Option<String>,
        }

        let wire = WireChatMember::deserialize(deserializer)?;
        let first_name = wire
            .first_name
            .or(wire.name)
            .ok_or_else(|| D::Error::missing_field("first_name"))?;

        Ok(Self {
            user_id: wire.user_id,
            first_name,
            last_name: wire.last_name,
            username: wire.username,
            avatar_url: wire.avatar_url,
            full_avatar_url: wire.full_avatar_url,
            description: wire.description,
            is_owner: wire.is_owner,
            is_admin: wire.is_admin,
            join_time: wire.join_time,
            permissions: wire.permissions,
            last_activity_time: wire.last_activity_time,
            last_access_time: wire.last_access_time,
            is_bot: wire.is_bot,
            alias: wire.alias,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatMembersList {
    pub members: Vec<ChatMember>,
    pub marker: Option<i64>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatAdminPermission {
    ReadAllMessages,
    AddRemoveMembers,
    AddAdmins,
    ChangeChatInfo,
    PinMessage,
    Write,
    CanCall,
    EditLink,
    PostEditDeleteMessage,
    EditMessage,
    DeleteMessage,
    Unknown(String),
}

impl ChatAdminPermission {
    pub fn as_str(&self) -> &str {
        match self {
            Self::ReadAllMessages => "read_all_messages",
            Self::AddRemoveMembers => "add_remove_members",
            Self::AddAdmins => "add_admins",
            Self::ChangeChatInfo => "change_chat_info",
            Self::PinMessage => "pin_message",
            Self::Write => "write",
            Self::CanCall => "can_call",
            Self::EditLink => "edit_link",
            Self::PostEditDeleteMessage => "post_edit_delete_message",
            Self::EditMessage => "edit_message",
            Self::DeleteMessage => "delete_message",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for ChatAdminPermission {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for ChatAdminPermission {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "read_all_messages" => Self::ReadAllMessages,
            "add_remove_members" => Self::AddRemoveMembers,
            "add_admins" => Self::AddAdmins,
            "change_chat_info" => Self::ChangeChatInfo,
            "pin_message" => Self::PinMessage,
            "write" => Self::Write,
            "can_call" => Self::CanCall,
            "edit_link" => Self::EditLink,
            "post_edit_delete_message" => Self::PostEditDeleteMessage,
            "edit_message" => Self::EditMessage,
            "delete_message" => Self::DeleteMessage,
            _ => Self::Unknown(value),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatAdmin {
    pub user_id: i64,
    pub permissions: Vec<ChatAdminPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetChatAdminsBody {
    pub admins: Vec<ChatAdmin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<i64>,
}

/// Body for POST /chats/{chatId}/members.
#[derive(Debug, Clone, Serialize)]
pub struct AddMembersBody {
    pub user_ids: Vec<i64>,
}

/// Body for DELETE /chats/{chatId}/members.
#[derive(Debug, Clone, Serialize)]
pub struct RemoveMemberQuery {
    pub user_id: i64,
}

/// Pinned message info.
#[derive(Debug, Clone, Deserialize)]
pub struct PinnedMessage {
    pub message: Message,
}

/// Body for PUT /chats/{chatId}/pin.
#[derive(Debug, Clone, Serialize)]
pub struct PinMessageBody {
    pub message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

/// Bot command for setMyCommands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCommand {
    pub name: String,
    pub description: String,
}

// ────────────────────────────────────────────────
// Sender actions
// ────────────────────────────────────────────────

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderAction {
    TypingOn,
    SendingImage,
    SendingVideo,
    SendingAudio,
    SendingFile,
    MarkSeen,
    Unknown(String),
}

impl SenderAction {
    pub fn as_str(&self) -> &str {
        match self {
            Self::TypingOn => "typing_on",
            Self::SendingImage => "sending_photo",
            Self::SendingVideo => "sending_video",
            Self::SendingAudio => "sending_audio",
            Self::SendingFile => "sending_file",
            Self::MarkSeen => "mark_seen",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

impl Serialize for SenderAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_string_enum(serializer, self.as_str())
    }
}

impl<'de> Deserialize<'de> for SenderAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize_string_enum(deserializer, |value| match value.as_str() {
            "typing_on" => Self::TypingOn,
            "sending_photo" => Self::SendingImage,
            "sending_video" => Self::SendingVideo,
            "sending_audio" => Self::SendingAudio,
            "sending_file" => Self::SendingFile,
            "mark_seen" => Self::MarkSeen,
            _ => Self::Unknown(value),
        })
    }
}

impl fmt::Display for SenderAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
