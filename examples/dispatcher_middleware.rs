//! # English
//!
//! Demonstrates application state shared by handlers and middleware that wraps
//! every matched handler. The middleware measures handling time, while the
//! handler reads a typed `AppName` value from `Context`. Use this structure for
//! database pools, configuration, metrics, authorization, or request tracing.
//! This example logs messages but does not reply to the user.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example dispatcher_middleware`.
//!
//! # Русский
//!
//! Показывает общее состояние приложения и middleware, которое оборачивает каждый
//! найденный handler. Middleware измеряет время обработки, а handler читает typed
//! значение `AppName` из `Context`. Такой шаблон подходит для connection pool БД,
//! конфигурации, метрик, авторизации и tracing. Пример пишет logs, но не отвечает
//! пользователю.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example dispatcher_middleware`.

use std::time::Instant;

use maxoxide::{Bot, Context, Dispatcher, Result};

#[derive(Debug)]
struct AppName(&'static str);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut dispatcher = Dispatcher::new(Bot::from_env()?).with_state(AppName("example"));
    dispatcher.middleware(|context, next| async move {
        let started = Instant::now();
        next.run(context).await?;
        tracing::info!(elapsed_ms = started.elapsed().as_millis(), "update handled");
        Ok(())
    });
    dispatcher.on_message(|context: Context| async move {
        let name = context.state::<AppName>().expect("state registered");
        tracing::info!(application = name.0, "received a message");
        Ok(())
    });
    dispatcher.start_polling().await
}
