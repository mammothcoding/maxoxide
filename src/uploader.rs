//! File upload helpers for the Max Bot API.
//!
//! The Max API uses a two-step upload flow:
//!
//! 1. `POST /uploads?type=<type>` → receive `{ url, token? }`
//! 2. `POST <url>` multipart form → for **image** and **file** types the upload
//!    response itself contains the `token`; for **video** and **audio** the token
//!    is already given in step 1.
//!
//! # Example
//!
//! ```no_run
//! use maxoxide::Bot;
//! use maxoxide::types::{NewMessageBody, NewAttachment, UploadedToken, UploadType};
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
//!         attachments: Some(vec![NewAttachment::Image {
//!             payload: UploadedToken { token },
//!         }]),
//!         ..Default::default()
//!     };
//!     bot.send_message(12345678, body).await.unwrap();
//! }
//! ```

use reqwest::multipart;
use tracing::debug;

use crate::{
    bot::Bot,
    errors::{MaxError, Result},
    types::{UploadEndpoint, UploadResponse, UploadType},
};

impl Bot {
    /// Full two-step upload:
    ///
    /// 1. Gets the upload URL (and pre-issued token for video/audio) from `POST /uploads`.
    /// 2. POSTs the file as `multipart/form-data` to that URL.
    ///
    /// Returns the **attachment token** to use in `NewAttachment`.
    ///
    /// For `image` and `file` types the token comes from the upload response body.
    /// For `video` and `audio` the token is pre-issued in step 1; the upload
    /// response is not used for the token (it returns a `retval` instead).
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

        // For video/audio: token is pre-issued in `endpoint.token`.
        // For image/file: token comes from the upload response body.
        match upload_type {
            UploadType::Video | UploadType::Audio => {
                endpoint.token.clone().ok_or_else(|| MaxError::Api {
                    code: 0,
                    message: "No token in upload endpoint response for video/audio".into(),
                })
            }
            UploadType::Image | UploadType::File => {
                let upload_resp: UploadResponse =
                    serde_json::from_str(&body).map_err(MaxError::Json)?;
                upload_resp.token.ok_or_else(|| MaxError::Api {
                    code: 0,
                    message: "No token in upload response body for image/file".into(),
                })
            }
        }
    }
}
