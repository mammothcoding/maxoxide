use std::{
    future::Future,
    ops::{BitAnd, BitOr, Not},
    sync::Arc,
    time::Duration,
};

use regex::Regex;
use tracing::{error, info, warn};

use crate::{
    bot::Bot,
    errors::{MaxError, Result},
    types::{AttachmentKind, Message, Update},
};

// ────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────

/// Context passed to every update handler.
///
/// Provides a reference to the `Bot` and the typed `Update` that triggered it.
#[derive(Clone)]
pub struct Context {
    pub bot: Bot,
    pub update: Update,
}

impl Context {
    pub fn new(bot: Bot, update: Update) -> Self {
        Self { bot, update }
    }
}

/// Context passed to `on_start` handlers.
#[derive(Clone)]
pub struct StartContext {
    pub bot: Bot,
}

impl StartContext {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

/// Context passed to scheduled task handlers.
#[derive(Clone)]
pub struct ScheduledTaskContext {
    pub bot: Bot,
}

impl ScheduledTaskContext {
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

/// Context passed to raw update handlers.
#[derive(Clone)]
pub struct RawUpdateContext {
    pub bot: Bot,
    pub raw: serde_json::Value,
}

impl RawUpdateContext {
    pub fn new(bot: Bot, raw: serde_json::Value) -> Self {
        Self { bot, raw }
    }
}

// ────────────────────────────────────────────────
// Handler traits
// ────────────────────────────────────────────────

/// A boxed async typed update handler function.
pub type HandlerFn = Arc<
    dyn Fn(Context) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;

type StartHandlerFn = Arc<
    dyn Fn(StartContext) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

type ScheduledTaskFn = Arc<
    dyn Fn(ScheduledTaskContext) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

type RawUpdateHandlerFn = Arc<
    dyn Fn(RawUpdateContext) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

fn make_handler<H, F>(handler: H) -> HandlerFn
where
    H: Fn(Context) -> F + Send + Sync + 'static,
    F: Future<Output = Result<()>> + Send + 'static,
{
    Arc::new(move |ctx| Box::pin(handler(ctx)))
}

fn make_start_handler<H, F>(handler: H) -> StartHandlerFn
where
    H: Fn(StartContext) -> F + Send + Sync + 'static,
    F: Future<Output = Result<()>> + Send + 'static,
{
    Arc::new(move |ctx| Box::pin(handler(ctx)))
}

fn make_scheduled_task<H, F>(handler: H) -> ScheduledTaskFn
where
    H: Fn(ScheduledTaskContext) -> F + Send + Sync + 'static,
    F: Future<Output = Result<()>> + Send + 'static,
{
    Arc::new(move |ctx| Box::pin(handler(ctx)))
}

fn make_raw_update_handler<H, F>(handler: H) -> RawUpdateHandlerFn
where
    H: Fn(RawUpdateContext) -> F + Send + Sync + 'static,
    F: Future<Output = Result<()>> + Send + 'static,
{
    Arc::new(move |ctx| Box::pin(handler(ctx)))
}

// ────────────────────────────────────────────────
// Dispatcher
// ────────────────────────────────────────────────

/// The dispatcher routes incoming `Update`s to registered handlers.
///
/// Handlers are matched in registration order. The first matching typed handler wins.
///
/// # Example
/// ```no_run
/// use maxoxide::{Bot, Dispatcher, Context, Result};
///
/// #[tokio::main]
/// async fn main() {
///     let bot = Bot::from_env();
///     let mut dp = Dispatcher::new(bot);
///
///     dp.on_message(|ctx: Context| async move {
///         if let maxoxide::types::Update::MessageCreated { message, .. } = &ctx.update {
///             ctx.bot
///                 .send_text_to_chat(message.chat_id(), message.text().unwrap_or(""))
///                 .await?;
///         }
///         Ok(())
///     });
///
///     dp.start_polling().await;
/// }
/// ```
pub struct Dispatcher {
    bot: Bot,
    handlers: Vec<(Filter, HandlerFn)>,
    start_handlers: Vec<StartHandlerFn>,
    raw_update_handlers: Vec<RawUpdateHandlerFn>,
    scheduled_tasks: Vec<(Duration, ScheduledTaskFn)>,
    error_handler: Option<Arc<dyn Fn(MaxError) + Send + Sync>>,
    poll_timeout: u32,
    poll_limit: u32,
}

/// Determines which updates a handler is interested in.
#[non_exhaustive]
#[derive(Clone)]
pub enum Filter {
    /// Fires on any update.
    Any,
    /// Fires only when a new message arrives.
    Message,
    /// Fires only when a message is edited.
    EditedMessage,
    /// Fires only when an inline button is pressed.
    Callback,
    /// Fires when a user starts the bot for the first time.
    BotStarted,
    /// Fires when the bot is added to a chat.
    BotAdded,
    /// Fires when a new message arrives whose text starts with this command.
    Command(String),
    /// Fires when the callback payload equals this string.
    CallbackPayload(String),
    /// Fires when an update carries a message in the given chat.
    Chat(i64),
    /// Fires when an update carries a message from the given sender.
    Sender(i64),
    /// Fires when an update carries a message whose text equals this string.
    TextExact(String),
    /// Fires when an update carries a message whose text contains this string.
    TextContains(String),
    /// Fires when an update carries a message whose text matches this regex.
    TextRegex(Regex),
    /// Fires when an update carries a message with any attachment.
    HasAttachment,
    /// Fires when an update carries a message with a specific attachment type.
    HasAttachmentType(AttachmentKind),
    /// Fires when an update carries a file attachment.
    HasFile,
    /// Fires when an update carries image, video, or audio attachment.
    HasMedia,
    /// Fires on `Update::Unknown`.
    UnknownUpdate,
    /// Logical AND for filters.
    And(Vec<Filter>),
    /// Logical OR for filters.
    Or(Vec<Filter>),
    /// Logical NOT for filters.
    Not(Box<Filter>),
    /// Custom predicate.
    Custom(Arc<dyn Fn(&Update) -> bool + Send + Sync>),
}

impl Filter {
    pub fn any() -> Self {
        Self::Any
    }

    pub fn message() -> Self {
        Self::Message
    }

    pub fn edited_message() -> Self {
        Self::EditedMessage
    }

    pub fn callback() -> Self {
        Self::Callback
    }

    pub fn bot_started() -> Self {
        Self::BotStarted
    }

    pub fn bot_added() -> Self {
        Self::BotAdded
    }

    pub fn command(command: impl Into<String>) -> Self {
        Self::Command(command.into())
    }

    pub fn callback_payload(payload: impl Into<String>) -> Self {
        Self::CallbackPayload(payload.into())
    }

    pub fn chat(chat_id: i64) -> Self {
        Self::Chat(chat_id)
    }

    pub fn sender(user_id: i64) -> Self {
        Self::Sender(user_id)
    }

    pub fn text_exact(text: impl Into<String>) -> Self {
        Self::TextExact(text.into())
    }

    pub fn text_contains(text: impl Into<String>) -> Self {
        Self::TextContains(text.into())
    }

    pub fn text_regex(pattern: &str) -> Result<Self> {
        Regex::new(pattern)
            .map(Self::TextRegex)
            .map_err(|e| MaxError::Api {
                code: 0,
                message: format!("Invalid regex filter: {e}"),
            })
    }

    pub fn has_attachment() -> Self {
        Self::HasAttachment
    }

    pub fn has_attachment_type(kind: AttachmentKind) -> Self {
        Self::HasAttachmentType(kind)
    }

    pub fn has_file() -> Self {
        Self::HasFile
    }

    pub fn has_media() -> Self {
        Self::HasMedia
    }

    pub fn unknown_update() -> Self {
        Self::UnknownUpdate
    }

    pub fn and(self, other: Filter) -> Self {
        match (self, other) {
            (Self::And(mut filters), Self::And(other_filters)) => {
                filters.extend(other_filters);
                Self::And(filters)
            }
            (Self::And(mut filters), other) => {
                filters.push(other);
                Self::And(filters)
            }
            (this, Self::And(mut filters)) => {
                let mut combined = vec![this];
                combined.append(&mut filters);
                Self::And(combined)
            }
            (this, other) => Self::And(vec![this, other]),
        }
    }

    pub fn or(self, other: Filter) -> Self {
        match (self, other) {
            (Self::Or(mut filters), Self::Or(other_filters)) => {
                filters.extend(other_filters);
                Self::Or(filters)
            }
            (Self::Or(mut filters), other) => {
                filters.push(other);
                Self::Or(filters)
            }
            (this, Self::Or(mut filters)) => {
                let mut combined = vec![this];
                combined.append(&mut filters);
                Self::Or(combined)
            }
            (this, other) => Self::Or(vec![this, other]),
        }
    }

    pub fn negate(self) -> Self {
        Self::Not(Box::new(self))
    }

    pub(crate) fn matches(&self, update: &Update) -> bool {
        match self {
            Self::Any => true,
            Self::Message => matches!(update, Update::MessageCreated { .. }),
            Self::EditedMessage => matches!(update, Update::MessageEdited { .. }),
            Self::Callback => matches!(update, Update::MessageCallback { .. }),
            Self::BotStarted => matches!(update, Update::BotStarted { .. }),
            Self::BotAdded => matches!(update, Update::BotAdded { .. }),
            Self::Command(cmd) => {
                if let Update::MessageCreated { message, .. } = update {
                    message
                        .text()
                        .map(|t| t.starts_with(cmd.as_str()))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            Self::CallbackPayload(payload) => {
                if let Update::MessageCallback { callback, .. } = update {
                    callback.payload.as_deref() == Some(payload.as_str())
                } else {
                    false
                }
            }
            Self::Chat(chat_id) => message_from_update(update)
                .map(|message| message.chat_id() == *chat_id)
                .unwrap_or(false),
            Self::Sender(user_id) => message_from_update(update)
                .and_then(Message::sender_user_id)
                .map(|sender_user_id| sender_user_id == *user_id)
                .unwrap_or(false),
            Self::TextExact(text) => message_from_update(update)
                .and_then(Message::text)
                .map(|message_text| message_text == text)
                .unwrap_or(false),
            Self::TextContains(text) => message_from_update(update)
                .and_then(Message::text)
                .map(|message_text| message_text.contains(text))
                .unwrap_or(false),
            Self::TextRegex(regex) => message_from_update(update)
                .and_then(Message::text)
                .map(|message_text| regex.is_match(message_text))
                .unwrap_or(false),
            Self::HasAttachment => message_from_update(update)
                .map(Message::has_attachments)
                .unwrap_or(false),
            Self::HasAttachmentType(kind) => message_has_attachment_kind(update, *kind),
            Self::HasFile => message_has_attachment_kind(update, AttachmentKind::File),
            Self::HasMedia => {
                message_has_attachment_kind(update, AttachmentKind::Image)
                    || message_has_attachment_kind(update, AttachmentKind::Video)
                    || message_has_attachment_kind(update, AttachmentKind::Audio)
            }
            Self::UnknownUpdate => matches!(update, Update::Unknown { .. }),
            Self::And(filters) => filters.iter().all(|filter| filter.matches(update)),
            Self::Or(filters) => filters.iter().any(|filter| filter.matches(update)),
            Self::Not(filter) => !filter.matches(update),
            Self::Custom(f) => f(update),
        }
    }
}

impl BitAnd for Filter {
    type Output = Filter;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl BitOr for Filter {
    type Output = Filter;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
    }
}

impl Not for Filter {
    type Output = Filter;

    fn not(self) -> Self::Output {
        self.negate()
    }
}

fn message_from_update(update: &Update) -> Option<&Message> {
    match update {
        Update::MessageCreated { message, .. } | Update::MessageEdited { message, .. } => {
            Some(message)
        }
        Update::MessageCallback {
            message: Some(message),
            ..
        } => Some(message),
        _ => None,
    }
}

fn message_has_attachment_kind(update: &Update, kind: AttachmentKind) -> bool {
    message_from_update(update)
        .and_then(|message| message.body.attachments.as_ref())
        .map(|attachments| {
            attachments
                .iter()
                .any(|attachment| attachment.kind() == kind)
        })
        .unwrap_or(false)
}

impl Dispatcher {
    /// Create a new dispatcher for the given bot.
    pub fn new(bot: Bot) -> Self {
        Self {
            bot,
            handlers: Vec::new(),
            start_handlers: Vec::new(),
            raw_update_handlers: Vec::new(),
            scheduled_tasks: Vec::new(),
            error_handler: None,
            poll_timeout: 30,
            poll_limit: 100,
        }
    }

    /// Set a global error handler called when a handler returns an error.
    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(MaxError) + Send + Sync + 'static,
    {
        self.error_handler = Some(Arc::new(f));
        self
    }

    /// Set the long-poll timeout in seconds (default: 30, max: 90).
    pub fn poll_timeout(mut self, secs: u32) -> Self {
        self.poll_timeout = secs;
        self
    }

    /// Set the long-poll update limit (default: 100).
    pub fn poll_limit(mut self, limit: u32) -> Self {
        self.poll_limit = limit;
        self
    }

    // ────────────────────────────────────────────────
    // Handler registration
    // ────────────────────────────────────────────────

    /// Register a handler that fires on any typed update.
    pub fn on<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::Any, handler)
    }

    /// Register a handler with an explicit filter.
    pub fn on_update<H, F>(&mut self, filter: Filter, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers.push((filter, make_handler(handler)));
        self
    }

    /// Register a handler for new messages.
    pub fn on_message<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::Message, handler)
    }

    /// Register a handler for edited messages.
    pub fn on_edited_message<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::EditedMessage, handler)
    }

    /// Register a handler for inline button callbacks.
    pub fn on_callback<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::Callback, handler)
    }

    /// Register a handler that fires when the bot is started by a user.
    pub fn on_bot_started<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::BotStarted, handler)
    }

    /// Register a handler for a specific bot command (e.g. `"/start"`).
    pub fn on_command<H, F>(&mut self, command: impl Into<String>, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::Command(command.into()), handler)
    }

    /// Register a handler for a specific callback payload value.
    pub fn on_callback_payload<H, F>(&mut self, payload: impl Into<String>, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::CallbackPayload(payload.into()), handler)
    }

    /// Register a handler with a custom filter predicate.
    pub fn on_filter<P, H, F>(&mut self, predicate: P, handler: H) -> &mut Self
    where
        P: Fn(&Update) -> bool + Send + Sync + 'static,
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::Custom(Arc::new(predicate)), handler)
    }

    /// Register a handler that runs once before polling starts.
    pub fn on_start<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(StartContext) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.start_handlers.push(make_start_handler(handler));
        self
    }

    /// Register a periodic task that starts with polling.
    pub fn task<H, F>(&mut self, interval: Duration, handler: H) -> &mut Self
    where
        H: Fn(ScheduledTaskContext) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.scheduled_tasks
            .push((interval, make_scheduled_task(handler)));
        self
    }

    /// Register a handler that receives raw JSON for every incoming update.
    pub fn on_raw_update<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(RawUpdateContext) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.raw_update_handlers
            .push(make_raw_update_handler(handler));
        self
    }

    // ────────────────────────────────────────────────
    // Dispatching
    // ────────────────────────────────────────────────

    /// Dispatch a raw JSON update through raw handlers and typed handlers.
    pub async fn dispatch_raw(&self, raw: serde_json::Value) {
        for handler in &self.raw_update_handlers {
            let ctx = RawUpdateContext::new(self.bot.clone(), raw.clone());
            if let Err(e) = handler(ctx).await {
                self.handle_error(e);
            }
        }

        match serde_json::from_value::<Update>(raw) {
            Ok(update) => self.dispatch(update).await,
            Err(e) => warn!("Failed to parse update JSON: {e}"),
        }
    }

    /// Dispatch a single typed update to the first matching handler.
    pub async fn dispatch(&self, update: Update) {
        for (filter, handler) in &self.handlers {
            if filter.matches(&update) {
                let ctx = Context::new(self.bot.clone(), update.clone());
                if let Err(e) = handler(ctx).await {
                    self.handle_error(e);
                }
                break;
            }
        }
    }

    fn handle_error(&self, error: MaxError) {
        if let Some(error_handler) = &self.error_handler {
            error_handler(error);
        } else {
            error!("Handler error: {error}");
        }
    }

    async fn run_start_handlers(&self) {
        for handler in &self.start_handlers {
            let ctx = StartContext::new(self.bot.clone());
            if let Err(e) = handler(ctx).await {
                self.handle_error(e);
            }
        }
    }

    fn spawn_scheduled_tasks(&self) {
        for (interval, handler) in &self.scheduled_tasks {
            let interval = *interval;
            let handler = handler.clone();
            let bot = self.bot.clone();
            let error_handler = self.error_handler.clone();

            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    let ctx = ScheduledTaskContext::new(bot.clone());
                    if let Err(e) = handler(ctx).await {
                        if let Some(error_handler) = &error_handler {
                            error_handler(e);
                        } else {
                            error!("Scheduled task error: {e}");
                        }
                    }
                }
            });
        }
    }

    // ────────────────────────────────────────────────
    // Long polling
    // ────────────────────────────────────────────────

    /// Start the long-polling loop. This runs until the process is killed.
    ///
    /// On startup it logs the bot's username so you know it's alive.
    pub async fn start_polling(self) {
        let me = match self.bot.get_me().await {
            Ok(u) => u,
            Err(e) => {
                error!("Failed to fetch bot info: {e}");
                return;
            }
        };
        info!(
            "Bot @{} started (long polling)",
            me.username.as_deref().unwrap_or("unknown")
        );

        self.run_start_handlers().await;
        self.spawn_scheduled_tasks();

        let timeout = self.poll_timeout;
        let limit = self.poll_limit;
        let bot = self.bot.clone();
        let mut marker: Option<i64> = None;

        loop {
            match bot
                .get_updates_raw(marker, Some(timeout), Some(limit))
                .await
            {
                Ok(resp) => {
                    if let Some(m) = resp.marker {
                        marker = Some(m);
                    }
                    for update in resp.updates {
                        self.dispatch_raw(update).await;
                    }
                }
                Err(e) => {
                    warn!("Polling error: {e} - retrying in 5 s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
