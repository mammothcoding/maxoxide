//! # English
//!
//! Shows how to read text written after an exact command token. Send
//! `/say hello world`, and the bot replies with `hello world`; send only `/say`,
//! and it replies with the fallback text. Use this for search commands, IDs,
//! short forms, and other commands that accept a free-form argument.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example command_arguments`.
//!
//! # Русский
//!
//! Показывает, как читать текст после точного token команды. Отправьте
//! `/say hello world`, и бот ответит `hello world`; для сообщения только `/say`
//! вернётся текст по умолчанию. Такой подход подходит для поиска, передачи ID,
//! коротких форм и других команд со свободным текстовым аргументом.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example command_arguments`.

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut dispatcher = Dispatcher::new(Bot::from_env()?);
    dispatcher.on_command("/say", |context: Context| async move {
        let arguments = context.command_arguments().unwrap_or("Nothing to say");
        if let Update::MessageCreated { message, .. } = &context.update {
            context
                .bot
                .send_text_to_chat(message.chat_id(), arguments)
                .await?;
        }
        Ok(())
    });
    dispatcher.start_polling().await
}
