//! # English
//!
//! Uploads an MP4 video in 2 MiB chunks when MAX issues a resumable upload token.
//! Use this for large videos or unstable connections: a failed chunk can be
//! retried without buffering or restarting the complete file. MAX decides whether
//! `Content-Range` mode is available. The example uploads only and does not send
//! the resulting video attachment to a chat.
//!
//! Requires `MAX_BOT_TOKEN` and `MAX_UPLOAD_PATH` pointing to an MP4 file.
//! Run: `MAX_BOT_TOKEN=... MAX_UPLOAD_PATH=... cargo run --example resumable_upload`.
//!
//! # Русский
//!
//! Загружает MP4-видео chunks по 2 МиБ, если MAX выдаёт token для resumable upload.
//! Используйте это для больших видео или нестабильной сети: неудачный chunk можно
//! повторить без буферизации и перезапуска всего файла. Доступность `Content-Range`
//! определяет MAX. Пример только загружает файл и не отправляет полученное video
//! attachment в чат.
//!
//! Нужны `MAX_BOT_TOKEN` и `MAX_UPLOAD_PATH` с путём к MP4-файлу.
//! Запуск: `MAX_BOT_TOKEN=... MAX_UPLOAD_PATH=... cargo run --example resumable_upload`.

use maxoxide::types::UploadType;
use maxoxide::uploader::UploadOptions;
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let path = std::env::var("MAX_UPLOAD_PATH").expect("MAX_UPLOAD_PATH is required");
    let options = UploadOptions::default().with_chunk_size(2 * 1024 * 1024);
    let token = Bot::from_env()?
        .upload_file_with_options(UploadType::Video, &path, "video.mp4", "video/mp4", options)
        .await?;
    println!("Video token received ({} characters)", token.len());
    Ok(())
}
