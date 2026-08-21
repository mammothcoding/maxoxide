//! # English
//!
//! Merges MAX webhook processing into an Axum application that already has its
//! own routes; `/health` and `/webhook` are served by one router. Use this when
//! your service already runs Axum and MAX updates should share its deployment,
//! middleware, and shutdown lifecycle. Add real dispatcher handlers before using
//! the skeleton in an application.
//!
//! Requires `MAX_BOT_TOKEN`, `WEBHOOK_SECRET`, and feature `webhook-axum`.
//! The sample listens on plain HTTP port 8443; put it behind a trusted HTTPS
//! reverse proxy on public port 443 and register the public URL with MAX separately.
//! Run: `cargo run --example webhook_axum_integration --features webhook-axum`.
//!
//! # Русский
//!
//! Встраивает обработку MAX webhook в Axum-приложение с собственными routes:
//! `/health` и `/webhook` обслуживаются одним Router. Используйте этот вариант,
//! когда сервис уже работает на Axum и updates MAX должны разделять его deployment,
//! middleware и shutdown lifecycle. Перед реальным использованием добавьте в
//! Dispatcher handlers вашего приложения.
//!
//! Нужны `MAX_BOT_TOKEN`, `WEBHOOK_SECRET` и feature `webhook-axum`.
//! Пример слушает обычный HTTP на порту 8443; снаружи нужен доверенный HTTPS reverse
//! proxy на порту 443, а публичный URL регистрируется в MAX отдельно.
//! Запуск: `cargo run --example webhook_axum_integration --features webhook-axum`.

use axum::{Router, routing::get};
use maxoxide::webhook::{WebhookService, axum_adapter};
use maxoxide::{Bot, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let secret = std::env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET is required");
    let webhook = axum_adapter::router(
        WebhookService::new(Dispatcher::new(Bot::from_env()?)).secret(secret)?,
        "/webhook",
    )?;
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(webhook);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8443").await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
