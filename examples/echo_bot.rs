//! # English
//!
//! A beginner-friendly echo bot with one command and one fallback handler.
//! `/start` sends a welcome message; every other message is copied back to the
//! same chat. Use this as a starting point for a simple support bot, a token and
//! polling check, or to learn how command handlers take priority over a general
//! message handler. The process runs until Ctrl+C.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example echo_bot`.
//!
//! # Русский
//!
//! Понятный эхо-бот с одной командой и общим обработчиком сообщений. На `/start`
//! бот отправляет приветствие, а любое другое сообщение копирует обратно в тот же
//! чат. Используйте пример как основу простого support-бота, для проверки токена и
//! polling или чтобы понять приоритет command handler над общим message handler.
//! Процесс работает до нажатия Ctrl+C.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example echo_bot`.

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher};

#[tokio::main]
async fn main() -> maxoxide::Result<()> {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env().expect("MAX_BOT_TOKEN must contain a valid bot token");
    let mut dp = Dispatcher::new(bot);

    // /start command
    dp.on_command("/start", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_markdown_to_chat(
                    message.chat_id(),
                    "Привет! Я эхо-бот. Напиши что-нибудь, и я отвечу тем же 🤖",
                )
                .await?;
        }
        Ok(())
    });

    // Mirror every other message
    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(без текста)").to_string();
            ctx.bot.send_text_to_chat(message.chat_id(), text).await?;
        }
        Ok(())
    });

    dp.start_polling().await
}
