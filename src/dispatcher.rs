use std::{future::Future, sync::Arc, time::Duration};

use tracing::{error, info, warn};

use crate::{bot::Bot, errors::Result, types::Update};

// ────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────

/// Context passed to every handler.
///
/// Provides a reference to the `Bot` and the raw `Update` that triggered it.
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

// ────────────────────────────────────────────────
// Handler trait
// ────────────────────────────────────────────────

/// A boxed async handler function.
pub type HandlerFn = Arc<
    dyn Fn(Context) -> std::pin::Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync,
>;

fn make_handler<H, F>(handler: H) -> HandlerFn
where
    H: Fn(Context) -> F + Send + Sync + 'static,
    F: Future<Output = Result<()>> + Send + 'static,
{
    Arc::new(move |ctx| Box::pin(handler(ctx)))
}

// ────────────────────────────────────────────────
// Dispatcher
// ────────────────────────────────────────────────

/// The dispatcher routes incoming `Update`s to registered handlers.
///
/// Handlers are matched in registration order. The first matching handler wins.
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
///             ctx.bot.send_text(message.chat_id(), message.text().unwrap_or("")).await?;
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
    error_handler: Option<Arc<dyn Fn(crate::errors::MaxError) + Send + Sync>>,
    poll_timeout: u32,
    poll_limit: u32,
}

/// Determines which updates a handler is interested in.
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
    /// Custom predicate.
    Custom(Arc<dyn Fn(&Update) -> bool + Send + Sync>),
}

impl Filter {
    pub(crate) fn matches(&self, update: &Update) -> bool {
        match self {
            Filter::Any => true,
            Filter::Message => matches!(update, Update::MessageCreated { .. }),
            Filter::EditedMessage => matches!(update, Update::MessageEdited { .. }),
            Filter::Callback => matches!(update, Update::MessageCallback { .. }),
            Filter::BotStarted => matches!(update, Update::BotStarted { .. }),
            Filter::BotAdded => matches!(update, Update::BotAdded { .. }),
            Filter::Command(cmd) => {
                if let Update::MessageCreated { message, .. } = update {
                    message
                        .text()
                        .map(|t| t.starts_with(cmd.as_str()))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
            Filter::CallbackPayload(payload) => {
                if let Update::MessageCallback { callback, .. } = update {
                    callback.payload.as_deref() == Some(payload.as_str())
                } else {
                    false
                }
            }
            Filter::Custom(f) => f(update),
        }
    }
}

impl Dispatcher {
    /// Create a new dispatcher for the given bot.
    pub fn new(bot: Bot) -> Self {
        Self {
            bot,
            handlers: Vec::new(),
            error_handler: None,
            poll_timeout: 30,
            poll_limit: 100,
        }
    }

    /// Set a global error handler called when a handler returns an error.
    pub fn on_error<F>(mut self, f: F) -> Self
    where
        F: Fn(crate::errors::MaxError) + Send + Sync + 'static,
    {
        self.error_handler = Some(Arc::new(f));
        self
    }

    /// Set the long-poll timeout in seconds (default: 30, max: 90).
    pub fn poll_timeout(mut self, secs: u32) -> Self {
        self.poll_timeout = secs;
        self
    }

    // ────────────────────────────────────────────────
    // Handler registration
    // ────────────────────────────────────────────────

    /// Register a handler that fires on any update.
    pub fn on<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers.push((Filter::Any, make_handler(handler)));
        self
    }

    /// Register a handler for new messages.
    pub fn on_message<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers.push((Filter::Message, make_handler(handler)));
        self
    }

    /// Register a handler for edited messages.
    pub fn on_edited_message<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers
            .push((Filter::EditedMessage, make_handler(handler)));
        self
    }

    /// Register a handler for inline button callbacks.
    pub fn on_callback<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers
            .push((Filter::Callback, make_handler(handler)));
        self
    }

    /// Register a handler that fires when the bot is started by a user.
    pub fn on_bot_started<H, F>(&mut self, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers
            .push((Filter::BotStarted, make_handler(handler)));
        self
    }

    /// Register a handler for a specific bot command (e.g. `"/start"`).
    pub fn on_command<H, F>(&mut self, command: impl Into<String>, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers
            .push((Filter::Command(command.into()), make_handler(handler)));
        self
    }

    /// Register a handler for a specific callback payload value.
    pub fn on_callback_payload<H, F>(&mut self, payload: impl Into<String>, handler: H) -> &mut Self
    where
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers.push((
            Filter::CallbackPayload(payload.into()),
            make_handler(handler),
        ));
        self
    }

    /// Register a handler with a custom filter predicate.
    pub fn on_filter<P, H, F>(&mut self, predicate: P, handler: H) -> &mut Self
    where
        P: Fn(&Update) -> bool + Send + Sync + 'static,
        H: Fn(Context) -> F + Send + Sync + 'static,
        F: Future<Output = Result<()>> + Send + 'static,
    {
        self.handlers
            .push((Filter::Custom(Arc::new(predicate)), make_handler(handler)));
        self
    }

    // ────────────────────────────────────────────────
    // Dispatching
    // ────────────────────────────────────────────────

    /// Dispatch a single update to all matching handlers.
    pub async fn dispatch(&self, update: Update) {
        for (filter, handler) in &self.handlers {
            if filter.matches(&update) {
                let ctx = Context::new(self.bot.clone(), update.clone());
                if let Err(e) = handler(ctx).await {
                    if let Some(eh) = &self.error_handler {
                        eh(e);
                    } else {
                        error!("Handler error: {e}");
                    }
                }
                // First match wins — break after calling the handler.
                break;
            }
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

        let timeout = self.poll_timeout;
        let limit = self.poll_limit;
        let bot = self.bot.clone();
        let mut marker: Option<i64> = None;

        loop {
            match bot.get_updates(marker, Some(timeout), Some(limit)).await {
                Ok(resp) => {
                    // Advance the marker so we don't re-receive these updates.
                    if let Some(m) = resp.marker {
                        marker = Some(m);
                    }
                    for update in resp.updates {
                        self.dispatch(update).await;
                    }
                }
                Err(e) => {
                    warn!("Polling error: {e} — retrying in 5 s");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
