//! # English
//!
//! Validates the signed `window.WebApp.initData` received from a MAX Mini App and
//! extracts authenticated user/chat IDs. Run this logic only on a trusted backend:
//! the bot token must never be shipped to browser code. Use it before accepting a
//! Mini App action, creating a session, or associating browser data with a MAX user.
//!
//! Requires `MAX_BOT_TOKEN` and the exact URL-encoded value in `MAX_INIT_DATA`.
//! Both values are sensitive and should not be written to logs.
//! Run: `MAX_BOT_TOKEN=... MAX_INIT_DATA=... cargo run --example miniapp_validation`.
//!
//! # Русский
//!
//! Проверяет подпись `window.WebApp.initData`, полученного из MAX Mini App, и
//! извлекает подтверждённые user/chat ID. Выполняйте эту логику только на trusted
//! backend: bot token нельзя передавать в browser code. Проверка нужна до обработки
//! действия Mini App, создания session или связывания browser-данных с пользователем MAX.
//!
//! Нужны `MAX_BOT_TOKEN` и точное URL-encoded значение в `MAX_INIT_DATA`.
//! Оба значения чувствительные, их нельзя записывать в logs.
//! Запуск: `MAX_BOT_TOKEN=... MAX_INIT_DATA=... cargo run --example miniapp_validation`.

use maxoxide::miniapp::MiniAppValidator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("MAX_BOT_TOKEN")?;
    let init_data = std::env::var("MAX_INIT_DATA")?;
    let validated = MiniAppValidator::new(token)?.validate(&init_data)?;
    println!(
        "Validated user: {:?}, chat: {:?}",
        validated.user.map(|user| user.id),
        validated.chat.map(|chat| chat.id)
    );
    Ok(())
}
