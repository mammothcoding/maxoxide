//! # English
//!
//! Streams a local file to MAX as a generic attachment while reporting progress.
//! The complete file is not loaded into memory, so this is the normal choice for
//! large PDFs, archives, and other files. The example only uploads the file and
//! prints the attachment token length; use that token in `NewAttachment::file`
//! when you also need to send a message.
//!
//! Requires `MAX_BOT_TOKEN` and `MAX_UPLOAD_PATH` pointing to a readable file.
//! Run: `MAX_BOT_TOKEN=... MAX_UPLOAD_PATH=... cargo run --example streaming_upload`.
//!
//! # Русский
//!
//! Потоково загружает локальный файл в MAX как обычное вложение и показывает
//! progress. Файл не читается целиком в память, поэтому это стандартный вариант
//! для больших PDF, архивов и других файлов. Пример только выполняет upload и
//! печатает длину attachment token; для отправки сообщения передайте token в
//! `NewAttachment::file`.
//!
//! Нужны `MAX_BOT_TOKEN` и `MAX_UPLOAD_PATH` с путём к читаемому файлу.
//! Запуск: `MAX_BOT_TOKEN=... MAX_UPLOAD_PATH=... cargo run --example streaming_upload`.

use maxoxide::types::UploadType;
use maxoxide::uploader::UploadOptions;
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::var("MAX_UPLOAD_PATH").expect("MAX_UPLOAD_PATH is required");
    let token = Bot::from_env()?
        .upload_file_with_options(
            UploadType::File,
            &path,
            std::path::Path::new(&path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("upload.bin"),
            "application/octet-stream",
            UploadOptions::default().with_progress(|progress| {
                println!(
                    "{:.0}% ({}/{})",
                    progress.percent, progress.uploaded, progress.total
                );
            }),
        )
        .await?;
    println!("Attachment token received ({} characters)", token.len());
    Ok(())
}
