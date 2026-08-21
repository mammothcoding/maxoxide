//! # English
//!
//! Exercises the experimental MAX Digital ID age-verification partner integration.
//! The client has not been verified against the live partner service. Use this only
//! if your organization has an approved Digital ID integration and can validate the
//! request against its partner-issued JSON schema.
//! Digital ID credentials are independent from the bot token. This example performs
//! a real external request and prints the raw response, so use controlled data and
//! do not copy it into production logs.
//!
//! Requires feature `digital-id`, `MAX_DIGITAL_ID_TOKEN`, and a JSON object in
//! `MAX_DIGITAL_ID_PAYLOAD`.
//! Run: `cargo run --example digital_id --features digital-id`.
//!
//! # Русский
//!
//! Демонстрирует experimental partner integration проверки возраста MAX Digital ID.
//! Клиент не проверен на live partner service. Используйте пример только при наличии
//! одобренной интеграции Digital ID и сверяйте запрос с выданной вашей организации
//! JSON-схемой. Credentials Digital ID не связаны с bot token. Пример выполняет реальный
//! внешний запрос и печатает raw response, поэтому нужны контролируемые данные, а
//! результат нельзя переносить в production logs.
//!
//! Нужны feature `digital-id`, `MAX_DIGITAL_ID_TOKEN` и JSON object в
//! `MAX_DIGITAL_ID_PAYLOAD`.
//! Запуск: `cargo run --example digital_id --features digital-id`.

use maxoxide::Result;
use maxoxide::digital_id::DigitalIdClient;

#[tokio::main]
async fn main() -> Result<()> {
    let token = std::env::var("MAX_DIGITAL_ID_TOKEN").expect("MAX_DIGITAL_ID_TOKEN is required");
    let payload =
        std::env::var("MAX_DIGITAL_ID_PAYLOAD").expect("MAX_DIGITAL_ID_PAYLOAD is required");
    let payload: serde_json::Value = serde_json::from_str(&payload)?;
    let response = DigitalIdClient::new(token)?
        .verify_age_raw(&payload)
        .await?;
    println!("Digital ID response: {response}");
    Ok(())
}
