//! # English
//!
//! The smallest useful long-polling bot: it waits for a new message and sends a
//! fixed reply into the same chat. Start here when you want to verify that a bot
//! token works and understand the basic `Bot` + `Dispatcher` structure before
//! adding commands, filters, or application state. The process runs until Ctrl+C.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example quickstart_bot`.
//!
//! # Русский
//!
//! Минимальный полезный бот на long polling: он ждёт новое сообщение и отправляет
//! фиксированный ответ в тот же чат. Начните с этого примера, чтобы проверить токен
//! и понять базовую связку `Bot` + `Dispatcher` до добавления команд, фильтров и
//! состояния приложения. Процесс работает до нажатия Ctrl+C.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example quickstart_bot`.

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let bot = Bot::from_env()?;
    let mut dispatcher = Dispatcher::new(bot);
    dispatcher.on_message(|context: Context| async move {
        if let Update::MessageCreated { message, .. } = &context.update {
            context
                .bot
                .send_text_to_chat(message.chat_id(), "Hello from maxoxide")
                .await?;
        }
        Ok(())
    });
    dispatcher.start_polling().await
}
