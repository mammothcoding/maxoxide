use std::io;
use std::time::Duration;

use thiserror::Error;

/// A structured error returned by the MAX HTTP API.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("MAX API error {status}{code_suffix}: {message}", code_suffix = .code.as_ref().map(|code| format!(" ({code})")).unwrap_or_default())]
pub struct ApiError {
    /// HTTP response status.
    pub status: u16,
    /// Machine-readable MAX error code, for example `attachment.not.ready`.
    pub code: Option<String>,
    /// Human-readable error description.
    pub message: String,
    /// Truncated response body retained for diagnostics.
    pub raw_response: Option<String>,
    /// Delay requested by a numeric HTTP `Retry-After` header.
    pub retry_after: Option<Duration>,
}

impl ApiError {
    /// Creates an API error.
    pub fn new(status: u16, code: Option<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            raw_response: None,
            retry_after: None,
        }
    }

    /// Attaches at most 4 KiB of the raw response body for diagnostics.
    pub fn with_raw_response(mut self, response: impl Into<String>) -> Self {
        self.raw_response = Some(response.into().chars().take(4096).collect());
        self
    }

    /// Attaches a server-requested retry delay.
    pub fn with_retry_after(mut self, retry_after: Option<Duration>) -> Self {
        self.retry_after = retry_after;
        self
    }

    /// Returns `true` when MAX is still processing an uploaded attachment.
    pub fn is_attachment_not_ready(&self) -> bool {
        self.code.as_deref() == Some("attachment.not.ready")
            || self.message.contains(".not.processed")
    }

    /// Returns `true` for HTTP 429 responses.
    pub fn is_rate_limited(&self) -> bool {
        self.status == 429
    }

    /// Returns `true` for temporary server failures.
    pub fn is_server_error(&self) -> bool {
        (500..=599).contains(&self.status)
    }
}

/// A request rejected locally before it reached MAX.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("validation error in {field}: {message}")]
pub struct ValidationError {
    /// Field or logical request section that failed validation.
    pub field: String,
    /// Explanation of the violated constraint.
    pub message: String,
}

impl ValidationError {
    /// Creates a validation error.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// All errors that can occur when using maxoxide.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum MaxError {
    /// HTTP transport error such as a timeout or connection failure.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The MAX API returned a non-success response.
    #[error(transparent)]
    Api(#[from] ApiError),

    /// JSON serialization or deserialization failed.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Local file or stream I/O failed.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Client or environment configuration is invalid.
    #[error("configuration error: {0}")]
    Configuration(String),

    /// A request violates a documented MAX constraint.
    #[error(transparent)]
    Validation(#[from] ValidationError),

    /// A successful server response did not satisfy the documented contract.
    #[error("invalid API response: {0}")]
    InvalidResponse(String),

    /// Polling was stopped externally.
    #[error("polling stopped")]
    PollingStopped,

    /// The current operation was cancelled by the caller.
    #[error("operation cancelled")]
    Cancelled,

    /// A webhook adapter or server failed.
    #[error("webhook error: {0}")]
    Webhook(String),
}

impl MaxError {
    /// Returns the structured API error when this error came from MAX.
    pub fn api_error(&self) -> Option<&ApiError> {
        match self {
            Self::Api(error) => Some(error),
            _ => None,
        }
    }
}

/// Result type used by maxoxide APIs.
pub type Result<T> = std::result::Result<T, MaxError>;
