//! # English
//!
//! Builds one outgoing message with sticker, contact, location, shared link, and
//! inline keyboard attachments. Use it as a catalog when you need to construct
//! these request payloads manually instead of using upload helpers. Replace the
//! placeholder sticker code, phone number, coordinates, and URL with controlled
//! test values before running; an invalid sticker code can make MAX reject the
//! whole message.
//!
//! Requires `MAX_BOT_TOKEN` and `MAX_CHAT_ID` and sends real contact/location data.
//! Run: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example all_attachments`.
//!
//! # Русский
//!
//! Собирает одно исходящее сообщение со sticker, contact, location, shared link и
//! inline keyboard attachments. Используйте как справочник для ручного создания
//! payload, когда upload helpers не нужны. Перед запуском замените placeholder
//! sticker code, телефон, координаты и URL на контролируемые тестовые значения:
//! неверный sticker code может привести к отклонению всего сообщения MAX.
//!
//! Нужны `MAX_BOT_TOKEN` и `MAX_CHAT_ID`; отправляются реальные contact/location.
//! Запуск: `MAX_BOT_TOKEN=... MAX_CHAT_ID=... cargo run --example all_attachments`.

use maxoxide::types::{
    Button, ContactAttachmentPayload, KeyboardPayload, NewAttachment, NewMessageBody,
    ShareAttachmentPayload,
};
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let chat_id = std::env::var("MAX_CHAT_ID")
        .expect("MAX_CHAT_ID is required")
        .parse::<i64>()
        .expect("MAX_CHAT_ID is an integer");
    let body = NewMessageBody::text("Attachment request examples")
        .with_attachment(NewAttachment::sticker("replace-with-sticker-code"))
        .with_attachment(NewAttachment::contact(ContactAttachmentPayload {
            name: Some("Support".into()),
            vcf_phone: Some("79990000000".into()),
            ..Default::default()
        }))
        .with_attachment(NewAttachment::location(55.751244, 37.618423))
        .with_attachment(NewAttachment::share(ShareAttachmentPayload {
            url: Some("https://max.ru".into()),
            token: None,
        }))
        .with_keyboard(KeyboardPayload {
            buttons: vec![vec![Button::callback("OK", "confirm")]],
        });
    Bot::from_env()?.send_message_to_chat(chat_id, body).await?;
    Ok(())
}
