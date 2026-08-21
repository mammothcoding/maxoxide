//! # maxoxide
//!
//! An async Rust library for building bots on the [Max messenger](https://max.ru) platform,
//! inspired by [teloxide](https://github.com/teloxide/teloxide).
//!
//! ## Quick start
//!
//! ```no_run
//! use maxoxide::{Bot, Dispatcher, Context};
//! use maxoxide::types::Update;
//!
//! #[tokio::main]
//! async fn main() -> maxoxide::Result<()> {
//!     tracing_subscriber::fmt::init();
//!
//!     let bot = Bot::from_env().expect("valid MAX_BOT_TOKEN");
//!     let mut dp = Dispatcher::new(bot);
//!
//!     // Echo every message back.
//!     dp.on_message(|ctx: Context| async move {
//!         if let Update::MessageCreated { message, .. } = &ctx.update {
//!             let text = message.text().unwrap_or("(no text)").to_string();
//!             ctx.bot.send_text_to_chat(message.chat_id(), text).await?;
//!         }
//!         Ok(())
//!     });
//!
//!     dp.start_polling().await
//! }
//! ```
//!
//! ## Recipient IDs
//!
//! MAX uses two different identifiers that are easy to confuse:
//!
//! - `user_id` is the global MAX ID of a user.
//! - `chat_id` is the ID of a concrete dialog, group, or channel.
//!
//! In a private dialog you often have both:
//!
//! - `message.sender.user_id` is the stable user identifier.
//! - `message.chat_id()` is the identifier of that specific dialog with the bot.
//!
//! Use the chat-based helpers when you already know a dialog/group `chat_id`:
//!
//! ```no_run
//! # use maxoxide::Bot;
//! # async fn example(bot: Bot, chat_id: i64) -> maxoxide::Result<()> {
//! bot.send_text_to_chat(chat_id, "Reply into the existing dialog").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Use the user-based helpers when you only know the global MAX `user_id`:
//!
//! ```no_run
//! # use maxoxide::Bot;
//! # async fn example(bot: Bot, user_id: i64) -> maxoxide::Result<()> {
//! bot.send_text_to_user(user_id, "Send by global user ID").await?;
//! # Ok(())
//! # }
//! ```

/// MAX Bot API client and client configuration.
pub mod bot;
#[cfg(feature = "digital-id")]
pub mod digital_id;
/// Update dispatcher, filters, middleware, and polling support.
pub mod dispatcher;
/// Error types returned by the crate.
pub mod errors;
pub mod miniapp;
mod rate_limit;
/// Serializable MAX Bot API request and response models.
pub mod types;
pub mod uploader;

#[cfg(any(feature = "webhook-axum", feature = "webhook-actix"))]
pub mod webhook;

#[cfg(test)]
mod tests;

// Re-export the most commonly used items at the crate root.
pub use bot::{Bot, BotBuilder, DEFAULT_BASE_URL, RetryPolicy, RussianTlsExt};
pub use dispatcher::{
    Context, Dispatcher, DispatcherShutdown, Filter, Next, RawUpdateContext, ScheduledTaskContext,
    StartContext,
};
pub use errors::{ApiError, MaxError, Result, ValidationError};
pub use rate_limit::{RateLimitConfig, RateLimitKey};
pub use reqwest;
