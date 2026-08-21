//! # English
//!
//! Adds MAX webhook processing to an Actix Web application that also serves a
//! normal `/health` route. Use this when an existing Actix service should receive
//! bot updates without running a second HTTP server. Add application handlers to
//! the dispatcher and merge the returned scope with the rest of your routes.
//!
//! Requires `MAX_BOT_TOKEN`, `WEBHOOK_SECRET`, and feature `webhook-actix`.
//! The sample listens on plain HTTP port 8443; terminate trusted HTTPS on public
//! port 443 in a reverse proxy and register the public webhook URL separately.
//! Run: `cargo run --example webhook_actix_integration --features webhook-actix`.
//!
//! # Русский
//!
//! Добавляет обработку MAX webhook в Actix Web application, которое также имеет
//! обычный route `/health`. Используйте этот вариант, когда существующий Actix
//! сервис должен принимать updates бота без запуска второго HTTP server. Добавьте
//! handlers приложения в Dispatcher и подключите полученный scope к остальным routes.
//!
//! Нужны `MAX_BOT_TOKEN`, `WEBHOOK_SECRET` и feature `webhook-actix`.
//! Пример слушает обычный HTTP на порту 8443; доверенный HTTPS на публичном порту
//! 443 должен завершаться в reverse proxy, public webhook URL регистрируется отдельно.
//! Запуск: `cargo run --example webhook_actix_integration --features webhook-actix`.

use actix_web::{App, HttpResponse, HttpServer, web};
use maxoxide::webhook::{WebhookService, actix_adapter};
use maxoxide::{Bot, Dispatcher, Result};

#[actix_web::main]
async fn main() -> Result<()> {
    let service = WebhookService::new(Dispatcher::new(Bot::from_env()?))
        .secret(std::env::var("WEBHOOK_SECRET").expect("WEBHOOK_SECRET is required"))?;
    HttpServer::new(move || {
        App::new()
            .route(
                "/health",
                web::get().to(|| async { HttpResponse::Ok().body("ok") }),
            )
            .service(
                actix_adapter::scope(service.clone(), "/webhook")
                    .expect("static webhook path is valid"),
            )
    })
    .bind(("0.0.0.0", 8443))?
    .run()
    .await?;
    Ok(())
}
