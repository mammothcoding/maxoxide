//! # English
//!
//! Sends a Markdown message and then an HTML message that replies to the first
//! one. Use this when a bot needs bold text, emphasis, code fragments, links, or
//! threaded replies and you want to compare MAX formatting modes. Running the
//! example creates two real messages in the selected chat.
//!
//! Requires `MAX_BOT_TOKEN` and `MAX_CHAT_ID`.
//! Run: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example message_formatting`.
//!
//! # Русский
//!
//! Отправляет Markdown-сообщение, а затем HTML-сообщение как reply на первое.
//! Используйте пример, когда боту нужны жирный текст, выделение, фрагменты кода,
//! ссылки или ответы в цепочке и хочется сравнить режимы форматирования MAX.
//! При запуске в выбранном чате будут созданы два настоящих сообщения.
//!
//! Нужны `MAX_BOT_TOKEN` и `MAX_CHAT_ID`.
//! Запуск: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example message_formatting`.

use maxoxide::types::{MessageFormat, NewMessageBody};
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let chat_id = std::env::var("MAX_CHAT_ID")
        .expect("MAX_CHAT_ID is required")
        .parse::<i64>()
        .expect("MAX_CHAT_ID is an integer");
    let bot = Bot::from_env()?;
    let first = bot
        .send_markdown_to_chat(chat_id, "**Bold**, *emphasis*, and `code`")
        .await?;
    bot.send_message_to_chat(
        chat_id,
        NewMessageBody::text("<strong>HTML reply</strong>")
            .with_format(MessageFormat::Html)
            .with_reply_to(first.message_id()),
    )
    .await?;
    Ok(())
}
