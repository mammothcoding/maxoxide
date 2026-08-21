# Отмена и progress большой загрузки

```rust
let cancellation = CancellationToken::new();
let cancel_from_ui = cancellation.clone();
let options = UploadOptions::default()
    .with_chunk_size(2 * 1024 * 1024)
    .with_cancellation_token(cancellation)
    .with_progress(|event| tracing::info!(percent = event.percent));

// Вызовите cancel_from_ui.cancel() из shutdown/UI task.
let token = bot.upload_file_with_options(
    UploadType::Video,
    path,
    "video.mp4",
    "video/mp4",
    options,
).await?;
```

Progress callback синхронный: обновляйте в нём только дешёвый atomic/channel. Не выполняйте blocking I/O.
