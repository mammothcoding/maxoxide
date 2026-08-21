//! File upload helpers for the Max Bot API.
//!
//! The Max API uses a two-step upload flow:
//!
//! 1. `POST /uploads?type=<type>` → receive `{ url, token? }`
//! 2. `POST <url>` multipart form → receive or activate an attachment token.
//!
//! MAX may return the attachment token either in the upload endpoint response,
//! in the multipart upload response, or for images as a `photos` token map.
//! Upload-and-send helpers preserve the image `photos` payload and retry briefly
//! while MAX finishes processing the uploaded attachment.
//!
//! # Example
//!
//! ```no_run
//! use maxoxide::Bot;
//! use maxoxide::types::{NewAttachment, NewMessageBody, UploadType};
//!
//! #[tokio::main]
//! async fn main() {
//!     let bot = Bot::from_env().expect("valid MAX_BOT_TOKEN");
//!
//!     // Upload an image from disk
//!     let token = bot
//!         .upload_file(UploadType::Image, "/path/to/photo.jpg", "photo.jpg", "image/jpeg")
//!         .await
//!         .unwrap();
//!
//!     // Attach it to a message
//!     let body = NewMessageBody {
//!         text: Some("Here is a photo".into()),
//!         attachments: Some(vec![NewAttachment::image(token)]),
//!         ..Default::default()
//!     };
//!     bot.send_message_to_chat(12345678, body).await.unwrap();
//! }
//! ```

use futures_util::StreamExt;
use reqwest::{Body, StatusCode, header, multipart};
use std::{
    fmt,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::io::AsyncReadExt;
use tokio_util::{io::ReaderStream, sync::CancellationToken};
use tracing::debug;

use crate::{
    bot::Bot,
    errors::{ApiError, MaxError, Result, ValidationError},
    types::{Message, NewAttachment, NewMessageBody, UploadEndpoint, UploadResponse, UploadType},
};

const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;
const MAX_CHUNK_SIZE: usize = 16 * 1024 * 1024;

/// Progress reported while streaming a file to MAX.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UploadProgress {
    /// Number of bytes successfully uploaded so far.
    pub uploaded: u64,
    /// Total file size in bytes.
    pub total: u64,
    /// Completion percentage in the inclusive range `0.0..=100.0`.
    pub percent: f64,
}

type ProgressCallback = Arc<dyn Fn(UploadProgress) + Send + Sync>;

/// Controls timeout, chunking, retry, progress, and cancellation for an upload.
#[derive(Clone)]
pub struct UploadOptions {
    /// Optional timeout applied to each upload request.
    pub timeout: Option<Duration>,
    /// Number of file bytes sent in each resumable chunk.
    pub chunk_size: usize,
    /// Maximum attempts for one resumable chunk.
    pub max_chunk_attempts: u32,
    /// Initial exponential-backoff delay between chunk retries.
    pub initial_retry_delay: Duration,
    /// Optional token used to cancel an in-progress upload.
    pub cancellation_token: Option<CancellationToken>,
    progress: Option<ProgressCallback>,
}

impl Default for UploadOptions {
    fn default() -> Self {
        Self {
            timeout: None,
            chunk_size: DEFAULT_CHUNK_SIZE,
            max_chunk_attempts: 3,
            initial_retry_delay: Duration::from_millis(250),
            cancellation_token: None,
            progress: None,
        }
    }
}

impl fmt::Debug for UploadOptions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UploadOptions")
            .field("timeout", &self.timeout)
            .field("chunk_size", &self.chunk_size)
            .field("max_chunk_attempts", &self.max_chunk_attempts)
            .field("initial_retry_delay", &self.initial_retry_delay)
            .field("has_cancellation_token", &self.cancellation_token.is_some())
            .field("has_progress_callback", &self.progress.is_some())
            .finish()
    }
}

impl UploadOptions {
    /// Sets the timeout applied to each upload request.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the resumable chunk size in bytes.
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Sets the token used to cancel the upload.
    pub fn with_cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation_token = Some(token);
        self
    }

    /// Registers a callback invoked after upload progress changes.
    pub fn with_progress(
        mut self,
        callback: impl Fn(UploadProgress) + Send + Sync + 'static,
    ) -> Self {
        self.progress = Some(Arc::new(callback));
        self
    }

    fn validate(&self) -> Result<()> {
        if self.chunk_size == 0 || self.chunk_size > MAX_CHUNK_SIZE {
            return Err(ValidationError::new(
                "chunk_size",
                format!("must be between 1 and {MAX_CHUNK_SIZE} bytes"),
            )
            .into());
        }
        if self.max_chunk_attempts == 0 {
            return Err(ValidationError::new("max_chunk_attempts", "must be at least 1").into());
        }
        Ok(())
    }

    fn report(&self, uploaded: u64, total: u64) {
        if let Some(callback) = &self.progress {
            callback(UploadProgress {
                uploaded: uploaded.min(total),
                total,
                percent: if total == 0 {
                    100.0
                } else {
                    uploaded.min(total) as f64 / total as f64 * 100.0
                },
            });
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UploadRecipient {
    Chat(i64),
    User(i64),
}

fn token_from_upload_response(
    endpoint: &UploadEndpoint,
    body: &str,
    upload_type: UploadType,
) -> Result<String> {
    let response = serde_json::from_str::<UploadResponse>(body).ok();
    let body_token = response
        .as_ref()
        .and_then(|response| response.token.clone());
    let first_photo_token = if upload_type == UploadType::Image {
        response
            .as_ref()
            .and_then(|response| response.photos.as_ref())
            .and_then(|photos| photos.values().next())
            .map(|photo| photo.token.clone())
    } else {
        None
    };

    body_token
        .or(first_photo_token)
        .or_else(|| endpoint.token.clone())
        .ok_or_else(|| {
            let message = match upload_type {
                UploadType::Image | UploadType::File => {
                    "No token in upload response body or upload endpoint response for image/file"
                }
                UploadType::Video | UploadType::Audio => {
                    "No token in upload endpoint response or upload response body for video/audio"
                }
            };

            MaxError::InvalidResponse(message.into())
        })
}

fn attachment_from_upload_response(
    endpoint: &UploadEndpoint,
    body: &str,
    upload_type: UploadType,
) -> Result<NewAttachment> {
    let image_photos = if upload_type == UploadType::Image {
        serde_json::from_str::<UploadResponse>(body)
            .ok()
            .and_then(|response| response.photos)
            .filter(|photos| !photos.is_empty())
    } else {
        None
    };

    if let Some(photos) = image_photos {
        return Ok(NewAttachment::image_photos(photos));
    }

    let token = token_from_upload_response(endpoint, body, upload_type.clone())?;
    let attachment = match upload_type {
        UploadType::Image => NewAttachment::image(token),
        UploadType::Video => NewAttachment::video(token),
        UploadType::Audio => NewAttachment::audio(token),
        UploadType::File => NewAttachment::file(token),
    };

    Ok(attachment)
}

fn validate_upload_filename(filename: &str) -> Result<()> {
    if filename.is_empty() {
        return Err(ValidationError::new("filename", "value is empty").into());
    }
    if filename
        .chars()
        .any(|character| character.is_control() || matches!(character, '"' | '\\'))
    {
        return Err(ValidationError::new(
            "filename",
            "must not contain control characters, quotes, or backslashes",
        )
        .into());
    }
    Ok(())
}

fn upload_api_error(status: StatusCode, body: String) -> ApiError {
    let value = serde_json::from_str::<serde_json::Value>(&body).ok();
    let code = value
        .as_ref()
        .and_then(|value| value.get("code"))
        .and_then(serde_json::Value::as_str)
        .map(String::from);
    let message = value
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            status
                .canonical_reason()
                .unwrap_or("MAX upload failed")
                .to_string()
        });

    ApiError::new(status.as_u16(), code, message).with_raw_response(body)
}

fn is_retryable_upload_error(error: &MaxError) -> bool {
    match error {
        MaxError::Http(_) => true,
        MaxError::Api(error) => error.is_rate_limited() || error.is_server_error(),
        _ => false,
    }
}

async fn sleep_or_cancel(
    delay: Duration,
    cancellation_token: Option<&CancellationToken>,
) -> Result<()> {
    if let Some(token) = cancellation_token {
        tokio::select! {
            _ = token.cancelled() => Err(MaxError::Cancelled),
            _ = tokio::time::sleep(delay) => Ok(()),
        }
    } else {
        tokio::time::sleep(delay).await;
        Ok(())
    }
}

impl Bot {
    /// Full two-step upload:
    ///
    /// 1. Gets the upload URL (and pre-issued token for video/audio) from `POST /uploads`.
    /// 2. POSTs the file as `multipart/form-data` to that URL.
    ///
    /// Returns the **attachment token** to use in `NewAttachment`.
    ///
    /// MAX can return the token from the upload endpoint response, from the
    /// multipart upload response, or for images as a `photos` token map. This
    /// method accepts all forms and returns the first usable token. The
    /// higher-level `send_image_*` helpers preserve the full `photos` payload.
    ///
    /// # Arguments
    /// * `upload_type` — one of `Image`, `Video`, `Audio`, `File`.
    /// * `path`        — path to a local file.
    /// * `filename`    — the filename to send in the multipart form (`data` field).
    /// * `mime`        — MIME type, e.g. `"image/jpeg"`, `"video/mp4"`, `"application/pdf"`.
    pub async fn upload_file(
        &self,
        upload_type: UploadType,
        path: impl AsRef<Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
    ) -> Result<String> {
        self.upload_file_with_options(upload_type, path, filename, mime, UploadOptions::default())
            .await
    }

    /// Streams a file with explicit progress, cancellation, timeout, and chunk controls.
    pub async fn upload_file_with_options(
        &self,
        upload_type: UploadType,
        path: impl AsRef<Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        options: UploadOptions,
    ) -> Result<String> {
        options.validate()?;
        let endpoint = self.get_upload_url(upload_type.clone()).await?;
        let response = self
            .upload_file_to_url_body(
                &endpoint,
                path.as_ref(),
                filename.into(),
                mime.into(),
                &options,
            )
            .await?;
        token_from_upload_response(&endpoint, &response, upload_type)
    }

    /// Like `upload_file`, but accepts raw bytes instead of a file path.
    pub async fn upload_bytes(
        &self,
        upload_type: UploadType,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
    ) -> Result<String> {
        let endpoint = self.get_upload_url(upload_type.clone()).await?;
        self.upload_bytes_to_url(&endpoint, bytes, filename.into(), mime.into(), upload_type)
            .await
    }

    /// Upload an image from disk and send it to a chat.
    pub async fn send_image_to_chat(
        &self,
        chat_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::Image,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload a video from disk and send it to a chat.
    pub async fn send_video_to_chat(
        &self,
        chat_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::Video,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload an audio file from disk and send it to a chat.
    pub async fn send_audio_to_chat(
        &self,
        chat_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::Audio,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload a generic file from disk and send it to a chat.
    pub async fn send_file_to_chat(
        &self,
        chat_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::File,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload an image from disk and send it to a user.
    pub async fn send_image_to_user(
        &self,
        user_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::User(user_id),
            UploadType::Image,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload a video from disk and send it to a user.
    pub async fn send_video_to_user(
        &self,
        user_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::User(user_id),
            UploadType::Video,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload an audio file from disk and send it to a user.
    pub async fn send_audio_to_user(
        &self,
        user_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::User(user_id),
            UploadType::Audio,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload a generic file from disk and send it to a user.
    pub async fn send_file_to_user(
        &self,
        user_id: i64,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_file_and_send(
            UploadRecipient::User(user_id),
            UploadType::File,
            path,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload image bytes and send them to a chat.
    pub async fn send_image_bytes_to_chat(
        &self,
        chat_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::Image,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload video bytes and send them to a chat.
    pub async fn send_video_bytes_to_chat(
        &self,
        chat_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::Video,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload audio bytes and send them to a chat.
    pub async fn send_audio_bytes_to_chat(
        &self,
        chat_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::Audio,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload file bytes and send them to a chat.
    pub async fn send_file_bytes_to_chat(
        &self,
        chat_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::Chat(chat_id),
            UploadType::File,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload image bytes and send them to a user.
    pub async fn send_image_bytes_to_user(
        &self,
        user_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::User(user_id),
            UploadType::Image,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload video bytes and send them to a user.
    pub async fn send_video_bytes_to_user(
        &self,
        user_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::User(user_id),
            UploadType::Video,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload audio bytes and send them to a user.
    pub async fn send_audio_bytes_to_user(
        &self,
        user_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::User(user_id),
            UploadType::Audio,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    /// Upload file bytes and send them to a user.
    pub async fn send_file_bytes_to_user(
        &self,
        user_id: i64,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        self.upload_bytes_and_send(
            UploadRecipient::User(user_id),
            UploadType::File,
            bytes,
            filename,
            mime,
            text,
        )
        .await
    }

    // ────────────────────────────────────────────────
    // Internal
    // ────────────────────────────────────────────────

    async fn upload_file_to_url_body(
        &self,
        endpoint: &UploadEndpoint,
        path: &Path,
        filename: String,
        mime: String,
        options: &UploadOptions,
    ) -> Result<String> {
        let metadata = tokio::fs::metadata(path).await?;
        if !metadata.is_file() {
            return Err(ValidationError::new("path", "value does not point to a file").into());
        }
        if metadata.len() == 0 {
            return Err(ValidationError::new("path", "file is empty").into());
        }
        validate_upload_filename(&filename)?;

        if endpoint.token.is_some() {
            self.upload_file_by_ranges(endpoint, path, &filename, metadata.len(), options)
                .await?;
            Ok(String::new())
        } else {
            self.upload_file_multipart(endpoint, path, filename, mime, metadata.len(), options)
                .await
        }
    }

    async fn upload_file_multipart(
        &self,
        endpoint: &UploadEndpoint,
        path: &Path,
        filename: String,
        mime: String,
        total: u64,
        options: &UploadOptions,
    ) -> Result<String> {
        let file = tokio::fs::File::open(path).await?;
        let uploaded = Arc::new(AtomicU64::new(0));
        let progress = options.progress.clone();
        let stream = ReaderStream::new(file).map(move |result| {
            if let (Ok(bytes), Some(callback)) = (&result, &progress) {
                let uploaded =
                    uploaded.fetch_add(bytes.len() as u64, Ordering::Relaxed) + bytes.len() as u64;
                callback(UploadProgress {
                    uploaded: uploaded.min(total),
                    total,
                    percent: uploaded.min(total) as f64 / total as f64 * 100.0,
                });
            }
            result
        });
        let part = multipart::Part::stream_with_length(Body::wrap_stream(stream), total)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|error| ValidationError::new("mime", error.to_string()))?;
        let form = multipart::Form::new().part("data", part);
        let request = self
            .api_client()
            .await?
            .post(&endpoint.url)
            .timeout(options.timeout.unwrap_or_else(|| self.upload_timeout()))
            .multipart(form);
        let response =
            Self::send_upload_request(request, options.cancellation_token.as_ref()).await?;
        Self::parse_upload_response(response).await
    }

    async fn upload_file_by_ranges(
        &self,
        endpoint: &UploadEndpoint,
        path: &Path,
        filename: &str,
        total: u64,
        options: &UploadOptions,
    ) -> Result<()> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut offset = 0_u64;
        let client = self.api_client().await?;
        let timeout = options.timeout.unwrap_or_else(|| self.upload_timeout());

        while offset < total {
            if options
                .cancellation_token
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
            {
                return Err(MaxError::Cancelled);
            }

            let bytes_to_read = (total - offset).min(options.chunk_size as u64) as usize;
            let mut chunk = vec![0; bytes_to_read];
            file.read_exact(&mut chunk).await?;
            let end = offset + chunk.len() as u64 - 1;
            let mut last_error = None;

            for attempt in 0..options.max_chunk_attempts {
                let request = client
                    .post(&endpoint.url)
                    .timeout(timeout)
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{filename}\""),
                    )
                    .header(
                        header::CONTENT_RANGE,
                        format!("bytes {offset}-{end}/{total}"),
                    )
                    .header(
                        header::CONTENT_TYPE,
                        "application/x-binary; charset=x-user-defined",
                    )
                    .header("X-File-Name", filename)
                    .header("X-Uploading-Mode", "parallel")
                    .body(chunk.clone());
                let result =
                    Self::send_upload_request(request, options.cancellation_token.as_ref()).await;
                let result = match result {
                    Ok(response) => Self::parse_upload_response(response).await.map(|_| ()),
                    Err(error) => Err(error),
                };

                match result {
                    Ok(()) => {
                        last_error = None;
                        break;
                    }
                    Err(error)
                        if attempt + 1 < options.max_chunk_attempts
                            && is_retryable_upload_error(&error) =>
                    {
                        last_error = Some(error);
                        let delay = options
                            .initial_retry_delay
                            .saturating_mul(1 << attempt.min(16));
                        sleep_or_cancel(delay, options.cancellation_token.as_ref()).await?;
                    }
                    Err(error) => return Err(error),
                }
            }

            if let Some(error) = last_error {
                return Err(error);
            }
            offset = end + 1;
            options.report(offset, total);
        }

        Ok(())
    }

    async fn send_upload_request(
        request: reqwest::RequestBuilder,
        cancellation_token: Option<&CancellationToken>,
    ) -> Result<reqwest::Response> {
        if let Some(token) = cancellation_token {
            tokio::select! {
                _ = token.cancelled() => Err(MaxError::Cancelled),
                response = request.send() => Ok(response?),
            }
        } else {
            Ok(request.send().await?)
        }
    }

    async fn parse_upload_response(response: reqwest::Response) -> Result<String> {
        let status = response.status();
        let body = response.text().await?;
        debug!(status = status.as_u16(), "MAX upload response");
        if status.is_success() {
            Ok(body)
        } else {
            Err(upload_api_error(status, body).into())
        }
    }

    async fn upload_bytes_to_url(
        &self,
        endpoint: &UploadEndpoint,
        bytes: Vec<u8>,
        filename: String,
        mime: String,
        upload_type: UploadType,
    ) -> Result<String> {
        let body = self
            .upload_bytes_to_url_body(endpoint, bytes, filename, mime)
            .await?;
        token_from_upload_response(endpoint, &body, upload_type)
    }

    async fn upload_bytes_to_url_as_attachment(
        &self,
        endpoint: &UploadEndpoint,
        bytes: Vec<u8>,
        filename: String,
        mime: String,
        upload_type: UploadType,
    ) -> Result<NewAttachment> {
        let body = self
            .upload_bytes_to_url_body(endpoint, bytes, filename, mime)
            .await?;
        attachment_from_upload_response(endpoint, &body, upload_type)
    }

    async fn upload_bytes_to_url_body(
        &self,
        endpoint: &UploadEndpoint,
        bytes: Vec<u8>,
        filename: String,
        mime: String,
    ) -> Result<String> {
        let part = multipart::Part::bytes(bytes)
            .file_name(filename)
            .mime_str(&mime)
            .map_err(|error| ValidationError::new("mime", error.to_string()))?;

        let form = multipart::Form::new().part("data", part);

        let resp = self
            .api_client()
            .await?
            .post(&endpoint.url)
            .timeout(self.upload_timeout())
            .multipart(form)
            .send()
            .await?;
        Self::parse_upload_response(resp).await
    }

    async fn upload_file_and_send(
        &self,
        recipient: UploadRecipient,
        upload_type: UploadType,
        path: impl AsRef<Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        let endpoint = self.get_upload_url(upload_type.clone()).await?;

        let response = self
            .upload_file_to_url_body(
                &endpoint,
                path.as_ref(),
                filename.into(),
                mime.into(),
                &UploadOptions::default(),
            )
            .await?;
        let attachment = attachment_from_upload_response(&endpoint, &response, upload_type)?;

        self.send_uploaded_attachment(recipient, attachment, text)
            .await
    }

    async fn upload_bytes_and_send(
        &self,
        recipient: UploadRecipient,
        upload_type: UploadType,
        bytes: Vec<u8>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        let endpoint = self.get_upload_url(upload_type.clone()).await?;

        let attachment = self
            .upload_bytes_to_url_as_attachment(
                &endpoint,
                bytes,
                filename.into(),
                mime.into(),
                upload_type,
            )
            .await?;

        self.send_uploaded_attachment(recipient, attachment, text)
            .await
    }

    async fn send_uploaded_attachment(
        &self,
        recipient: UploadRecipient,
        attachment: NewAttachment,
        text: Option<String>,
    ) -> Result<Message> {
        let body = NewMessageBody::text_opt(text).with_attachment(attachment);
        match recipient {
            UploadRecipient::Chat(chat_id) => self.send_message_to_chat(chat_id, body).await,
            UploadRecipient::User(user_id) => self.send_message_to_user(user_id, body).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_CHUNK_SIZE, UploadOptions, attachment_from_upload_response, token_from_upload_response,
        validate_upload_filename,
    };
    use crate::types::{NewAttachment, UploadEndpoint, UploadType};

    fn endpoint(token: Option<&str>) -> UploadEndpoint {
        UploadEndpoint {
            url: "https://upload.example.test".into(),
            token: token.map(Into::into),
        }
    }

    #[test]
    fn upload_token_prefers_multipart_response_body() {
        let token = token_from_upload_response(
            &endpoint(Some("endpoint_token")),
            r#"{"token":"body_token"}"#,
            UploadType::File,
        )
        .unwrap();

        assert_eq!(token, "body_token");
    }

    #[test]
    fn upload_token_falls_back_to_endpoint_token() {
        let token = token_from_upload_response(
            &endpoint(Some("endpoint_token")),
            r#"{}"#,
            UploadType::Image,
        )
        .unwrap();

        assert_eq!(token, "endpoint_token");
    }

    #[test]
    fn upload_token_falls_back_to_first_photo_token_for_images() {
        let token = token_from_upload_response(
            &endpoint(None),
            r#"{"photos":{"photo-1":{"token":"photo_token"}}}"#,
            UploadType::Image,
        )
        .unwrap();

        assert_eq!(token, "photo_token");
    }

    #[test]
    fn image_attachment_uses_uploaded_photo_tokens() {
        let attachment = attachment_from_upload_response(
            &endpoint(None),
            r#"{"photos":{"photo-1":{"token":"photo_token"}}}"#,
            UploadType::Image,
        )
        .unwrap();

        let NewAttachment::Image { payload } = attachment else {
            panic!("image upload should create an image attachment");
        };

        let photos = payload
            .photos
            .expect("image attachment should carry photos");
        assert_eq!(photos["photo-1"].token, "photo_token");
        assert!(payload.token.is_none());
    }

    #[test]
    fn upload_token_reports_missing_token() {
        let error = token_from_upload_response(&endpoint(None), r#"{}"#, UploadType::Image)
            .expect_err("missing token should fail");

        assert!(error.to_string().contains("No token"));
    }

    #[test]
    fn upload_options_reject_unbounded_chunks() {
        assert!(UploadOptions::default().validate().is_ok());
        assert!(
            UploadOptions::default()
                .with_chunk_size(MAX_CHUNK_SIZE + 1)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn resumable_filename_rejects_header_injection() {
        assert!(validate_upload_filename("video.mp4").is_ok());
        assert!(validate_upload_filename("video.mp4\r\nX-Foo: bar").is_err());
    }
}
