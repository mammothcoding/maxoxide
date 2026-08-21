//! # English
//!
//! Starts a minimal standalone Actix Web server with only the MAX `/webhook`
//! scope. Use this when webhook reception is a separate small service rather than
//! part of an existing web application. Register your update handlers on the
//! dispatcher before adapting this skeleton for production.
//!
//! Requires `MAX_BOT_TOKEN`, `WEBHOOK_SECRET`, and feature `webhook-actix`.
//! The sample listens on plain HTTP port 8443; expose it through a trusted HTTPS
//! reverse proxy on public port 443 and create the MAX subscription separately.
//! Run: `cargo run --example webhook_actix --features webhook-actix`.
//!
//! # Русский
//!
//! Запускает минимальный отдельный Actix Web server только со scope `/webhook` для
//! MAX. Подходит, когда приём webhook вынесен в небольшой самостоятельный сервис,
//! а не встроен в существующее web-приложение. Перед production-использованием
//! зарегистрируйте в Dispatcher handlers вашего бота.
//!
//! Нужны `MAX_BOT_TOKEN`, `WEBHOOK_SECRET` и feature `webhook-actix`.
//! Пример слушает обычный HTTP на порту 8443; снаружи нужен доверенный HTTPS reverse
//! proxy на порту 443, а subscription MAX создаётся отдельно.
//! Запуск: `cargo run --example webhook_actix --features webhook-actix`.

use actix_web::{App, HttpServer};
use maxoxide::webhook::{WebhookService, actix_adapter};
use maxoxide::{Bot, Dispatcher, Result};

#[actix_web::main]
async fn main() -> Result<()> {
    let secret = std::env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET is required");
    let service = WebhookService::new(Dispatcher::new(Bot::from_env()?)).secret(secret)?;
    HttpServer::new(move || {
        App::new().service(
            actix_adapter::scope(service.clone(), "/webhook")
                .expect("static webhook path is valid"),
        )
    })
    .bind(("0.0.0.0", 8443))?
    .run()
    .await?;
    Ok(())
}
