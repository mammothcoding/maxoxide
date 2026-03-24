//! Echo bot — mirrors every received message back to the sender.
//!
//! Run:
//!   MAX_BOT_TOKEN=your_token cargo run --example echo_bot

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();
    let mut dp = Dispatcher::new(bot);

    // /start command
    dp.on_command("/start", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_markdown(
                    message.chat_id(),
                    "Привет! Я эхо-бот. Напиши что-нибудь, и я отвечу тем же 🤖",
                )
                .await?;
        }
        Ok(())
    });

    // Mirror every other message
    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(без текста)").to_string();
            ctx.bot.send_text(message.chat_id(), text).await?;
        }
        Ok(())
    });

    dp.start_polling().await;
}
