//! # English
//!
//! Sends one message containing the most common inline keyboard buttons:
//! callback, text command, link, contact request, location request, and clipboard.
//! Use this when designing a menu and you want to see how button rows look in a
//! real MAX client. This example only sends the keyboard; handle the `confirm`
//! callback with `callback_actions.rs` or your own dispatcher.
//!
//! Requires `MAX_BOT_TOKEN` and a target `MAX_CHAT_ID` where the bot may write.
//! Run: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example keyboard_gallery`.
//!
//! # Русский
//!
//! Отправляет одно сообщение с основными inline-кнопками: callback, текстовая
//! команда, ссылка, запрос контакта, запрос геопозиции и копирование в clipboard.
//! Используйте пример при проектировании меню, чтобы посмотреть расположение кнопок
//! в реальном клиенте MAX. Этот файл только отправляет клавиатуру; callback
//! `confirm` обрабатывается в `callback_actions.rs` или вашем Dispatcher.
//!
//! Нужны `MAX_BOT_TOKEN` и `MAX_CHAT_ID` чата, куда бот может писать.
//! Запуск: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example keyboard_gallery`.

use maxoxide::types::{Button, KeyboardPayload, NewMessageBody};
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let chat_id = std::env::var("MAX_CHAT_ID")
        .expect("MAX_CHAT_ID is required")
        .parse::<i64>()
        .expect("MAX_CHAT_ID is an integer");
    let keyboard = KeyboardPayload {
        buttons: vec![
            vec![
                Button::callback("Confirm", "confirm"),
                Button::message("/help"),
            ],
            vec![Button::link("MAX", "https://max.ru")],
            vec![Button::request_contact("Share contact")],
            vec![Button::request_geo_location("Share location")],
            vec![Button::clipboard("Copy", "copied text")],
        ],
    };
    Bot::from_env()?
        .send_message_to_chat(
            chat_id,
            NewMessageBody::text("Choose an action").with_keyboard(keyboard),
        )
        .await?;
    Ok(())
}
