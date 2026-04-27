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
//!     let bot = Bot::from_env();
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

use reqwest::multipart;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

use crate::{
    bot::Bot,
    errors::{MaxError, Result},
    types::{Message, NewAttachment, NewMessageBody, UploadEndpoint, UploadResponse, UploadType},
};

const ATTACHMENT_READY_RETRY_DELAYS_MS: [u64; 5] = [500, 1_000, 2_000, 4_000, 8_000];

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

            MaxError::Api {
                code: 0,
                message: message.into(),
            }
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

fn is_attachment_not_processed_error(error: &MaxError) -> bool {
    matches!(
        error,
        MaxError::Api { message, .. } if message.contains(".not.processed")
    )
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
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
    ) -> Result<String> {
        // Step 1 — request the upload URL.
        let endpoint = self.get_upload_url(upload_type.clone()).await?;
        debug!("Upload URL: {}", endpoint.url);

        // Step 2 — POST the file as multipart.
        let bytes = tokio::fs::read(path).await.map_err(|e| MaxError::Api {
            code: 0,
            message: format!("Failed to read file: {e}"),
        })?;

        let token = self
            .upload_bytes_to_url(&endpoint, bytes, filename.into(), mime.into(), upload_type)
            .await?;

        Ok(token)
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
        debug!("Upload URL: {}", endpoint.url);
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
            .map_err(|e| MaxError::Api {
                code: 0,
                message: format!("Invalid MIME type: {e}"),
            })?;

        let form = multipart::Form::new().part("data", part);

        let resp = self
            .client()
            .post(&endpoint.url)
            .multipart(form)
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await?;
        debug!("Upload response {status}: {body}");

        if !status.is_success() {
            return Err(MaxError::Api {
                code: status.as_u16(),
                message: body,
            });
        }

        Ok(body)
    }

    async fn upload_file_and_send(
        &self,
        recipient: UploadRecipient,
        upload_type: UploadType,
        path: impl AsRef<std::path::Path>,
        filename: impl Into<String>,
        mime: impl Into<String>,
        text: Option<String>,
    ) -> Result<Message> {
        let endpoint = self.get_upload_url(upload_type.clone()).await?;
        debug!("Upload URL: {}", endpoint.url);

        let bytes = tokio::fs::read(path).await.map_err(|e| MaxError::Api {
            code: 0,
            message: format!("Failed to read file: {e}"),
        })?;

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
        debug!("Upload URL: {}", endpoint.url);

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
        for (attempt, retry_delay_ms) in std::iter::once(0)
            .chain(ATTACHMENT_READY_RETRY_DELAYS_MS)
            .enumerate()
        {
            if retry_delay_ms > 0 {
                sleep(Duration::from_millis(retry_delay_ms)).await;
            }

            let body = NewMessageBody::text_opt(text.clone()).with_attachment(attachment.clone());

            let result = match recipient {
                UploadRecipient::Chat(chat_id) => self.send_message_to_chat(chat_id, body).await,
                UploadRecipient::User(user_id) => self.send_message_to_user(user_id, body).await,
            };

            match result {
                Ok(message) => return Ok(message),
                Err(error)
                    if is_attachment_not_processed_error(&error)
                        && attempt < ATTACHMENT_READY_RETRY_DELAYS_MS.len() =>
                {
                    debug!(
                        "Uploaded attachment is not processed yet; retrying send in {} ms",
                        ATTACHMENT_READY_RETRY_DELAYS_MS[attempt]
                    );
                }
                Err(error) => return Err(error),
            }
        }

        unreachable!("uploaded attachment retry loop always returns on the final attempt")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        attachment_from_upload_response, is_attachment_not_processed_error,
        token_from_upload_response,
    };
    use crate::{
        errors::MaxError,
        types::{NewAttachment, UploadEndpoint, UploadType},
    };

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
    fn detects_attachment_processing_errors() {
        assert!(is_attachment_not_processed_error(&MaxError::Api {
            code: 400,
            message: "Key: errors.process.attachment.file.not.processed".into(),
        }));
        assert!(!is_attachment_not_processed_error(&MaxError::Api {
            code: 400,
            message: "permission.denied".into(),
        }));
    }
}
