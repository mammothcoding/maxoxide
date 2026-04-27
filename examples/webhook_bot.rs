//! Webhook bot — HTTPS callback alternative to long polling.
//!
//! Requires feature: `webhook`
//!
//! Run:
//!   MAX_BOT_TOKEN=your_token WEBHOOK_SECRET=my_secret \
//!   cargo run --example webhook_bot --features webhook

use maxoxide::types::{SubscribeBody, Update};
use maxoxide::webhook::WebhookServer;
use maxoxide::{Bot, Context, Dispatcher};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let token = std::env::var("MAX_BOT_TOKEN").expect("MAX_BOT_TOKEN not set");
    let secret =
        std::env::var("WEBHOOK_SECRET").unwrap_or_else(|_| "change_me_for_public_webhook".into());
    let webhook_url =
        std::env::var("WEBHOOK_URL").unwrap_or_else(|_| "https://your-domain.com/webhook".into());

    let bot = Bot::new(token);
    let mut dp = Dispatcher::new(bot.clone());

    dp.on_command("/start", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_markdown_to_chat(message.chat_id(), "Привет! Бот запущен через Webhook 🚀")
                .await?;
        }
        Ok(())
    });

    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(без текста)").to_string();
            ctx.bot.send_text_to_chat(message.chat_id(), text).await?;
        }
        Ok(())
    });

    // Register the webhook with Max
    bot.subscribe(SubscribeBody {
        url: webhook_url,
        update_types: None, // receive all update types
        version: None,
        secret: Some(secret.clone()),
    })
    .await
    .expect("Failed to register webhook");

    tracing::info!("Webhook registered, starting server on :8443");

    // Start the axum server behind a public HTTPS endpoint.
    // MAX expects webhook delivery on HTTPS port 443.
    WebhookServer::new(dp)
        .secret(secret)
        .path("/webhook")
        .serve("0.0.0.0:8443")
        .await;
}
