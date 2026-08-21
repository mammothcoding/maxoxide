use std::{
    any::{Any, TypeId},
    collections::HashMap,
    future::Future,
    ops::{BitAnd, BitOr, Not},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use regex::Regex;
use tokio::{sync::Semaphore, task::JoinSet};
use tokio_util::sync::CancellationToken;
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
    /// Bot client used to answer the update.
    pub bot: Bot,
    /// Parsed update currently being handled.
    pub update: Update,
    data: Arc<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl Context {
    /// Creates a handler context without application state.
    pub fn new(bot: Bot, update: Update) -> Self {
        Self {
            bot,
            update,
            data: Arc::default(),
        }
    }

    fn with_data(
        bot: Bot,
        update: Update,
        data: Arc<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    ) -> Self {
        Self { bot, update, data }
    }

    /// Returns application state registered through [`Dispatcher::with_state`].
    pub fn state<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.data
            .get(&TypeId::of::<T>())
            .cloned()
            .and_then(|value| value.downcast::<T>().ok())
    }

    /// Returns command arguments after the exact command token.
    pub fn command_arguments(&self) -> Option<&str> {
        message_from_update(&self.update)
            .and_then(Message::text)
            .and_then(|text| {
                text.split_once(char::is_whitespace)
                    .map(|(_, args)| args.trim())
            })
            .filter(|args| !args.is_empty())
    }
}

/// Context passed to `on_start` handlers.
#[derive(Clone)]
pub struct StartContext {
    /// Bot client owned by the dispatcher.
    pub bot: Bot,
}

impl StartContext {
    /// Creates a startup handler context.
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

/// Context passed to scheduled task handlers.
#[derive(Clone)]
pub struct ScheduledTaskContext {
    /// Bot client owned by the dispatcher.
    pub bot: Bot,
}

impl ScheduledTaskContext {
    /// Creates a scheduled-task context.
    pub fn new(bot: Bot) -> Self {
        Self { bot }
    }
}

/// Context passed to raw update handlers.
#[derive(Clone)]
pub struct RawUpdateContext {
    /// Bot client used to process the raw update.
    pub bot: Bot,
    /// Update JSON exactly as received from MAX.
    pub raw: serde_json::Value,
}

impl RawUpdateContext {
    /// Creates a raw update context.
    pub fn new(bot: Bot, raw: serde_json::Value) -> Self {
        Self { bot, raw }
    }
}

// ────────────────────────────────────────────────
// Handler traits
// ────────────────────────────────────────────────

/// A boxed async typed update handler function.
pub type HandlerFn =
    Arc<dyn Fn(Context) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

type MiddlewareFn =
    Arc<dyn Fn(Context, Next) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;
type ErrorHandlerFn = Arc<dyn Fn(&MaxError) + Send + Sync>;

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

fn make_middleware<M, F>(middleware: M) -> MiddlewareFn
where
    M: Fn(Context, Next) -> F + Send + Sync + 'static,
    F: Future<Output = Result<()>> + Send + 'static,
{
    Arc::new(move |context, next| Box::pin(middleware(context, next)))
}

/// Remaining middleware chain passed to a dispatcher middleware function.
#[derive(Clone)]
pub struct Next {
    middlewares: Arc<Vec<MiddlewareFn>>,
    handler: HandlerFn,
    index: usize,
}

impl Next {
    /// Continues the middleware chain and eventually invokes the update handler.
    pub async fn run(self, context: Context) -> Result<()> {
        if let Some(middleware) = self.middlewares.get(self.index).cloned() {
            let next = Self {
                middlewares: self.middlewares,
                handler: self.handler,
                index: self.index + 1,
            };
            middleware(context, next).await
        } else {
            (self.handler)(context).await
        }
    }
}

/// Cloneable handle used to stop a running dispatcher.
#[derive(Debug, Clone)]
pub struct DispatcherShutdown {
    token: CancellationToken,
}

impl DispatcherShutdown {
    /// Requests graceful dispatcher shutdown.
    pub fn shutdown(&self) {
        self.token.cancel();
    }

    /// Returns whether shutdown has already been requested.
    pub fn is_shutdown(&self) -> bool {
        self.token.is_cancelled()
    }
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
/// async fn main() -> Result<()> {
///     let bot = Bot::from_env().expect("valid MAX_BOT_TOKEN");
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
///     dp.start_polling().await
/// }
/// ```
pub struct Dispatcher {
    bot: Bot,
    handlers: Vec<(Filter, HandlerFn)>,
    middlewares: Vec<MiddlewareFn>,
    context_data: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    start_handlers: Vec<StartHandlerFn>,
    raw_update_handlers: Vec<RawUpdateHandlerFn>,
    scheduled_tasks: Vec<(Duration, ScheduledTaskFn)>,
    error_handler: Option<ErrorHandlerFn>,
    poll_timeout: u32,
    poll_limit: u32,
    concurrency: Arc<Semaphore>,
    shutdown: CancellationToken,
    shutdown_timeout: Duration,
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
    /// Fires when a mini app requests message construction.
    MessageConstructionRequest,
    /// Fires when interactive message construction completes.
    MessageConstructed,
    /// Fires when a user starts the bot for the first time.
    BotStarted,
    /// Fires when the bot is added to a chat.
    BotAdded,
    /// Fires when the bot is removed from a chat.
    BotRemoved,
    /// Fires when a user stops the bot.
    BotStopped,
    /// Fires when a message is removed.
    MessageRemoved,
    /// Fires when a nullable message-edit event is received without a message.
    MessageEditedMissing,
    /// Fires when a chat title changes.
    ChatTitleChanged,
    /// Fires when a user is added to a chat.
    UserAdded,
    /// Fires when a user is removed from a chat.
    UserRemoved,
    /// Fires when a user clears the bot dialog.
    DialogCleared,
    /// Fires when a user mutes the bot dialog.
    DialogMuted,
    /// Fires when a user unmutes the bot dialog.
    DialogUnmuted,
    /// Fires when a user removes the bot dialog.
    DialogRemoved,
    /// Fires when a chat is created through a chat button.
    MessageChatCreated,
    /// Fires when the first message token exactly matches this command.
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
    /// Matches every update.
    pub fn any() -> Self {
        Self::Any
    }

    /// Matches newly created messages.
    pub fn message() -> Self {
        Self::Message
    }

    /// Matches edited messages.
    pub fn edited_message() -> Self {
        Self::EditedMessage
    }

    /// Matches message callback events.
    pub fn callback() -> Self {
        Self::Callback
    }

    /// Matches Mini App message construction requests.
    pub fn message_construction_request() -> Self {
        Self::MessageConstructionRequest
    }

    /// Matches completed interactive message construction events.
    pub fn message_constructed() -> Self {
        Self::MessageConstructed
    }

    /// Matches events emitted when a user starts the bot.
    pub fn bot_started() -> Self {
        Self::BotStarted
    }

    /// Matches events emitted when the bot is added to a chat.
    pub fn bot_added() -> Self {
        Self::BotAdded
    }

    /// Matches events emitted when the bot is removed from a chat.
    pub fn bot_removed() -> Self {
        Self::BotRemoved
    }

    /// Matches events emitted when a user stops the bot.
    pub fn bot_stopped() -> Self {
        Self::BotStopped
    }

    /// Matches removed-message events.
    pub fn message_removed() -> Self {
        Self::MessageRemoved
    }

    /// Matches message-edit events whose message payload is absent.
    pub fn message_edited_missing() -> Self {
        Self::MessageEditedMissing
    }

    /// Matches chat title changes.
    pub fn chat_title_changed() -> Self {
        Self::ChatTitleChanged
    }

    /// Matches events emitted when a user joins a chat.
    pub fn user_added() -> Self {
        Self::UserAdded
    }

    /// Matches events emitted when a user leaves a chat.
    pub fn user_removed() -> Self {
        Self::UserRemoved
    }

    /// Matches dialog-cleared events.
    pub fn dialog_cleared() -> Self {
        Self::DialogCleared
    }

    /// Matches dialog-muted events.
    pub fn dialog_muted() -> Self {
        Self::DialogMuted
    }

    /// Matches dialog-unmuted events.
    pub fn dialog_unmuted() -> Self {
        Self::DialogUnmuted
    }

    /// Matches dialog-removed events.
    pub fn dialog_removed() -> Self {
        Self::DialogRemoved
    }

    /// Matches chats created through a message chat button.
    pub fn message_chat_created() -> Self {
        Self::MessageChatCreated
    }

    /// Matches an exact command token, excluding any following arguments.
    pub fn command(command: impl Into<String>) -> Self {
        Self::Command(command.into())
    }

    /// Matches a callback carrying the exact payload.
    pub fn callback_payload(payload: impl Into<String>) -> Self {
        Self::CallbackPayload(payload.into())
    }

    /// Matches message updates from the specified chat.
    pub fn chat(chat_id: i64) -> Self {
        Self::Chat(chat_id)
    }

    /// Matches message updates from the specified user.
    pub fn sender(user_id: i64) -> Self {
        Self::Sender(user_id)
    }

    /// Matches a message whose text exactly equals `text`.
    pub fn text_exact(text: impl Into<String>) -> Self {
        Self::TextExact(text.into())
    }

    /// Matches a message whose text contains `text`.
    pub fn text_contains(text: impl Into<String>) -> Self {
        Self::TextContains(text.into())
    }

    /// Compiles a regular expression and matches it against message text.
    pub fn text_regex(pattern: &str) -> Result<Self> {
        Regex::new(pattern).map(Self::TextRegex).map_err(|error| {
            crate::errors::ValidationError::new("pattern", error.to_string()).into()
        })
    }

    /// Matches messages containing at least one attachment.
    pub fn has_attachment() -> Self {
        Self::HasAttachment
    }

    /// Matches messages containing an attachment of `kind`.
    pub fn has_attachment_type(kind: AttachmentKind) -> Self {
        Self::HasAttachmentType(kind)
    }

    /// Matches messages containing a file attachment.
    pub fn has_file() -> Self {
        Self::HasFile
    }

    /// Matches messages containing an image, video, or audio attachment.
    pub fn has_media() -> Self {
        Self::HasMedia
    }

    /// Matches updates not recognized by the current model.
    pub fn unknown_update() -> Self {
        Self::UnknownUpdate
    }

    /// Combines this filter and `other` with logical AND.
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

    /// Combines this filter and `other` with logical OR.
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

    /// Negates this filter.
    pub fn negate(self) -> Self {
        Self::Not(Box::new(self))
    }

    pub(crate) fn matches(&self, update: &Update) -> bool {
        match self {
            Self::Any => true,
            Self::Message => matches!(update, Update::MessageCreated { .. }),
            Self::EditedMessage => matches!(update, Update::MessageEdited { .. }),
            Self::Callback => matches!(update, Update::MessageCallback { .. }),
            Self::MessageConstructionRequest => {
                matches!(update, Update::MessageConstructionRequest { .. })
            }
            Self::MessageConstructed => matches!(update, Update::MessageConstructed { .. }),
            Self::BotStarted => matches!(update, Update::BotStarted { .. }),
            Self::BotAdded => matches!(update, Update::BotAdded { .. }),
            Self::BotRemoved => matches!(update, Update::BotRemoved { .. }),
            Self::BotStopped => matches!(update, Update::BotStopped { .. }),
            Self::MessageRemoved => matches!(update, Update::MessageRemoved { .. }),
            Self::MessageEditedMissing => matches!(update, Update::MessageEditedMissing { .. }),
            Self::ChatTitleChanged => matches!(update, Update::ChatTitleChanged { .. }),
            Self::UserAdded => matches!(update, Update::UserAdded { .. }),
            Self::UserRemoved => matches!(update, Update::UserRemoved { .. }),
            Self::DialogCleared => matches!(update, Update::DialogCleared { .. }),
            Self::DialogMuted => matches!(update, Update::DialogMuted { .. }),
            Self::DialogUnmuted => matches!(update, Update::DialogUnmuted { .. }),
            Self::DialogRemoved => matches!(update, Update::DialogRemoved { .. }),
            Self::MessageChatCreated => matches!(update, Update::MessageChatCreated { .. }),
            Self::Command(cmd) => {
                if let Update::MessageCreated { message, .. } = update {
                    message
                        .text()
                        .is_some_and(|text| command_matches(text, cmd))
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

fn command_matches(text: &str, expected: &str) -> bool {
    let Some(token) = text.split_whitespace().next() else {
        return false;
    };
    if expected.contains('@') {
        token == expected
    } else {
        token.split_once('@').map_or(token, |(command, _)| command) == expected
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
            middlewares: Vec::new(),
            context_data: HashMap::new(),
            start_handlers: Vec::new(),
            raw_update_handlers: Vec::new(),
            scheduled_tasks: Vec::new(),
            error_handler: None,
            poll_timeout: 30,
            poll_limit: 100,
            concurrency: Arc::new(Semaphore::new(64)),
            shutdown: CancellationToken::new(),
            shutdown_timeout: Duration::from_secs(30),
        }
    }

    /// Set a global error handler called when a handler returns an error.
    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(&MaxError) + Send + Sync + 'static,
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

    /// Limits the number of handlers that may execute concurrently.
    pub fn max_concurrent_handlers(mut self, maximum: usize) -> Self {
        self.concurrency = Arc::new(Semaphore::new(maximum.max(1)));
        self
    }

    /// Sets how long polling waits for in-flight handlers during shutdown.
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Registers typed application state available through [`Context::state`].
    pub fn with_state<T: Send + Sync + 'static>(mut self, state: T) -> Self {
        self.context_data.insert(TypeId::of::<T>(), Arc::new(state));
        self
    }

    /// Returns a handle that can gracefully stop this dispatcher after it starts.
    pub fn shutdown_handle(&self) -> DispatcherShutdown {
        DispatcherShutdown {
            token: self.shutdown.clone(),
        }
    }

    /// Registers middleware that wraps every matched typed handler.
    pub fn middleware<M, F>(&mut self, middleware: M) -> &mut Self
    where
        M: Fn(Context, Next) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.middlewares.push(make_middleware(middleware));
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

    /// Registers a handler for message-construction requests.
    pub fn on_message_construction_request<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::MessageConstructionRequest, handler)
    }

    /// Registers a handler for completed constructed messages.
    pub fn on_message_constructed<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::MessageConstructed, handler)
    }

    /// Register a handler that fires when the bot is started by a user.
    pub fn on_bot_started<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::BotStarted, handler)
    }

    /// Register a handler that fires when the bot is added to a chat.
    pub fn on_bot_added<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::BotAdded, handler)
    }

    /// Register a handler that fires when the bot is removed from a chat.
    pub fn on_bot_removed<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::BotRemoved, handler)
    }

    /// Register a handler that fires when a user stops the bot.
    pub fn on_bot_stopped<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::BotStopped, handler)
    }

    /// Register a handler for removed messages.
    pub fn on_message_removed<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::MessageRemoved, handler)
    }

    /// Register a handler for edited-message updates without a message payload.
    pub fn on_message_edited_missing<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::MessageEditedMissing, handler)
    }

    /// Register a handler for chat title changes.
    pub fn on_chat_title_changed<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::ChatTitleChanged, handler)
    }

    /// Register a handler for user-added updates.
    pub fn on_user_added<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::UserAdded, handler)
    }

    /// Register a handler for user-removed updates.
    pub fn on_user_removed<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::UserRemoved, handler)
    }

    /// Register a handler for dialog-cleared updates.
    pub fn on_dialog_cleared<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::DialogCleared, handler)
    }

    /// Register a handler for dialog-muted updates.
    pub fn on_dialog_muted<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::DialogMuted, handler)
    }

    /// Register a handler for dialog-unmuted updates.
    pub fn on_dialog_unmuted<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::DialogUnmuted, handler)
    }

    /// Register a handler for dialog-removed updates.
    pub fn on_dialog_removed<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::DialogRemoved, handler)
    }

    /// Register a handler for chats created through chat buttons.
    pub fn on_message_chat_created<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.on_update(Filter::MessageChatCreated, handler)
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
    pub async fn dispatch_raw(&self, raw: serde_json::Value) -> Result<()> {
        for handler in &self.raw_update_handlers {
            let ctx = RawUpdateContext::new(self.bot.clone(), raw.clone());
            handler(ctx).await?;
        }

        let update = serde_json::from_value::<Update>(raw)?;
        self.dispatch(update).await
    }

    /// Dispatch a single typed update to the first matching handler.
    pub async fn dispatch(&self, update: Update) -> Result<()> {
        for (filter, handler) in &self.handlers {
            if filter.matches(&update) {
                let _permit = self
                    .concurrency
                    .acquire()
                    .await
                    .expect("dispatcher semaphore is never closed");
                let context = Context::with_data(
                    self.bot.clone(),
                    update,
                    Arc::new(self.context_data.clone()),
                );
                let next = Next {
                    middlewares: Arc::new(self.middlewares.clone()),
                    handler: handler.clone(),
                    index: 0,
                };
                return next.run(context).await;
            }
        }

        Ok(())
    }

    fn handle_error(&self, error: &MaxError) {
        if let Some(error_handler) = &self.error_handler {
            error_handler(error);
        } else {
            error!("Handler error: {error}");
        }
    }

    async fn run_start_handlers(&self) -> Result<()> {
        for handler in &self.start_handlers {
            let ctx = StartContext::new(self.bot.clone());
            handler(ctx).await?;
        }
        Ok(())
    }

    fn spawn_scheduled_tasks(&self) {
        for (interval, handler) in &self.scheduled_tasks {
            let interval = *interval;
            let handler = handler.clone();
            let bot = self.bot.clone();
            let error_handler = self.error_handler.clone();
            let shutdown = self.shutdown.clone();

            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = shutdown.cancelled() => break,
                        _ = tokio::time::sleep(interval) => {}
                    }
                    let ctx = ScheduledTaskContext::new(bot.clone());
                    if let Err(e) = handler(ctx).await {
                        if let Some(error_handler) = &error_handler {
                            error_handler(&e);
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

    /// Starts long polling and stops gracefully on Ctrl+C or through a shutdown handle.
    ///
    /// On startup it logs the bot's username so you know it's alive.
    pub async fn start_polling(self) -> Result<()> {
        let me = self.bot.get_me().await?;
        info!(
            "Bot @{} started (long polling)",
            me.username.as_deref().unwrap_or("unknown")
        );

        self.run_start_handlers().await?;
        self.spawn_scheduled_tasks();

        let timeout = self.poll_timeout;
        let limit = self.poll_limit;
        let bot = self.bot.clone();
        let shutdown = self.shutdown.clone();
        let shutdown_timeout = self.shutdown_timeout;
        let dispatcher = Arc::new(self);
        let mut marker: Option<i64> = None;
        let mut handlers = JoinSet::new();
        let ctrl_c = tokio::signal::ctrl_c();
        tokio::pin!(ctrl_c);

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                signal = &mut ctrl_c => {
                    if let Err(error) = signal {
                        warn!("Failed to listen for Ctrl+C: {error}");
                    }
                    shutdown.cancel();
                    break;
                }
                Some(joined) = handlers.join_next(), if !handlers.is_empty() => {
                    match joined {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => dispatcher.handle_error(&error),
                        Err(error) => error!("Dispatcher handler task failed: {error}"),
                    }
                }
                response = bot.get_updates_raw(marker, Some(timeout), Some(limit)) => match response {
                    Ok(resp) => {
                        if let Some(m) = resp.marker {
                            marker = Some(m);
                        }
                        for update in resp.updates {
                            let dispatcher = dispatcher.clone();
                            handlers.spawn(async move { dispatcher.dispatch_raw(update).await });
                        }
                    }
                    Err(error) => {
                        warn!("Polling error: {error} - retrying in 5 s");
                        tokio::select! {
                            _ = shutdown.cancelled() => break,
                            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                        }
                    }
                }
            }
        }

        let drain = async {
            while let Some(joined) = handlers.join_next().await {
                match joined {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => dispatcher.handle_error(&error),
                    Err(error) => error!("Dispatcher handler task failed: {error}"),
                }
            }
        };
        if tokio::time::timeout(shutdown_timeout, drain).await.is_err() {
            handlers.abort_all();
            warn!("Dispatcher shutdown timed out; remaining handlers were aborted");
        }

        Ok(())
    }
}
