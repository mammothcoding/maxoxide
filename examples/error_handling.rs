//! # English
//!
//! Shows the minimum useful error split for a production command or service:
//! structured errors returned by MAX, HTTP timeouts, and all other failures.
//! Use the same matching style when API status/code needs different behavior
//! from a network outage, for example to retry later or show a clear diagnostic.
//! The example performs only `get_me` and does not send messages.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example error_handling`.
//!
//! # Русский
//!
//! Показывает минимально полезное разделение ошибок для production-команды или
//! сервиса: structured error от MAX, HTTP timeout и остальные сбои. Используйте
//! такой `match`, когда API status/code требует другого поведения, чем проблема
//! сети, например для отложенного retry или понятной диагностики. Пример вызывает
//! только `get_me` и не отправляет сообщения.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example error_handling`.

use maxoxide::{Bot, MaxError, Result};

#[tokio::main]
async fn main() -> Result<()> {
    match Bot::from_env()?.get_me().await {
        Ok(bot) => println!("Authenticated as {}", bot.display_name()),
        Err(MaxError::Api(error)) => {
            eprintln!(
                "MAX returned HTTP {} and code {:?}",
                error.status, error.code
            );
        }
        Err(MaxError::Http(error)) if error.is_timeout() => eprintln!("Request timed out"),
        Err(error) => return Err(error),
    }
    Ok(())
}
