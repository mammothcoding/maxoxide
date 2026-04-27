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
//! async fn main() {
//!     tracing_subscriber::fmt::init();
//!
//!     let bot = Bot::from_env(); // reads MAX_BOT_TOKEN
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
//!     dp.start_polling().await;
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

pub mod bot;
pub mod dispatcher;
pub mod errors;
pub mod types;
pub mod uploader;

#[cfg(feature = "webhook")]
pub mod webhook;

#[cfg(test)]
mod tests;

// Re-export the most commonly used items at the crate root.
pub use bot::Bot;
pub use dispatcher::{
    Context, Dispatcher, Filter, RawUpdateContext, ScheduledTaskContext, StartContext,
};
pub use errors::{MaxError, Result};
pub use reqwest;
