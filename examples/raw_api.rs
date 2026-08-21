//! # English
//!
//! Demonstrates `Bot::execute`, the low-level escape hatch for a relative MAX Bot
//! API endpoint that maxoxide does not model yet. The sample deliberately calls
//! `/me` and reads raw JSON so the pattern is easy to compare with typed
//! `Bot::get_me`. Prefer typed methods whenever they exist; raw calls require you
//! to validate request and response fields yourself.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example raw_api`.
//!
//! # Русский
//!
//! Показывает `Bot::execute` — низкоуровневый escape hatch для относительного MAX
//! Bot API endpoint, которого ещё нет среди typed methods maxoxide. Для понятности
//! пример вызывает `/me` и читает raw JSON, чтобы его можно было сравнить с
//! `Bot::get_me`. Если typed method уже существует, используйте его: в raw-вызове
//! проверять request и response fields нужно самостоятельно.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example raw_api`.

use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let response = Bot::from_env()?
        .execute::<serde_json::Value>(reqwest::Method::GET, "/me", &[], None)
        .await?;
    println!("Bot ID: {}", response["user_id"]);
    Ok(())
}
