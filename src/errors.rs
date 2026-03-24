use thiserror::Error;

/// All errors that can occur when using the Max Bot API.
#[derive(Debug, Error)]
pub enum MaxError {
    /// HTTP transport error (network issues, timeouts, etc.)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The API returned a non-2xx status code.
    #[error("API error {code}: {message}")]
    Api { code: u16, message: String },

    /// JSON (de)serialization error.
    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Polling was stopped externally.
    #[error("Polling stopped")]
    PollingStopped,
}

pub type Result<T> = std::result::Result<T, MaxError>;
