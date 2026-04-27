//! Demonstrates upload-and-send helpers for image, video, audio, and file
//! attachments.
//!
//! Run:
//!   MAX_BOT_TOKEN=your_token \
//!   MAX_IMAGE_PATH=./photo.jpg \
//!   MAX_VIDEO_PATH=./clip.mp4 \
//!   MAX_AUDIO_PATH=./track.mp3 \
//!   MAX_FILE_PATH=./report.pdf \
//!   cargo run --example media_bot
//!
//! Optional MIME overrides:
//!   MAX_IMAGE_MIME, MAX_VIDEO_MIME, MAX_AUDIO_MIME, MAX_FILE_MIME

use std::{env, path::Path};

use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[derive(Clone, Copy)]
enum MediaKind {
    Image,
    Video,
    Audio,
    File,
}

impl MediaKind {
    fn path_env(self) -> &'static str {
        match self {
            Self::Image => "MAX_IMAGE_PATH",
            Self::Video => "MAX_VIDEO_PATH",
            Self::Audio => "MAX_AUDIO_PATH",
            Self::File => "MAX_FILE_PATH",
        }
    }

    fn mime_env(self) -> &'static str {
        match self {
            Self::Image => "MAX_IMAGE_MIME",
            Self::Video => "MAX_VIDEO_MIME",
            Self::Audio => "MAX_AUDIO_MIME",
            Self::File => "MAX_FILE_MIME",
        }
    }

    fn default_mime(self) -> &'static str {
        match self {
            Self::Image => "image/jpeg",
            Self::Video => "video/mp4",
            Self::Audio => "audio/mpeg",
            Self::File => "application/octet-stream",
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();
    let mut dp = Dispatcher::new(bot);

    dp.on_command("/image", |ctx: Context| send_media(ctx, MediaKind::Image));
    dp.on_command("/video", |ctx: Context| send_media(ctx, MediaKind::Video));
    dp.on_command("/audio", |ctx: Context| send_media(ctx, MediaKind::Audio));
    dp.on_command("/file", |ctx: Context| send_media(ctx, MediaKind::File));

    dp.on_command("/start", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_text_to_chat(
                    message.chat_id(),
                    "Use /image, /video, /audio, or /file after setting the matching MAX_*_PATH env var.",
                )
                .await?;
        }
        Ok(())
    });

    dp.start_polling().await;
}

async fn send_media(ctx: Context, kind: MediaKind) -> Result<()> {
    if let Update::MessageCreated { message, .. } = &ctx.update {
        let path_env = kind.path_env();
        let Some(path) = env::var(path_env).ok() else {
            ctx.bot
                .send_text_to_chat(
                    message.chat_id(),
                    format!("Set {path_env} to a local file path and run this command again."),
                )
                .await?;
            return Ok(());
        };

        let filename = filename_from_path(&path);
        let mime = env::var(kind.mime_env()).unwrap_or_else(|_| kind.default_mime().to_string());
        let text = Some(format!("Sent file from {path_env}"));

        match kind {
            MediaKind::Image => {
                ctx.bot
                    .send_image_to_chat(message.chat_id(), &path, filename, mime, text)
                    .await?;
            }
            MediaKind::Video => {
                ctx.bot
                    .send_video_to_chat(message.chat_id(), &path, filename, mime, text)
                    .await?;
            }
            MediaKind::Audio => {
                ctx.bot
                    .send_audio_to_chat(message.chat_id(), &path, filename, mime, text)
                    .await?;
            }
            MediaKind::File => {
                ctx.bot
                    .send_file_to_chat(message.chat_id(), &path, filename, mime, text)
                    .await?;
            }
        }
    }
    Ok(())
}

fn filename_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload.bin")
        .to_string()
}
