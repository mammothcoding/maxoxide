//! Serializable request, response, attachment, and update models for the MAX Bot API.

use std::{collections::BTreeMap, fmt};

use hmac::{Hmac, Mac};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{DeserializeOwned, Error as DeError},
    ser::SerializeStruct,
};
use sha2::Sha256;

use crate::errors::ValidationError;

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
    /// User's first name or legacy display name returned by MAX.
    pub first_name: String,
    /// User's last name, when available.
    pub last_name: Option<String>,
    /// Public username, when configured.
    pub username: Option<String>,
    /// Whether the account represents a bot.
    pub is_bot: Option<bool>,
    /// Unix timestamp of the user's last activity, when exposed.
    pub last_activity_time: Option<i64>,
    /// Public profile description, when configured.
    pub description: Option<String>,
    /// URL of the standard-size avatar.
    pub avatar_url: Option<String>,
    /// URL of the full-size avatar.
    pub full_avatar_url: Option<String>,
    /// Commands advertised by a bot account.
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

/// Information about the current bot returned by `GET /me`.
#[derive(Debug, Clone, Serialize)]
pub struct BotInfo {
    /// Global MAX identifier of the current bot.
    pub user_id: i64,
    /// Bot's first name or legacy display name.
    pub first_name: String,
    /// Bot's last name, when configured.
    pub last_name: Option<String>,
    /// Public bot username, when configured.
    pub username: Option<String>,
    /// Whether MAX identifies this account as a bot.
    pub is_bot: bool,
    /// Unix timestamp of the bot's last activity, when exposed.
    pub last_activity_time: Option<i64>,
    /// Public bot description.
    pub description: Option<String>,
    /// URL of the standard-size bot avatar.
    pub avatar_url: Option<String>,
    /// URL of the full-size bot avatar.
    pub full_avatar_url: Option<String>,
    /// Commands currently advertised by the bot.
    pub commands: Option<Vec<BotCommand>>,
}

impl BotInfo {
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

impl<'de> Deserialize<'de> for BotInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireBotInfo {
            user_id: i64,
            #[serde(default)]
            first_name: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            last_name: Option<String>,
            #[serde(default)]
            username: Option<String>,
            is_bot: bool,
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

        let wire = WireBotInfo::deserialize(deserializer)?;
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
/// Kind of MAX chat.
pub enum ChatType {
    /// One-to-one dialog.
    Dialog,
    /// Group chat.
    Chat,
    /// Broadcast channel.
    Channel,
    /// Value introduced by MAX after this crate version.
    Unknown(String),
}

impl ChatType {
    /// Returns the MAX wire value.
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
/// Current membership or availability state of a chat.
pub enum ChatStatus {
    /// The chat is active and accessible.
    Active,
    /// The chat was removed.
    Removed,
    /// The current bot or user left the chat.
    Left,
    /// The chat was closed.
    Closed,
    /// The chat is suspended by MAX.
    Suspended,
    /// Value introduced by MAX after this crate version.
    Unknown(String),
}

impl ChatStatus {
    /// Returns the MAX wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "active",
            Self::Removed => "removed",
            Self::Left => "left",
            Self::Closed => "closed",
            Self::Suspended => "suspended",
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
            "suspended" => Self::Suspended,
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
    /// Dialog, group, or channel classification.
    pub r#type: ChatType,
    /// Current chat status, when returned by MAX.
    pub status: Option<ChatStatus>,
    /// Display title of the chat.
    pub title: Option<String>,
    /// Chat icon metadata.
    pub icon: Option<Image>,
    /// Unix timestamp of the most recent chat event.
    pub last_event_time: Option<i64>,
    /// Number of chat participants.
    pub participants_count: Option<i32>,
    /// Global user ID of the chat owner.
    pub owner_id: Option<i64>,
    /// Participant aliases or metadata keyed by MAX identifier.
    pub participants: Option<BTreeMap<String, i64>>,
    /// Whether the chat can be accessed publicly.
    pub is_public: Option<bool>,
    /// Public invitation or access link.
    pub link: Option<String>,
    /// Chat description.
    pub description: Option<String>,
    /// Peer user for a one-to-one dialog.
    pub dialog_with_user: Option<User>,
    /// Total number of messages known to MAX.
    pub messages_count: Option<i64>,
    /// Message identifier associated with a button-created chat.
    pub chat_message_id: Option<String>,
    /// Currently pinned message.
    pub pinned_message: Option<Box<Message>>,
}

/// Image URL returned for a chat or profile icon.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Image {
    /// Absolute image URL.
    pub url: String,
}

/// Body for PATCH /chats/{chatId}.
#[derive(Debug, Clone, Serialize, Default)]
pub struct EditChatBody {
    /// Replacement chat icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<PhotoAttachmentPayload>,
    /// Replacement chat title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Replacement chat description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Message ID to pin as part of the edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    /// Whether members should receive a notification.
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
    /// MAX Markdown formatting.
    #[default]
    Markdown,
    /// HTML formatting supported by MAX.
    Html,
    /// Value introduced by MAX after this crate version.
    Unknown(String),
}

impl MessageFormat {
    /// Returns the MAX wire value.
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
    /// User who sent the message, when available.
    pub sender: Option<User>,
    /// Chat or channel that received the message.
    pub recipient: Recipient,
    /// Message creation timestamp supplied by MAX.
    pub timestamp: i64,
    /// Reply or forwarding metadata.
    pub link: Option<LinkedMessage>,
    /// Message text, attachments, and markup.
    pub body: MessageBody,
    /// Message statistics, when exposed.
    pub stat: Option<MessageStat>,
    /// Public message URL, when available.
    pub url: Option<String>,
    /// User who created a constructed message on behalf of the sender.
    pub constructor: Option<User>,
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
    /// Kind of receiving chat.
    pub chat_type: ChatType,
    /// Optional global MAX user ID for dialog recipients.
    pub user_id: Option<i64>,
    /// Channel post identifier when the message represents a post or comment.
    pub post_id: Option<String>,
}

/// Message payload emitted after an interactive message construction flow.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConstructedMessage {
    /// User represented as the constructed message sender.
    pub sender: Option<User>,
    /// Construction completion timestamp supplied by MAX.
    pub timestamp: i64,
    /// Reply or forwarding metadata.
    pub link: Option<LinkedMessage>,
    /// Constructed text, attachments, and markup.
    pub body: MessageBody,
}

/// Content and identifiers of a received message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageBody {
    /// Unique MAX message identifier.
    pub mid: String,
    /// Monotonic message sequence number within its chat.
    pub seq: i64,
    /// Message text, when present.
    pub text: Option<String>,
    /// Attachments that MAX could deserialize or preserve as unknown values.
    #[serde(default, deserialize_with = "deserialize_attachments_lossy")]
    pub attachments: Option<Vec<Attachment>>,
    /// Structured text markup ranges.
    #[serde(default, deserialize_with = "deserialize_markup_lossy")]
    pub markup: Option<Vec<MarkupElement>>,
}

/// Statistics attached to a message.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MessageStat {
    /// Number of message views, when available.
    pub views: Option<i32>,
}

/// A structured formatting range in received message text.
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum MarkupElement {
    /// Bold text range.
    Strong {
        /// Zero-based start offset in MAX's text representation.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Emphasized text range.
    Emphasized {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Monospaced text range.
    Monospaced {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Hyperlink text range.
    Link {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
        /// Link target URL.
        url: String,
    },
    /// Struck-through text range.
    Strikethrough {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Underlined text range.
    Underline {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Mention of a MAX user.
    UserMention {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
        /// Public link to the mentioned user, when supplied.
        user_link: Option<String>,
        /// Global identifier of the mentioned user, when supplied.
        user_id: Option<i64>,
    },
    /// Heading text range.
    Heading {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Highlighted text range.
    Highlighted {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Quoted text range.
    Quote {
        /// Zero-based start offset.
        from: i32,
        /// Range length.
        length: i32,
    },
    /// Markup kind unknown to this crate version.
    Unknown {
        /// Original MAX markup type.
        r#type: String,
        /// Parsed start offset, when valid.
        from: Option<i32>,
        /// Parsed range length, when valid.
        length: Option<i32>,
        /// Complete JSON object for forward-compatible processing.
        raw: serde_json::Value,
    },
}

impl MarkupElement {
    /// Returns the MAX wire type of this markup element.
    pub fn kind(&self) -> &str {
        match self {
            Self::Strong { .. } => "strong",
            Self::Emphasized { .. } => "emphasized",
            Self::Monospaced { .. } => "monospaced",
            Self::Link { .. } => "link",
            Self::Strikethrough { .. } => "strikethrough",
            Self::Underline { .. } => "underline",
            Self::UserMention { .. } => "user_mention",
            Self::Heading { .. } => "heading",
            Self::Highlighted { .. } => "highlighted",
            Self::Quote { .. } => "quote",
            Self::Unknown { r#type, .. } => r#type.as_str(),
        }
    }
}

impl Serialize for MarkupElement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct BaseMarkup<'a> {
            #[serde(rename = "type")]
            markup_type: &'a str,
            from: i32,
            length: i32,
        }

        #[derive(Serialize)]
        struct LinkMarkup<'a> {
            #[serde(rename = "type")]
            markup_type: &'a str,
            from: i32,
            length: i32,
            url: &'a str,
        }

        #[derive(Serialize)]
        struct UserMentionMarkup<'a> {
            #[serde(rename = "type")]
            markup_type: &'a str,
            from: i32,
            length: i32,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_link: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            user_id: Option<i64>,
        }

        match self {
            Self::Strong { from, length }
            | Self::Emphasized { from, length }
            | Self::Monospaced { from, length }
            | Self::Strikethrough { from, length }
            | Self::Underline { from, length }
            | Self::Heading { from, length }
            | Self::Highlighted { from, length }
            | Self::Quote { from, length } => BaseMarkup {
                markup_type: self.kind(),
                from: *from,
                length: *length,
            }
            .serialize(serializer),
            Self::Link { from, length, url } => LinkMarkup {
                markup_type: "link",
                from: *from,
                length: *length,
                url,
            }
            .serialize(serializer),
            Self::UserMention {
                from,
                length,
                user_link,
                user_id,
            } => UserMentionMarkup {
                markup_type: "user_mention",
                from: *from,
                length: *length,
                user_link: user_link.as_deref(),
                user_id: *user_id,
            }
            .serialize(serializer),
            Self::Unknown { raw, .. } => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for MarkupElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        let markup_type = raw
            .get("type")
            .and_then(|value| value.as_str())
            .map(String::from)
            .ok_or_else(|| D::Error::missing_field("type"))?;
        let from = raw
            .get("from")
            .and_then(|value| value.as_i64())
            .and_then(|value| i32::try_from(value).ok());
        let length = raw
            .get("length")
            .and_then(|value| value.as_i64())
            .and_then(|value| i32::try_from(value).ok());

        let Some(from) = from else {
            return Ok(Self::Unknown {
                r#type: markup_type,
                from,
                length,
                raw,
            });
        };
        let Some(length) = length else {
            return Ok(Self::Unknown {
                r#type: markup_type,
                from: Some(from),
                length,
                raw,
            });
        };

        Ok(match markup_type.as_str() {
            "strong" => Self::Strong { from, length },
            "emphasized" => Self::Emphasized { from, length },
            "monospaced" => Self::Monospaced { from, length },
            "link" => {
                let Some(url) = raw.get("url").and_then(|value| value.as_str()) else {
                    return Ok(Self::Unknown {
                        r#type: markup_type,
                        from: Some(from),
                        length: Some(length),
                        raw,
                    });
                };
                Self::Link {
                    from,
                    length,
                    url: url.to_string(),
                }
            }
            "strikethrough" => Self::Strikethrough { from, length },
            "underline" => Self::Underline { from, length },
            "user_mention" => Self::UserMention {
                from,
                length,
                user_link: raw
                    .get("user_link")
                    .and_then(|value| value.as_str())
                    .map(String::from),
                user_id: raw.get("user_id").and_then(|value| value.as_i64()),
            },
            "heading" => Self::Heading { from, length },
            "highlighted" => Self::Highlighted { from, length },
            "quote" => Self::Quote { from, length },
            _ => Self::Unknown {
                r#type: markup_type,
                from: Some(from),
                length: Some(length),
                raw,
            },
        })
    }
}

fn deserialize_markup_lossy<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<MarkupElement>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Vec<serde_json::Value>>::deserialize(deserializer)?;

    Ok(raw.map(|items| {
        items
            .into_iter()
            .map(|value| {
                serde_json::from_value::<MarkupElement>(value.clone()).unwrap_or_else(|_| {
                    MarkupElement::Unknown {
                        r#type: value
                            .get("type")
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        from: value
                            .get("from")
                            .and_then(|value| value.as_i64())
                            .and_then(|value| i32::try_from(value).ok()),
                        length: value
                            .get("length")
                            .and_then(|value| value.as_i64())
                            .and_then(|value| i32::try_from(value).ok()),
                        raw: value,
                    }
                })
            })
            .collect()
    }))
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

/// Message referenced by a reply or forward link.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinkedMessage {
    /// Link relation type returned by MAX.
    pub r#type: String,
    /// Original message sender, when included.
    pub sender: Option<User>,
    /// Chat containing the linked message.
    pub chat_id: Option<i64>,
    /// Linked message body, when included.
    pub message: Option<MessageBody>,
}

/// Response from GET /messages.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageList {
    /// Messages returned for the requested identifiers.
    pub messages: Vec<Message>,
}

/// A comment attached to a channel post.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommentMessage {
    /// User who created the comment.
    pub sender: Option<User>,
    /// Channel post that received the comment.
    pub recipient: Recipient,
    /// Comment creation timestamp supplied by MAX.
    pub timestamp: i64,
    /// Parent comment metadata for replies.
    pub link: Option<CommentLinkedMessage>,
    /// Comment text and markup.
    pub body: CommentMessageBody,
}

/// Text and markup carried by a comment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommentMessageBody {
    /// Unique MAX comment identifier.
    pub mid: String,
    /// Monotonic sequence number.
    pub seq: i64,
    /// Comment text, when present.
    pub text: Option<String>,
    /// Structured text markup ranges.
    #[serde(default, deserialize_with = "deserialize_markup_lossy")]
    pub markup: Option<Vec<MarkupElement>>,
}

/// A parent comment referenced by a reply.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CommentLinkedMessage {
    /// Link relation type returned by MAX.
    pub r#type: String,
    /// Parent comment sender, when included.
    pub sender: Option<User>,
    /// Chat containing the parent comment.
    pub chat_id: Option<i64>,
    /// Parent comment body, when included.
    pub message: Option<CommentMessageBody>,
}

/// Page returned by the experimental comments API.
#[derive(Debug, Clone, Deserialize)]
pub struct CommentList {
    /// Comments in the current page.
    pub messages: Vec<CommentMessage>,
    /// Pagination marker for the next request.
    pub marker: Option<i64>,
}

/// Filters accepted by `GET /messages/{messageId}/comments`.
#[derive(Debug, Clone, Default)]
pub struct GetCommentsOptions {
    /// Restricts the response to specific comment IDs.
    pub comment_ids: Option<Vec<String>>,
    /// Returns comments before this sequence marker.
    pub before: Option<i64>,
    /// Returns comments after this sequence marker.
    pub after: Option<i64>,
    /// Maximum number of comments to return.
    pub count: Option<u32>,
}

/// Body used to create or edit a comment.
#[derive(Debug, Clone, Default, Serialize)]
pub struct NewCommentBody {
    /// Comment text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Parent-comment reply link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<NewMessageLink>,
    /// Text formatting mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<MessageFormat>,
}

impl NewCommentBody {
    /// Creates a plain-text comment body.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    /// Makes this comment a reply to `comment_id`.
    pub fn with_reply_to(mut self, comment_id: impl Into<String>) -> Self {
        self.link = Some(NewMessageLink {
            r#type: LinkType::Reply,
            mid: comment_id.into(),
        });
        self
    }

    /// Sets the text formatting mode.
    pub fn with_format(mut self, format: MessageFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Validates the documented 4000-character comment limit.
    pub fn validate(&self) -> std::result::Result<(), ValidationError> {
        if self
            .text
            .as_ref()
            .is_some_and(|text| text.chars().count() > 4000)
        {
            return Err(ValidationError::new(
                "text",
                "must not exceed 4000 characters",
            ));
        }
        if self
            .link
            .as_ref()
            .is_some_and(|link| link.r#type != LinkType::Reply)
        {
            return Err(ValidationError::new(
                "link.type",
                "comments only support reply links",
            ));
        }

        Ok(())
    }
}

// ────────────────────────────────────────────────
// Attachments
// ────────────────────────────────────────────────

/// Attachment received from MAX.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum Attachment {
    /// Image attachment.
    Image {
        /// Image metadata.
        payload: MediaPayload,
    },
    /// Video attachment.
    Video {
        /// Video metadata.
        payload: MediaPayload,
    },
    /// Audio attachment.
    Audio {
        /// Audio metadata.
        payload: MediaPayload,
    },
    /// Generic file attachment.
    File {
        /// File metadata.
        payload: FilePayload,
    },
    /// Sticker attachment.
    Sticker {
        /// Sticker metadata.
        payload: StickerPayload,
    },
    /// Inline keyboard attachment.
    InlineKeyboard {
        /// Keyboard rows and buttons.
        payload: KeyboardPayload,
    },
    /// Geographic location attachment.
    Location {
        /// Location coordinates.
        payload: LocationPayload,
    },
    /// Shared contact attachment.
    Contact {
        /// Contact card metadata.
        payload: ContactPayload,
    },
    /// Link preview or shared post attachment.
    Share {
        /// Shared resource metadata.
        payload: SharePayload,
    },
    /// Opaque application data attachment.
    Data {
        /// Application-defined string value.
        data: String,
    },
    /// Attachment kind unknown to this crate version.
    Unknown {
        /// Original MAX attachment type.
        r#type: String,
        /// Original payload field, when present.
        payload: Option<serde_json::Value>,
        /// Complete JSON object for forward-compatible processing.
        raw: serde_json::Value,
    },
}

impl Attachment {
    /// Returns the normalized attachment category.
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
            Self::Share { .. } => AttachmentKind::Share,
            Self::Data { .. } => AttachmentKind::Data,
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
            Self::Share { payload } => serialize_share_attachment(serializer, payload),
            Self::Data { data } => serialize_data_attachment(serializer, data),
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

fn serialize_share_attachment<S>(serializer: S, payload: &SharePayload) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[derive(Serialize)]
    struct ShareAttachmentPayload<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<&'a str>,
    }

    let mut state = serializer.serialize_struct("Attachment", 5)?;
    state.serialize_field("type", "share")?;
    state.serialize_field(
        "payload",
        &ShareAttachmentPayload {
            url: payload.url.as_deref(),
            token: payload.token.as_deref(),
        },
    )?;
    if let Some(title) = &payload.title {
        state.serialize_field("title", title)?;
    }
    if let Some(description) = &payload.description {
        state.serialize_field("description", description)?;
    }
    if let Some(image_url) = &payload.image_url {
        state.serialize_field("image_url", image_url)?;
    }
    state.end()
}

fn serialize_data_attachment<S>(serializer: S, data: &str) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_struct("Attachment", 2)?;
    state.serialize_field("type", "data")?;
    state.serialize_field("data", data)?;
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
            "share" => Ok(Self::Share {
                payload: deserialize_attachment_payload(&raw)?,
            }),
            "data" => {
                #[derive(Deserialize)]
                struct DataPayload {
                    data: String,
                }

                Ok(Self::Data {
                    data: deserialize_attachment_payload::<DataPayload, D::Error>(&raw)?.data,
                })
            }
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
    let value = merged_attachment_payload(raw);

    serde_json::from_value(value).map_err(E::custom)
}

fn merged_attachment_payload(raw: &serde_json::Value) -> serde_json::Value {
    let mut value = raw
        .get("payload")
        .cloned()
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));

    let Some(raw_object) = raw.as_object() else {
        return value;
    };

    let serde_json::Value::Object(value_object) = &mut value else {
        return value;
    };

    for (key, field_value) in raw_object {
        if key != "type" && key != "payload" {
            value_object
                .entry(key.clone())
                .or_insert_with(|| field_value.clone());
        }
    }

    value
}

/// Normalized category of a received attachment.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// Image attachment.
    Image,
    /// Video attachment.
    Video,
    /// Audio attachment.
    Audio,
    /// Generic file attachment.
    File,
    /// Sticker attachment.
    Sticker,
    /// Inline keyboard attachment.
    InlineKeyboard,
    /// Geographic location attachment.
    Location,
    /// Contact card attachment.
    Contact,
    /// Shared link or post attachment.
    Share,
    /// Opaque application data attachment.
    Data,
    /// Attachment not recognized by this crate version.
    Unknown,
}

/// Metadata carried by image, video, and audio attachments.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaPayload {
    /// Download URL, when supplied.
    pub url: Option<String>,
    /// Reusable attachment token, when supplied.
    pub token: Option<String>,
    /// MAX photo identifier for image media.
    pub photo_id: Option<i64>,
    /// Video preview metadata.
    pub thumbnail: Option<VideoThumbnail>,
    /// Media width in pixels.
    pub width: Option<i32>,
    /// Media height in pixels.
    pub height: Option<i32>,
    /// Media duration in seconds.
    pub duration: Option<i32>,
    /// Speech transcription, when generated by MAX.
    pub transcription: Option<String>,
}

/// Preview image associated with a video attachment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoThumbnail {
    /// Absolute preview image URL.
    pub url: Option<String>,
}

/// Metadata carried by a generic file attachment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilePayload {
    /// Download URL, when supplied.
    pub url: Option<String>,
    /// Reusable attachment token, when supplied.
    pub token: Option<String>,
    /// Original filename.
    pub filename: Option<String>,
    /// File size in bytes.
    pub size: Option<i64>,
}

/// Metadata carried by a sticker attachment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StickerPayload {
    /// MAX sticker code.
    pub code: String,
    /// Sticker image URL.
    pub url: Option<String>,
    /// Sticker width in pixels.
    pub width: Option<i32>,
    /// Sticker height in pixels.
    pub height: Option<i32>,
}

/// Coordinates carried by a location attachment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocationPayload {
    /// Latitude in degrees, from `-90` to `90`.
    pub latitude: f64,
    /// Longitude in degrees, from `-180` to `180`.
    pub longitude: f64,
}

/// Metadata carried by a contact attachment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContactPayload {
    /// Display name supplied for the contact.
    pub name: Option<String>,
    /// MAX contact identifier, when available.
    pub contact_id: Option<i64>,
    /// Contact card in VCF form.
    pub vcf_info: Option<String>,
    /// Phone value supplied separately from the VCF card.
    pub vcf_phone: Option<String>,
    /// HMAC protecting the VCF data.
    pub hash: Option<String>,
    /// MAX profile associated with the contact, when available.
    #[serde(alias = "tam_info")]
    pub max_info: Option<User>,
}

impl ContactPayload {
    /// Validates that `hash` matches `vcf_info` for the current bot token.
    pub fn validate_hash(&self, access_token: &str) -> bool {
        let Some(hash) = self.hash.as_deref().filter(|value| !value.is_empty()) else {
            return false;
        };
        let Some(vcf_info) = self.vcf_info.as_deref().filter(|value| !value.is_empty()) else {
            return false;
        };
        if access_token.is_empty() {
            return false;
        }

        let mut mac = Hmac::<Sha256>::new_from_slice(access_token.as_bytes())
            .expect("HMAC accepts keys of any size");
        mac.update(vcf_info.as_bytes());
        let expected = encode_hex(&mac.finalize().into_bytes());

        constant_time_str_eq(hash, &expected)
    }

    /// Extracts phone numbers from the VCF payload returned by MAX.
    pub fn phones_from_vcf(&self) -> Vec<String> {
        let Some(vcf_info) = &self.vcf_info else {
            return Vec::new();
        };

        vcf_info
            .replace("\r\n", "\n")
            .lines()
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                if !name.to_ascii_uppercase().starts_with("TEL") {
                    return None;
                }

                let phone = value
                    .chars()
                    .filter(|ch| ch.is_ascii_digit() || *ch == '+')
                    .collect::<String>();
                (!phone.is_empty()).then_some(phone)
            })
            .collect()
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn constant_time_str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut diff = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        diff |= (left_byte ^ right_byte) as usize;
    }
    diff == 0
}

/// Metadata carried by a shared URL or post attachment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SharePayload {
    /// Shared resource URL.
    pub url: Option<String>,
    /// Shared resource token.
    pub token: Option<String>,
    /// Preview title.
    pub title: Option<String>,
    /// Preview description.
    pub description: Option<String>,
    /// Preview image URL.
    pub image_url: Option<String>,
}

// ────────────────────────────────────────────────
// Keyboard
// ────────────────────────────────────────────────

/// Inline keyboard payload containing rows of buttons.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct KeyboardPayload {
    /// Rows of buttons (max 30 rows, max 7 buttons per row).
    pub buttons: Vec<Vec<Button>>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Interactive button accepted by MAX inline keyboards.
pub enum Button {
    /// Sends a callback event to the bot.
    Callback {
        /// Text displayed on the button.
        text: String,
        /// Application-defined callback payload.
        payload: String,
    },
    /// Opens a URL.
    Link {
        /// Text displayed on the button.
        text: String,
        /// URL opened by the MAX client.
        url: String,
    },
    /// Sends a text message as the user.
    Message {
        /// Text displayed and sent by the button.
        text: String,
    },
    /// Opens a MAX mini app.
    OpenApp {
        /// Text displayed on the button.
        text: String,
        /// Mini App URL or identifier expected by MAX.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        web_app: String,
        /// Application-defined launch payload.
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<String>,
        /// Contact identifier passed to the Mini App.
        #[serde(skip_serializing_if = "Option::is_none")]
        contact_id: Option<i64>,
    },
    /// Copies the payload to the clipboard.
    ///
    /// This button exists in the official Go SDK, but is not listed in the
    /// public REST documentation's button overview yet.
    Clipboard {
        /// Text displayed on the button.
        text: String,
        /// Text copied to the clipboard.
        payload: String,
    },
    /// Requests the user's contact card.
    ///
    /// MAX documents this button, but live tests have observed contact updates
    /// with empty `contact_id` and `vcf_phone`, so phone delivery is not
    /// currently guaranteed on the MAX side.
    RequestContact {
        /// Text displayed on the button.
        text: String,
    },
    /// Requests the user's geo location.
    ///
    /// Live tests have observed MAX returning a `location` attachment with
    /// `latitude` and `longitude` directly on the attachment object.
    RequestGeoLocation {
        /// Text displayed on the button.
        text: String,
        /// Whether MAX should use the quicker location request flow.
        #[serde(skip_serializing_if = "Option::is_none")]
        quick: Option<bool>,
    },
    /// Creates a new chat associated with the current message.
    Chat {
        /// Text displayed on the button.
        text: String,
        /// Title assigned to the created chat.
        chat_title: String,
        /// Optional description assigned to the created chat.
        #[serde(skip_serializing_if = "Option::is_none")]
        chat_description: Option<String>,
        /// Payload returned in the chat-created update.
        #[serde(skip_serializing_if = "Option::is_none")]
        start_payload: Option<String>,
        /// Optional caller-provided request identifier.
        #[serde(skip_serializing_if = "Option::is_none")]
        uuid: Option<i64>,
    },
}

impl Button {
    /// Creates a callback button.
    pub fn callback(text: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Callback {
            text: text.into(),
            payload: payload.into(),
        }
    }

    /// Creates a button that opens a URL.
    pub fn link(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self::Link {
            text: text.into(),
            url: url.into(),
        }
    }

    /// Creates a button that sends its text as a message.
    pub fn message(text: impl Into<String>) -> Self {
        Self::Message { text: text.into() }
    }

    /// Creates a button that opens a Mini App.
    pub fn open_app(text: impl Into<String>, web_app: impl Into<String>) -> Self {
        Self::OpenApp {
            text: text.into(),
            web_app: web_app.into(),
            payload: None,
            contact_id: None,
        }
    }

    /// Creates a Mini App button with an application launch payload.
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

    /// Creates a Mini App button with all optional fields exposed.
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

    /// Creates a button that copies `payload` to the clipboard.
    pub fn clipboard(text: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::Clipboard {
            text: text.into(),
            payload: payload.into(),
        }
    }

    /// Creates a button that asks the user to share a contact.
    pub fn request_contact(text: impl Into<String>) -> Self {
        Self::RequestContact { text: text.into() }
    }

    /// Creates a button that asks the user to share a location.
    pub fn request_geo_location(text: impl Into<String>) -> Self {
        Self::RequestGeoLocation {
            text: text.into(),
            quick: None,
        }
    }

    /// Creates a chat button with a title and default optional fields.
    pub fn chat(text: impl Into<String>, chat_title: impl Into<String>) -> Self {
        Self::Chat {
            text: text.into(),
            chat_title: chat_title.into(),
            chat_description: None,
            start_payload: None,
            uuid: None,
        }
    }

    /// Creates a chat button with all optional fields exposed.
    pub fn chat_full(
        text: impl Into<String>,
        chat_title: impl Into<String>,
        chat_description: Option<String>,
        start_payload: Option<String>,
        uuid: Option<i64>,
    ) -> Self {
        Self::Chat {
            text: text.into(),
            chat_title: chat_title.into(),
            chat_description,
            start_payload,
            uuid,
        }
    }

    fn validate(&self) -> std::result::Result<(), ValidationError> {
        let (text, link) = match self {
            Self::Callback { text, payload } => {
                if payload.is_empty() {
                    return Err(ValidationError::new("button.payload", "value is empty"));
                }
                (text, None)
            }
            Self::Link { text, url } => (text, Some(url.as_str())),
            Self::Message { text }
            | Self::OpenApp { text, .. }
            | Self::Clipboard { text, .. }
            | Self::RequestContact { text }
            | Self::RequestGeoLocation { text, .. }
            | Self::Chat { text, .. } => (text, None),
        };

        if text.is_empty() {
            return Err(ValidationError::new("button.text", "value is empty"));
        }
        if link.is_some_and(|url| url.chars().count() > 2048) {
            return Err(ValidationError::new(
                "button.url",
                "must not exceed 2048 characters",
            ));
        }

        Ok(())
    }
}

impl KeyboardPayload {
    /// Validates documented keyboard row and button limits.
    pub fn validate(&self) -> std::result::Result<(), ValidationError> {
        if self.buttons.len() > 30 {
            return Err(ValidationError::new(
                "keyboard.buttons",
                "must not contain more than 30 rows",
            ));
        }

        for (row_index, row) in self.buttons.iter().enumerate() {
            if row.len() > 7 {
                return Err(ValidationError::new(
                    format!("keyboard.buttons[{row_index}]"),
                    "must not contain more than 7 buttons",
                ));
            }
            let has_wide_button = row.iter().any(|button| {
                matches!(
                    button,
                    Button::Link { .. }
                        | Button::OpenApp { .. }
                        | Button::RequestGeoLocation { .. }
                        | Button::RequestContact { .. }
                )
            });
            if has_wide_button && row.len() > 3 {
                return Err(ValidationError::new(
                    format!("keyboard.buttons[{row_index}]"),
                    "rows with link, open_app, request_geo_location, or request_contact buttons are limited to 3 buttons",
                ));
            }
            for button in row {
                button.validate()?;
            }
        }

        Ok(())
    }
}

// ────────────────────────────────────────────────
// New message body (outgoing)
// ────────────────────────────────────────────────

/// Body for POST /messages.
#[derive(Debug, Clone, Serialize, Default)]
pub struct NewMessageBody {
    /// Message text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Attachments sent with the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<NewAttachment>>,
    /// Reply or forward relationship.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<NewMessageLink>,
    /// Whether recipients should receive a notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
    /// Text formatting mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<MessageFormat>,
}

impl NewMessageBody {
    /// Creates an empty message body.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates a message body containing text.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    /// Creates a text body when `text` is present, otherwise an empty body.
    pub fn text_opt(text: Option<impl Into<String>>) -> Self {
        match text {
            Some(text) => Self::text(text),
            None => Self::empty(),
        }
    }

    /// Appends one attachment.
    pub fn with_attachment(mut self, attachment: NewAttachment) -> Self {
        self.attachments
            .get_or_insert_with(Vec::new)
            .push(attachment);
        self
    }

    /// Appends all attachments from an iterator.
    pub fn with_attachments(
        mut self,
        attachments: impl IntoIterator<Item = NewAttachment>,
    ) -> Self {
        self.attachments
            .get_or_insert_with(Vec::new)
            .extend(attachments);
        self
    }

    /// Appends an inline keyboard attachment.
    pub fn with_keyboard(self, keyboard: KeyboardPayload) -> Self {
        self.with_attachment(NewAttachment::inline_keyboard(keyboard))
    }

    /// Sets the text formatting mode.
    pub fn with_format(mut self, format: MessageFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Controls whether recipients should receive a notification.
    pub fn with_notify(mut self, notify: bool) -> Self {
        self.notify = Some(notify);
        self
    }

    /// Makes the outgoing message a reply to `message_id`.
    pub fn with_reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.link = Some(NewMessageLink {
            r#type: LinkType::Reply,
            mid: message_id.into(),
        });
        self
    }

    /// Forwards the message identified by `message_id`.
    pub fn with_forward_from(mut self, message_id: impl Into<String>) -> Self {
        self.link = Some(NewMessageLink {
            r#type: LinkType::Forward,
            mid: message_id.into(),
        });
        self
    }

    /// Validates the documented message and keyboard constraints.
    pub fn validate(&self) -> std::result::Result<(), ValidationError> {
        if self
            .text
            .as_ref()
            .is_some_and(|text| text.chars().count() > 4000)
        {
            return Err(ValidationError::new(
                "text",
                "must not exceed 4000 characters",
            ));
        }
        if let Some(attachments) = &self.attachments {
            for attachment in attachments {
                attachment.validate()?;
            }
        }

        Ok(())
    }
}

/// Attachment accepted in an outgoing message body.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NewAttachment {
    /// Inline keyboard attachment.
    InlineKeyboard {
        /// Keyboard rows and buttons.
        payload: KeyboardPayload,
    },
    /// Uploaded image or external image URL.
    Image {
        /// Image token, URL, or photo token map.
        payload: ImageAttachmentPayload,
    },
    /// Uploaded video attachment.
    Video {
        /// Upload token issued by MAX.
        payload: UploadedToken,
    },
    /// Uploaded audio attachment.
    Audio {
        /// Upload token issued by MAX.
        payload: UploadedToken,
    },
    /// Uploaded generic file attachment.
    File {
        /// Upload token issued by MAX.
        payload: UploadedToken,
    },
    /// Sticker attachment.
    Sticker {
        /// MAX sticker code.
        payload: StickerAttachmentPayload,
    },
    /// Contact card attachment.
    Contact {
        /// Contact fields accepted by MAX.
        payload: ContactAttachmentPayload,
    },
    /// Geographic location attachment.
    Location {
        /// Latitude in degrees, from `-90` to `90`.
        latitude: f64,
        /// Longitude in degrees, from `-180` to `180`.
        longitude: f64,
    },
    /// Shared URL or uploaded shared resource.
    Share {
        /// Shared resource URL or token.
        payload: ShareAttachmentPayload,
    },
}

impl NewAttachment {
    /// Creates an inline keyboard attachment.
    pub fn inline_keyboard(keyboard: KeyboardPayload) -> Self {
        Self::InlineKeyboard { payload: keyboard }
    }

    /// Creates an image attachment from an upload token.
    pub fn image(token: impl Into<String>) -> Self {
        Self::Image {
            payload: ImageAttachmentPayload::token(token),
        }
    }

    /// Creates an image attachment from an external URL.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            payload: ImageAttachmentPayload::url(url),
        }
    }

    /// Creates an image attachment from MAX's upload photo-token map.
    pub fn image_photos(photos: PhotoTokens) -> Self {
        Self::Image {
            payload: ImageAttachmentPayload::photos(photos),
        }
    }

    /// Creates a video attachment from an upload token.
    pub fn video(token: impl Into<String>) -> Self {
        Self::Video {
            payload: UploadedToken::new(token),
        }
    }

    /// Creates an audio attachment from an upload token.
    pub fn audio(token: impl Into<String>) -> Self {
        Self::Audio {
            payload: UploadedToken::new(token),
        }
    }

    /// Creates a generic file attachment from an upload token.
    pub fn file(token: impl Into<String>) -> Self {
        Self::File {
            payload: UploadedToken::new(token),
        }
    }

    /// Creates a sticker attachment from a MAX sticker code.
    pub fn sticker(code: impl Into<String>) -> Self {
        Self::Sticker {
            payload: StickerAttachmentPayload { code: code.into() },
        }
    }

    /// Creates a contact card attachment.
    pub fn contact(payload: ContactAttachmentPayload) -> Self {
        Self::Contact { payload }
    }

    /// Creates a geographic location attachment.
    pub fn location(latitude: f64, longitude: f64) -> Self {
        Self::Location {
            latitude,
            longitude,
        }
    }

    /// Creates a shared resource attachment.
    pub fn share(payload: ShareAttachmentPayload) -> Self {
        Self::Share { payload }
    }

    fn validate(&self) -> std::result::Result<(), ValidationError> {
        match self {
            Self::InlineKeyboard { payload } => payload.validate(),
            Self::Image { payload }
                if payload.url.is_none() && payload.token.is_none() && payload.photos.is_none() =>
            {
                Err(ValidationError::new(
                    "attachments.image.payload",
                    "url, token, or photos is required",
                ))
            }
            Self::Video { payload } | Self::Audio { payload } | Self::File { payload }
                if payload.token.is_empty() =>
            {
                Err(ValidationError::new(
                    "attachments.payload.token",
                    "value is empty",
                ))
            }
            Self::Sticker { payload } if payload.code.is_empty() => Err(ValidationError::new(
                "attachments.sticker.payload.code",
                "value is empty",
            )),
            Self::Location {
                latitude,
                longitude,
            } if !(-90.0..=90.0).contains(latitude) || !(-180.0..=180.0).contains(longitude) => {
                Err(ValidationError::new(
                    "attachments.location",
                    "coordinates are outside valid latitude/longitude ranges",
                ))
            }
            Self::Share { payload } if payload.url.is_none() && payload.token.is_none() => Err(
                ValidationError::new("attachments.share.payload", "url or token is required"),
            ),
            _ => Ok(()),
        }
    }
}

/// Sticker payload accepted in an outgoing message.
#[derive(Debug, Clone, Serialize)]
pub struct StickerAttachmentPayload {
    /// MAX sticker code.
    pub code: String,
}

/// Contact card accepted in an outgoing message.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ContactAttachmentPayload {
    /// Contact display name.
    pub name: Option<String>,
    /// MAX contact identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<i64>,
    /// Contact details encoded as VCF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcf_info: Option<String>,
    /// Contact phone number supplied separately from VCF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vcf_phone: Option<String>,
}

/// Shared resource accepted in an outgoing message.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ShareAttachmentPayload {
    /// External resource URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Upload token issued by MAX.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

/// One image token returned in MAX's `photos` upload map.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhotoToken {
    /// Ready-to-use image attachment token.
    pub token: String,
    /// Additional upload metadata preserved for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl PhotoToken {
    /// Creates a photo-token entry without additional metadata.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            extra: BTreeMap::new(),
        }
    }
}

/// Image upload tokens keyed by MAX photo size or identifier.
pub type PhotoTokens = BTreeMap<String, PhotoToken>;

/// Image reference accepted in an outgoing message.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageAttachmentPayload {
    /// External image URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Ready-to-use image attachment token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Complete token map returned by an image upload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photos: Option<PhotoTokens>,
}

impl ImageAttachmentPayload {
    /// Creates an image reference from one attachment token.
    pub fn token(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            ..Default::default()
        }
    }

    /// Creates an image reference from an external URL.
    pub fn url(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..Default::default()
        }
    }

    /// Creates an image reference from MAX's photo-token map.
    pub fn photos(photos: PhotoTokens) -> Self {
        Self {
            photos: Some(photos),
            ..Default::default()
        }
    }
}

/// Upload token accepted by video, audio, and file attachments.
#[derive(Debug, Clone, Serialize)]
pub struct UploadedToken {
    /// Ready-to-use attachment token.
    pub token: String,
}

impl UploadedToken {
    /// Wraps a MAX attachment token.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

/// Reply or forward link included in an outgoing message.
#[derive(Debug, Clone, Serialize)]
pub struct NewMessageLink {
    /// Relationship to the referenced message.
    pub r#type: LinkType,
    /// Referenced MAX message identifier.
    pub mid: String,
}

/// Relationship between an outgoing message and an existing message.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkType {
    /// Forward the referenced message.
    Forward,
    /// Reply to the referenced message.
    Reply,
    /// Value introduced by MAX after this crate version.
    Unknown(String),
}

impl LinkType {
    /// Returns the MAX wire value.
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
    /// Whether MAX should omit a generated link preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_link_preview: Option<bool>,
}

impl SendMessageOptions {
    /// Creates query options with explicit link-preview behavior.
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
    /// Typed updates in this polling page.
    pub updates: Vec<Update>,
    /// Marker to pass to the next polling request.
    pub marker: Option<i64>,
}

/// Raw container returned by GET /updates before typed update deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct RawUpdatesResponse {
    /// Raw update JSON objects in this polling page.
    pub updates: Vec<serde_json::Value>,
    /// Marker to pass to the next polling request.
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
    MessageCreated {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Newly created message.
        message: Message,
    },
    /// A message was edited.
    MessageEdited {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Current message representation.
        message: Message,
    },
    /// A message edit event without a message payload.
    MessageEditedMissing {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
    },
    /// A message was deleted.
    MessageRemoved {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Identifier of the removed message.
        message_id: String,
        /// Chat from which the message was removed.
        chat_id: i64,
        /// Global identifier of the user associated with removal.
        user_id: i64,
    },
    /// A user pressed an inline button.
    MessageCallback {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Callback identifier, user, and payload.
        callback: Callback,
        /// Message containing the pressed button, when included.
        message: Option<Message>,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// A mini app requested interactive message construction.
    MessageConstructionRequest {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// User who initiated construction.
        user: User,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
        /// Session identifier used to complete construction.
        session_id: String,
        /// Application-defined launch data.
        data: Option<String>,
        /// Construction input supplied by MAX.
        input: serde_json::Value,
    },
    /// A user completed interactive message construction.
    MessageConstructed {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// User who completed construction.
        user: User,
        /// Construction session identifier.
        session_id: String,
        /// Resulting constructed message.
        message: ConstructedMessage,
    },
    /// The bot was started in a private chat.
    BotStarted {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Private dialog identifier.
        chat_id: i64,
        /// User who started the bot.
        user: User,
        /// Deep-link payload supplied when starting the bot.
        payload: Option<String>,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// The bot was added to a chat.
    BotAdded {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Chat to which the bot was added.
        chat_id: i64,
        /// User associated with the membership change.
        user: User,
        /// Whether the target chat is a channel.
        is_channel: Option<bool>,
    },
    /// The bot was removed from a chat.
    BotRemoved {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Chat from which the bot was removed.
        chat_id: i64,
        /// User associated with the membership change.
        user: User,
        /// Whether the target chat is a channel.
        is_channel: Option<bool>,
    },
    /// A user stopped the bot in a private dialog.
    BotStopped {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Private dialog identifier.
        chat_id: i64,
        /// User who stopped the bot.
        user: User,
        /// Application payload supplied by MAX.
        payload: Option<String>,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// A user cleared the dialog history with the bot.
    DialogCleared {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Private dialog identifier.
        chat_id: i64,
        /// User who cleared the dialog.
        user: User,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// A user muted the dialog with the bot.
    DialogMuted {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Private dialog identifier.
        chat_id: i64,
        /// User who muted the dialog.
        user: User,
        /// Timestamp until which the dialog remains muted.
        muted_until: i64,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// A user unmuted the dialog with the bot.
    DialogUnmuted {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Private dialog identifier.
        chat_id: i64,
        /// User who unmuted the dialog.
        user: User,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// A user removed the dialog with the bot.
    DialogRemoved {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Private dialog identifier.
        chat_id: i64,
        /// User who removed the dialog.
        user: User,
        /// User interface locale reported by MAX.
        user_locale: Option<String>,
    },
    /// A user joined a chat where the bot is a member.
    UserAdded {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Chat the user joined.
        chat_id: i64,
        /// User who joined the chat.
        user: User,
        /// Global identifier of the inviting user.
        inviter_id: Option<i64>,
        /// Whether the target chat is a channel.
        is_channel: Option<bool>,
    },
    /// A user left a chat where the bot is a member.
    UserRemoved {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Chat the user left or was removed from.
        chat_id: i64,
        /// User who left or was removed.
        user: User,
        /// Global identifier of the administrator who removed the user.
        admin_id: Option<i64>,
        /// Whether the target chat is a channel.
        is_channel: Option<bool>,
    },
    /// The bot received a message with a chat title change.
    ChatTitleChanged {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Chat whose title changed.
        chat_id: i64,
        /// User who changed the title.
        user: User,
        /// New chat title.
        title: String,
    },
    /// A chat was created through a `Button::Chat` button.
    MessageChatCreated {
        /// Event timestamp supplied by MAX.
        timestamp: i64,
        /// Newly created chat.
        chat: Chat,
        /// Identifier of the message containing the chat button.
        message_id: String,
        /// Application payload configured on the button.
        start_payload: Option<String>,
    },
    /// A newer or currently unsupported update type.
    Unknown {
        /// Original `update_type` value, when present.
        update_type: Option<String>,
        /// Event timestamp, when present and valid.
        timestamp: Option<i64>,
        /// Complete JSON update for forward-compatible processing.
        raw: serde_json::Value,
    },
}

impl Update {
    /// Returns the timestamp of this update when one was present.
    pub fn timestamp(&self) -> Option<i64> {
        match self {
            Self::MessageCreated { timestamp, .. }
            | Self::MessageEdited { timestamp, .. }
            | Self::MessageEditedMissing { timestamp }
            | Self::MessageRemoved { timestamp, .. }
            | Self::MessageCallback { timestamp, .. }
            | Self::MessageConstructionRequest { timestamp, .. }
            | Self::MessageConstructed { timestamp, .. }
            | Self::BotStarted { timestamp, .. }
            | Self::BotAdded { timestamp, .. }
            | Self::BotRemoved { timestamp, .. }
            | Self::BotStopped { timestamp, .. }
            | Self::DialogCleared { timestamp, .. }
            | Self::DialogMuted { timestamp, .. }
            | Self::DialogUnmuted { timestamp, .. }
            | Self::DialogRemoved { timestamp, .. }
            | Self::UserAdded { timestamp, .. }
            | Self::UserRemoved { timestamp, .. }
            | Self::ChatTitleChanged { timestamp, .. }
            | Self::MessageChatCreated { timestamp, .. } => Some(*timestamp),
            Self::Unknown { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the timestamp or `0` when an unknown update did not include one.
    pub fn timestamp_or_default(&self) -> i64 {
        self.timestamp().unwrap_or_default()
    }

    /// Returns the `chat_id` carried by this update, when the update has one.
    ///
    /// This is useful for maintaining your own chat registry after MAX
    /// deprecated `GET /chats`: store chat IDs from `bot_added`, `bot_started`,
    /// message updates, and remove them on `bot_removed` when appropriate.
    pub fn chat_id(&self) -> Option<i64> {
        match self {
            Self::MessageCreated { message, .. } | Self::MessageEdited { message, .. } => {
                Some(message.chat_id())
            }
            Self::MessageEditedMissing { .. } => None,
            Self::MessageConstructionRequest { .. } | Self::MessageConstructed { .. } => None,
            Self::MessageRemoved { chat_id, .. }
            | Self::BotStarted { chat_id, .. }
            | Self::BotAdded { chat_id, .. }
            | Self::BotRemoved { chat_id, .. }
            | Self::BotStopped { chat_id, .. }
            | Self::DialogCleared { chat_id, .. }
            | Self::DialogMuted { chat_id, .. }
            | Self::DialogUnmuted { chat_id, .. }
            | Self::DialogRemoved { chat_id, .. }
            | Self::UserAdded { chat_id, .. }
            | Self::UserRemoved { chat_id, .. }
            | Self::ChatTitleChanged { chat_id, .. } => Some(*chat_id),
            Self::MessageCallback { message, .. } => message.as_ref().map(Message::chat_id),
            Self::MessageChatCreated { chat, .. } => Some(chat.chat_id),
            Self::Unknown { .. } => None,
        }
    }

    /// Returns the MAX `update_type` wire value, when available.
    pub fn update_type(&self) -> Option<&str> {
        match self {
            Self::MessageCreated { .. } => Some("message_created"),
            Self::MessageEdited { .. } => Some("message_edited"),
            Self::MessageEditedMissing { .. } => Some("message_edited"),
            Self::MessageRemoved { .. } => Some("message_removed"),
            Self::MessageCallback { .. } => Some("message_callback"),
            Self::MessageConstructionRequest { .. } => Some("message_construction_request"),
            Self::MessageConstructed { .. } => Some("message_constructed"),
            Self::BotStarted { .. } => Some("bot_started"),
            Self::BotAdded { .. } => Some("bot_added"),
            Self::BotRemoved { .. } => Some("bot_removed"),
            Self::BotStopped { .. } => Some("bot_stopped"),
            Self::DialogCleared { .. } => Some("dialog_cleared"),
            Self::DialogMuted { .. } => Some("dialog_muted"),
            Self::DialogUnmuted { .. } => Some("dialog_unmuted"),
            Self::DialogRemoved { .. } => Some("dialog_removed"),
            Self::UserAdded { .. } => Some("user_added"),
            Self::UserRemoved { .. } => Some("user_removed"),
            Self::ChatTitleChanged { .. } => Some("chat_title_changed"),
            Self::MessageChatCreated { .. } => Some("message_chat_created"),
            Self::Unknown { update_type, .. } => update_type.as_deref(),
        }
    }

    /// Returns preserved JSON for an unknown update.
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
        struct MessageEditedUpdate {
            timestamp: i64,
            #[serde(default)]
            message: Option<Message>,
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
        struct MessageConstructionRequestUpdate {
            timestamp: i64,
            user: User,
            #[serde(default)]
            user_locale: Option<String>,
            session_id: String,
            #[serde(default)]
            data: Option<String>,
            input: serde_json::Value,
        }

        #[derive(Deserialize)]
        struct MessageConstructedUpdate {
            timestamp: i64,
            user: User,
            session_id: String,
            message: ConstructedMessage,
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
        struct UserDialogUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            #[serde(default)]
            payload: Option<String>,
            #[serde(default)]
            user_locale: Option<String>,
        }

        #[derive(Deserialize)]
        struct DialogMutedUpdate {
            timestamp: i64,
            chat_id: i64,
            user: User,
            muted_until: i64,
            #[serde(default)]
            user_locale: Option<String>,
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

        #[derive(Deserialize)]
        struct MessageChatCreatedUpdate {
            timestamp: i64,
            chat: Chat,
            message_id: String,
            #[serde(default)]
            start_payload: Option<String>,
        }

        Ok(match kind {
            "message_created" => parse_update!(MessageUpdate, |wire: MessageUpdate| {
                Self::MessageCreated {
                    timestamp: wire.timestamp,
                    message: wire.message,
                }
            }),
            "message_edited" => parse_update!(MessageEditedUpdate, |wire: MessageEditedUpdate| {
                match wire.message {
                    Some(message) => Self::MessageEdited {
                        timestamp: wire.timestamp,
                        message,
                    },
                    None => Self::MessageEditedMissing {
                        timestamp: wire.timestamp,
                    },
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
            "message_construction_request" => parse_update!(
                MessageConstructionRequestUpdate,
                |wire: MessageConstructionRequestUpdate| {
                    Self::MessageConstructionRequest {
                        timestamp: wire.timestamp,
                        user: wire.user,
                        user_locale: wire.user_locale,
                        session_id: wire.session_id,
                        data: wire.data,
                        input: wire.input,
                    }
                }
            ),
            "message_constructed" => {
                parse_update!(
                    MessageConstructedUpdate,
                    |wire: MessageConstructedUpdate| {
                        Self::MessageConstructed {
                            timestamp: wire.timestamp,
                            user: wire.user,
                            session_id: wire.session_id,
                            message: wire.message,
                        }
                    }
                )
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
            "bot_stopped" => parse_update!(UserDialogUpdate, |wire: UserDialogUpdate| {
                Self::BotStopped {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    payload: wire.payload,
                    user_locale: wire.user_locale,
                }
            }),
            "dialog_cleared" => parse_update!(UserDialogUpdate, |wire: UserDialogUpdate| {
                Self::DialogCleared {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    user_locale: wire.user_locale,
                }
            }),
            "dialog_muted" => parse_update!(DialogMutedUpdate, |wire: DialogMutedUpdate| {
                Self::DialogMuted {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    muted_until: wire.muted_until,
                    user_locale: wire.user_locale,
                }
            }),
            "dialog_unmuted" => parse_update!(UserDialogUpdate, |wire: UserDialogUpdate| {
                Self::DialogUnmuted {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    user_locale: wire.user_locale,
                }
            }),
            "dialog_removed" => parse_update!(UserDialogUpdate, |wire: UserDialogUpdate| {
                Self::DialogRemoved {
                    timestamp: wire.timestamp,
                    chat_id: wire.chat_id,
                    user: wire.user,
                    user_locale: wire.user_locale,
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
            "message_chat_created" => {
                parse_update!(
                    MessageChatCreatedUpdate,
                    |wire: MessageChatCreatedUpdate| {
                        Self::MessageChatCreated {
                            timestamp: wire.timestamp,
                            chat: wire.chat,
                            message_id: wire.message_id,
                            start_payload: wire.start_payload,
                        }
                    }
                )
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
    /// Identifier used when answering this callback.
    pub callback_id: String,
    /// User who pressed the button.
    pub user: User,
    /// Application payload configured on the button.
    pub payload: Option<String>,
    /// Callback timestamp supplied by MAX.
    pub timestamp: i64,
}

// ────────────────────────────────────────────────
// Subscriptions (webhook)
// ────────────────────────────────────────────────

/// Active webhook subscription returned by MAX.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Subscription {
    /// HTTPS endpoint receiving updates.
    pub url: String,
    /// Subscription creation timestamp supplied by MAX.
    pub time: i64,
    /// Restricted update types, or all supported updates when absent.
    pub update_types: Option<Vec<String>>,
    /// Bot API version associated with the subscription.
    pub version: Option<String>,
}

/// List of active webhook subscriptions.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionList {
    /// Subscriptions registered for the current bot.
    pub subscriptions: Vec<Subscription>,
}

/// Body used to create a webhook subscription.
#[derive(Debug, Clone, Serialize)]
pub struct SubscribeBody {
    /// HTTPS URL of your bot endpoint (must be port 443, no self-signed certs).
    pub url: String,
    /// Optional list of update types to receive (e.g. `["message_created", "bot_started"]`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_types: Option<Vec<String>>,
    /// Optional Bot API version requested for webhook payloads.
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

/// Kind of file accepted by the MAX upload endpoint.
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
    /// Returns the lowercase query value expected by MAX.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::File => "file",
        }
    }
}

/// Signed upload destination issued by `POST /uploads`.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadEndpoint {
    /// Signed URL to which file data must be uploaded.
    pub url: String,
    /// Pre-issued attachment token, when MAX returns one.
    pub token: Option<String>,
}

// ────────────────────────────────────────────────
// Answer on callback
// ────────────────────────────────────────────────

/// Body for POST /answers.
#[derive(Debug, Clone, Serialize, Default)]
pub struct AnswerCallbackBody {
    /// Identifier received in the callback event.
    pub callback_id: String,
    /// Optional replacement message shown after answering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<NewMessageBody>,
    /// Short notification displayed to the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notification: Option<String>,
}

// ────────────────────────────────────────────────
// Simple results and video metadata
// ────────────────────────────────────────────────

/// Generic simple JSON result `{"success": true}`.
#[derive(Debug, Clone, Deserialize)]
pub struct SimpleResult {
    /// Whether MAX completed the operation successfully.
    pub success: bool,
    /// Additional result message, when supplied.
    pub message: Option<String>,
    /// User IDs for which a bulk operation failed.
    pub failed_user_ids: Option<Vec<i64>>,
    /// Structured failure details returned by MAX.
    pub failed_user_details: Option<Vec<serde_json::Value>>,
}

/// Metadata returned for an uploaded video.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoInfo {
    /// Ready-to-use video attachment token.
    pub token: String,
    /// Available encoded video URLs.
    pub urls: Option<VideoUrls>,
    /// Video thumbnail metadata.
    pub thumbnail: Option<PhotoAttachmentPayload>,
    /// Video width in pixels.
    pub width: Option<i32>,
    /// Video height in pixels.
    pub height: Option<i32>,
    /// Video duration in seconds.
    pub duration: Option<i32>,
}

/// Named video renditions returned by MAX.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VideoUrls {
    /// Rendition metadata keyed by MAX-defined names.
    #[serde(flatten)]
    pub values: BTreeMap<String, serde_json::Value>,
}

/// Photo metadata used for chat icons and video thumbnails.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PhotoAttachmentPayload {
    /// Absolute image URL.
    pub url: Option<String>,
    /// Reusable attachment token.
    pub token: Option<String>,
    /// MAX photo identifier.
    pub photo_id: Option<i64>,
    /// Image width in pixels.
    pub width: Option<i32>,
    /// Image height in pixels.
    pub height: Option<i32>,
    /// Additional fields preserved for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

// ────────────────────────────────────────────────
// Chat members and admins
// ────────────────────────────────────────────────

/// User membership information returned for a chat.
#[derive(Debug, Clone, Serialize)]
pub struct ChatMember {
    /// Global MAX user identifier.
    pub user_id: i64,
    /// Member's first name or legacy display name.
    pub first_name: String,
    /// Member's last name, when available.
    pub last_name: Option<String>,
    /// Public username, when configured.
    pub username: Option<String>,
    /// URL of the standard-size avatar.
    pub avatar_url: Option<String>,
    /// URL of the full-size avatar.
    pub full_avatar_url: Option<String>,
    /// Public profile description.
    pub description: Option<String>,
    /// Whether this member owns the chat.
    pub is_owner: Option<bool>,
    /// Whether this member is an administrator.
    pub is_admin: Option<bool>,
    /// Timestamp at which the member joined.
    pub join_time: Option<i64>,
    /// Administrator permissions granted to the member.
    pub permissions: Option<Vec<ChatAdminPermission>>,
    /// Timestamp of the member's last activity.
    pub last_activity_time: Option<i64>,
    /// Timestamp of the member's last chat access.
    pub last_access_time: Option<i64>,
    /// Whether the member account is a bot.
    pub is_bot: Option<bool>,
    /// Administrator alias displayed in the chat.
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

/// Paginated chat member list.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatMembersList {
    /// Members in the current page.
    pub members: Vec<ChatMember>,
    /// Marker to pass to the next request.
    pub marker: Option<i64>,
}

/// Permission that can be granted to a chat administrator.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatAdminPermission {
    /// Read all chat messages.
    ReadAllMessages,
    /// Add and remove chat members.
    AddRemoveMembers,
    /// Promote or demote administrators.
    AddAdmins,
    /// Change chat title, icon, and description.
    ChangeChatInfo,
    /// Pin and unpin messages.
    PinMessage,
    /// Send messages to the chat or channel.
    Write,
    /// Start calls.
    CanCall,
    /// Edit the public chat link.
    EditLink,
    /// Create, edit, and delete channel posts.
    PostEditDeleteMessage,
    /// Edit messages.
    EditMessage,
    /// Delete messages.
    DeleteMessage,
    /// General edit capability returned by MAX.
    Edit,
    /// General delete capability returned by MAX.
    Delete,
    /// View chat or channel statistics.
    ViewStats,
    /// Value introduced by MAX after this crate version.
    Unknown(String),
}

impl ChatAdminPermission {
    /// Returns the MAX wire value.
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
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::ViewStats => "view_stats",
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
            "edit" => Self::Edit,
            "delete" => Self::Delete,
            "view_stats" => Self::ViewStats,
            _ => Self::Unknown(value),
        })
    }
}

/// Administrator entry used when replacing chat administrators.
#[derive(Debug, Clone, Serialize)]
pub struct ChatAdmin {
    /// Global user identifier of the administrator.
    pub user_id: i64,
    /// Permissions granted to the administrator.
    pub permissions: Vec<ChatAdminPermission>,
    /// Optional administrator alias displayed in the chat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// Body used to replace chat administrators.
#[derive(Debug, Clone, Serialize)]
pub struct SetChatAdminsBody {
    /// Complete administrator list to apply.
    pub admins: Vec<ChatAdmin>,
    /// Optional pagination or synchronization marker expected by MAX.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<i64>,
}

/// Body for POST /chats/{chatId}/members.
#[derive(Debug, Clone, Serialize)]
pub struct AddMembersBody {
    /// Global user identifiers to add.
    pub user_ids: Vec<i64>,
}

/// Body for DELETE /chats/{chatId}/members.
#[derive(Debug, Clone, Serialize)]
pub struct RemoveMemberQuery {
    /// Global user identifier to remove.
    pub user_id: i64,
}

/// Query options for DELETE /chats/{chatId}/members.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct RemoveMemberOptions {
    /// Whether the removed user should also be blocked from rejoining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<bool>,
}

impl RemoveMemberOptions {
    /// Creates options with explicit post-removal blocking behavior.
    pub fn block(block: bool) -> Self {
        Self { block: Some(block) }
    }
}

/// Pinned message info.
#[derive(Debug, Clone, Deserialize)]
pub struct PinnedMessage {
    /// Currently pinned message.
    pub message: Message,
}

/// Body for PUT /chats/{chatId}/pin.
#[derive(Debug, Clone, Serialize)]
pub struct PinMessageBody {
    /// Identifier of the message to pin.
    pub message_id: String,
    /// Whether members should receive a notification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notify: Option<bool>,
}

/// Bot command displayed in the MAX client menu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCommand {
    /// Command name without a leading slash.
    pub name: String,
    /// Command description displayed by MAX, when configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl BotCommand {
    /// Creates a command with a description.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: Some(description.into()),
        }
    }

    /// Creates a command without a description.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }
}

/// Response returned after replacing the current bot command list.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BotCommands {
    /// Commands now configured for the bot.
    #[serde(default)]
    pub commands: Vec<BotCommand>,
}

// ────────────────────────────────────────────────
// Sender actions
// ────────────────────────────────────────────────

/// Activity indicator sent on behalf of the bot.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderAction {
    /// Show that the bot is typing.
    TypingOn,
    /// Show that the bot is sending an image.
    SendingImage,
    /// Show that the bot is sending a video.
    SendingVideo,
    /// Show that the bot is sending audio.
    SendingAudio,
    /// Show that the bot is sending a file.
    SendingFile,
    /// Mark recent messages as seen.
    MarkSeen,
    /// Value introduced by MAX after this crate version.
    Unknown(String),
}

impl SenderAction {
    /// Returns the MAX wire value.
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
