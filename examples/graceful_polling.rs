//! # English
//!
//! Configures bounded handler concurrency and graceful dispatcher shutdown.
//! Ctrl+C always stops polling; setting `MAX_STOP_AFTER_SECS` additionally asks
//! the dispatcher to stop after a fixed delay. Use this in services that must
//! finish in-flight handlers before a container, systemd unit, test, or parent
//! process terminates them.
//!
//! Run: `MAX_BOT_TOKEN=... MAX_STOP_AFTER_SECS=30 cargo run --example graceful_polling`.
//! Omit `MAX_STOP_AFTER_SECS` to stop only with Ctrl+C.
//!
//! # Русский
//!
//! Настраивает ограничение параллельных handlers и graceful shutdown Dispatcher.
//! Ctrl+C всегда останавливает polling; переменная `MAX_STOP_AFTER_SECS` дополнительно
//! запрашивает остановку через заданное число секунд. Используйте это в сервисах,
//! которым нужно завершить выполняющиеся handlers перед остановкой container,
//! systemd unit, теста или родительского процесса.
//!
//! Запуск: `MAX_BOT_TOKEN=... MAX_STOP_AFTER_SECS=30 cargo run --example graceful_polling`.
//! Без `MAX_STOP_AFTER_SECS` процесс останавливается только через Ctrl+C.

use std::time::Duration;

use maxoxide::{Bot, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let dispatcher = Dispatcher::new(Bot::from_env()?)
        .max_concurrent_handlers(32)
        .shutdown_timeout(Duration::from_secs(15));
    let shutdown = dispatcher.shutdown_handle();

    if let Ok(seconds) = std::env::var("MAX_STOP_AFTER_SECS") {
        let seconds = seconds
            .parse::<u64>()
            .expect("MAX_STOP_AFTER_SECS is a number");
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(seconds)).await;
            shutdown.shutdown();
        });
    }

    dispatcher.start_polling().await
}
