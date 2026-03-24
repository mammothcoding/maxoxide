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
//!             ctx.bot.send_text(message.chat_id(), text).await?;
//!         }
//!         Ok(())
//!     });
//!
//!     dp.start_polling().await;
//! }
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
pub use dispatcher::{Context, Dispatcher, Filter};
pub use errors::{MaxError, Result};
