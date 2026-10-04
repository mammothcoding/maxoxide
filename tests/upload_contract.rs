use std::{
    collections::HashMap,
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use maxoxide::types::UploadType;
use maxoxide::uploader::UploadOptions;
use maxoxide::{Bot, MaxError, RateLimitConfig, RetryPolicy};

#[derive(Debug, Clone)]
struct ChunkRequest {
    range: String,
    body: Vec<u8>,
}

#[derive(Clone)]
struct UploadState {
    upload_url: String,
    upload_requests: Arc<AtomicUsize>,
    chunks: Arc<Mutex<Vec<ChunkRequest>>>,
    attempts: Arc<Mutex<HashMap<String, usize>>>,
}

async fn upload_api(State(state): State<UploadState>, request: Request<Body>) -> Response {
    if request.uri().path() == "/uploads" {
        state.upload_requests.fetch_add(1, Ordering::SeqCst);
        return (
            StatusCode::OK,
            [("content-type", "application/json")],
            format!(r#"{{"url":"{}","token":"file-token"}}"#, state.upload_url),
        )
            .into_response();
    }

    let range = request
        .headers()
        .get("content-range")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let body = to_bytes(request.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    state.chunks.lock().unwrap().push(ChunkRequest {
        range: range.clone(),
        body,
    });
    let mut attempts = state.attempts.lock().unwrap();
    let attempt = attempts.entry(range.clone()).or_default();
    *attempt += 1;
    if range == "bytes 4-7/10" && *attempt == 1 {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [("content-type", "application/json")],
            r#"{"code":"temporary","message":"retry this chunk"}"#,
        )
            .into_response();
    }
    StatusCode::OK.into_response()
}

#[tokio::test]
async fn resumable_upload_retries_only_the_failed_chunk() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = UploadState {
        upload_url: format!("http://{address}/upload"),
        upload_requests: Arc::new(AtomicUsize::new(0)),
        chunks: Arc::new(Mutex::new(Vec::new())),
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new().fallback(upload_api).with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"0123456789").unwrap();
    let progress = Arc::new(Mutex::new(Vec::new()));
    let progress_sink = progress.clone();
    let options = UploadOptions::default()
        .with_chunk_size(4)
        .with_progress(move |event| progress_sink.lock().unwrap().push(event));
    let bot = Bot::builder("token")
        .base_url(format!("http://{address}"))
        .rate_limits(RateLimitConfig::disabled())
        .retry_policy(RetryPolicy::disabled())
        .build()
        .unwrap();

    let token = bot
        .upload_file_with_options(
            UploadType::File,
            file.path(),
            "sample.bin",
            "application/octet-stream",
            options,
        )
        .await
        .unwrap();
    assert_eq!(token, "file-token");

    let chunks = state.chunks.lock().unwrap();
    assert_eq!(
        chunks
            .iter()
            .map(|chunk| chunk.range.as_str())
            .collect::<Vec<_>>(),
        [
            "bytes 0-3/10",
            "bytes 4-7/10",
            "bytes 4-7/10",
            "bytes 8-9/10",
        ]
    );
    assert_eq!(chunks[0].body, b"0123");
    assert_eq!(chunks[1].body, chunks[2].body);
    assert_eq!(chunks[3].body, b"89");
    let progress = progress.lock().unwrap();
    assert_eq!(progress.last().unwrap().uploaded, 10);
    assert_eq!(progress.last().unwrap().percent, 100.0);
    server.abort();
}

#[tokio::test]
async fn pre_cancelled_upload_stops_before_the_first_chunk() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = UploadState {
        upload_url: format!("http://{address}/upload"),
        upload_requests: Arc::new(AtomicUsize::new(0)),
        chunks: Arc::new(Mutex::new(Vec::new())),
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new().fallback(upload_api).with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(b"content").unwrap();
    let cancellation = tokio_util::sync::CancellationToken::new();
    cancellation.cancel();
    let bot = Bot::builder("token")
        .base_url(format!("http://{address}"))
        .rate_limits(RateLimitConfig::disabled())
        .build()
        .unwrap();

    let error = bot
        .upload_file_with_options(
            UploadType::File,
            file.path(),
            "sample.bin",
            "application/octet-stream",
            UploadOptions::default()
                .with_timeout(Duration::from_secs(1))
                .with_cancellation_token(cancellation),
        )
        .await
        .unwrap_err();
    assert!(matches!(error, MaxError::Cancelled));
    assert!(state.chunks.lock().unwrap().is_empty());
    server.abort();
}

#[tokio::test]
async fn rejects_empty_and_oversized_uploads_before_requesting_an_upload_url() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = UploadState {
        upload_url: format!("http://{address}/upload"),
        upload_requests: Arc::new(AtomicUsize::new(0)),
        chunks: Arc::new(Mutex::new(Vec::new())),
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    let app = Router::new().fallback(upload_api).with_state(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let bot = Bot::builder("token")
        .base_url(format!("http://{address}"))
        .rate_limits(RateLimitConfig::disabled())
        .retry_policy(RetryPolicy::disabled())
        .build()
        .unwrap();

    let empty_file = tempfile::NamedTempFile::new().unwrap();
    let oversized_file = tempfile::NamedTempFile::new().unwrap();
    oversized_file.as_file().set_len(50_000_001).unwrap();

    let errors = [
        bot.upload_file(
            UploadType::Image,
            empty_file.path(),
            "empty.png",
            "image/png",
        )
        .await
        .unwrap_err(),
        bot.send_image_to_chat(42, oversized_file.path(), "large.png", "image/png", None)
            .await
            .unwrap_err(),
        bot.upload_bytes(UploadType::Image, Vec::new(), "empty.png", "image/png")
            .await
            .unwrap_err(),
        bot.send_image_bytes_to_chat(42, Vec::new(), "empty.png", "image/png", None)
            .await
            .unwrap_err(),
        bot.upload_bytes(
            UploadType::Image,
            vec![0; 50_000_001],
            "large.png",
            "image/png",
        )
        .await
        .unwrap_err(),
    ];

    assert!(
        errors
            .iter()
            .all(|error| matches!(error, MaxError::Validation(_)))
    );
    assert_eq!(state.upload_requests.load(Ordering::SeqCst), 0);
    assert!(state.chunks.lock().unwrap().is_empty());
    server.abort();
}
