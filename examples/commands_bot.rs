//! # English
//!
//! Routes `/start` and `/help` to separate handlers using exact command tokens.
//! Use this pattern when a bot has a small command menu and unrelated messages
//! should be ignored. `/start extra` still matches `/start`, but `/starter` does
//! not. The bot uses long polling and runs until Ctrl+C.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example commands_bot`.
//!
//! # Русский
//!
//! Направляет `/start` и `/help` в разные handlers по точному token команды.
//! Такой шаблон подходит боту с небольшим меню команд, который должен игнорировать
//! остальные сообщения. `/start extra` совпадёт с `/start`, а `/starter` — нет.
//! Бот использует long polling и работает до нажатия Ctrl+C.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example commands_bot`.

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut dispatcher = Dispatcher::new(Bot::from_env()?);
    dispatcher.on_command("/start", |context: Context| async move {
        if let Update::MessageCreated { message, .. } = &context.update {
            context
                .bot
                .send_text_to_chat(message.chat_id(), "Use /help to see commands")
                .await?;
        }
        Ok(())
    });
    dispatcher.on_command("/help", |context: Context| async move {
        if let Update::MessageCreated { message, .. } = &context.update {
            context
                .bot
                .send_markdown_to_chat(message.chat_id(), "**Commands:** `/start`, `/help`")
                .await?;
        }
        Ok(())
    });
    dispatcher.start_polling().await
}
