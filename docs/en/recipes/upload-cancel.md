# Cancel and observe a large upload

```rust
let cancellation = CancellationToken::new();
let cancel_from_ui = cancellation.clone();
let options = UploadOptions::default()
    .with_chunk_size(2 * 1024 * 1024)
    .with_cancellation_token(cancellation)
    .with_progress(|event| tracing::info!(percent = event.percent));

// Call cancel_from_ui.cancel() from your shutdown/UI task.
let token = bot.upload_file_with_options(
    UploadType::Video,
    path,
    "video.mp4",
    "video/mp4",
    options,
).await?;
```

Progress callbacks are synchronous and should only update cheap atomics/channels. Do not perform blocking I/O inside them.
