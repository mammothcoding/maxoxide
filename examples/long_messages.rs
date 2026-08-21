//! # English
//!
//! Splits text at MAX's 4000-character limit without cutting a Unicode character
//! in the middle, then sends every part in order. Use this for reports, generated
//! articles, logs, or AI responses that may exceed one MAX message. Set
//! `MAX_LONG_TEXT` to your content; otherwise the example generates long demo text.
//!
//! Requires `MAX_BOT_TOKEN` and `MAX_CHAT_ID` and creates multiple messages.
//! Run: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example long_messages`.
//!
//! # Русский
//!
//! Делит текст по лимиту MAX в 4000 символов, не разрезая Unicode-символ посередине,
//! и отправляет все части по порядку. Подходит для отчётов, сгенерированных статей,
//! logs и AI-ответов, которые могут не поместиться в одно сообщение. Передайте свой
//! текст через `MAX_LONG_TEXT`; без переменной будет создан длинный demo-текст.
//!
//! Нужны `MAX_BOT_TOKEN` и `MAX_CHAT_ID`; пример создаёт несколько сообщений.
//! Запуск: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example long_messages`.

use maxoxide::types::MessageFormat;
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let chat_id = std::env::var("MAX_CHAT_ID")
        .expect("MAX_CHAT_ID is required")
        .parse::<i64>()
        .expect("MAX_CHAT_ID is an integer");
    let text = std::env::var("MAX_LONG_TEXT").unwrap_or_else(|_| "Long message. ".repeat(500));
    let messages = Bot::from_env()?
        .send_long_formatted_text_to_chat(chat_id, text, MessageFormat::Markdown)
        .await?;
    println!("Sent {} parts", messages.len());
    Ok(())
}
