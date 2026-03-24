//! Keyboard bot — demonstrates inline keyboard buttons and callback handling.
//!
//! Run:
//!   MAX_BOT_TOKEN=your_token cargo run --example keyboard_bot

use maxoxide::types::{AnswerCallbackBody, Button, KeyboardPayload, NewMessageBody, Update};
use maxoxide::{Bot, Context, Dispatcher};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();
    let mut dp = Dispatcher::new(bot);

    // /menu command — shows an inline keyboard
    dp.on_command("/menu", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let keyboard = KeyboardPayload {
                buttons: vec![
                    vec![
                        Button::callback("🔴 Красный", "color:red"),
                        Button::callback("🟢 Зелёный", "color:green"),
                        Button::callback("🔵 Синий", "color:blue"),
                    ],
                    vec![Button::link(
                        "📖 Документация",
                        "https://dev.max.ru/docs-api",
                    )],
                ],
            };

            let body = NewMessageBody::text("Выбери цвет:").with_keyboard(keyboard);
            ctx.bot.send_message(message.chat_id(), body).await?;
        }
        Ok(())
    });

    // Handle "color:red" button
    dp.on_callback_payload("color:red", |ctx: Context| async move {
        if let Update::MessageCallback { callback, .. } = &ctx.update {
            ctx.bot
                .answer_callback(AnswerCallbackBody {
                    callback_id: callback.callback_id.clone(),
                    notification: Some("Ты выбрал красный! 🔴".into()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    });

    // Handle "color:green" button
    dp.on_callback_payload("color:green", |ctx: Context| async move {
        if let Update::MessageCallback { callback, .. } = &ctx.update {
            ctx.bot
                .answer_callback(AnswerCallbackBody {
                    callback_id: callback.callback_id.clone(),
                    notification: Some("Ты выбрал зелёный! 🟢".into()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    });

    // Handle "color:blue" button
    dp.on_callback_payload("color:blue", |ctx: Context| async move {
        if let Update::MessageCallback { callback, .. } = &ctx.update {
            ctx.bot
                .answer_callback(AnswerCallbackBody {
                    callback_id: callback.callback_id.clone(),
                    notification: Some("Ты выбрал синий! 🔵".into()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    });

    // Catch-all for any other callback
    dp.on_callback(|ctx: Context| async move {
        if let Update::MessageCallback { callback, .. } = &ctx.update {
            ctx.bot
                .answer_callback(AnswerCallbackBody {
                    callback_id: callback.callback_id.clone(),
                    notification: Some("Неизвестная кнопка".into()),
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    });

    dp.start_polling().await;
}
