//! # English
//!
//! Waits for an inline button callback whose payload is exactly `confirm` and
//! acknowledges it with a short MAX notification. Use this after sending a
//! callback button from `keyboard_gallery.rs`, or as a template for confirmation,
//! approval, and pagination actions. MAX expects callbacks to be answered, so the
//! handler calls `answer_callback` instead of sending an unrelated message.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example callback_actions`, then press a
//! button carrying the `confirm` payload. The process runs until Ctrl+C.
//!
//! # Русский
//!
//! Ждёт нажатие inline-кнопки с точным payload `confirm` и подтверждает callback
//! коротким уведомлением MAX. Запустите пример после отправки кнопки из
//! `keyboard_gallery.rs` или используйте как шаблон подтверждения, одобрения и
//! pagination. MAX ожидает ответ на callback, поэтому handler вызывает
//! `answer_callback`, а не отправляет отдельное сообщение.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example callback_actions`, затем нажмите
//! кнопку с payload `confirm`. Процесс работает до нажатия Ctrl+C.

use maxoxide::types::{AnswerCallbackBody, Update};
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut dispatcher = Dispatcher::new(Bot::from_env()?);
    dispatcher.on_callback_payload("confirm", |context: Context| async move {
        if let Update::MessageCallback { callback, .. } = &context.update {
            context
                .bot
                .answer_callback(AnswerCallbackBody {
                    callback_id: callback.callback_id.clone(),
                    notification: Some("Confirmed".into()),
                    message: None,
                })
                .await?;
        }
        Ok(())
    });
    dispatcher.start_polling().await
}
