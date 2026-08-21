# Messages and media

## Addressing

Every send helper has an explicit target kind:

```rust
bot.send_text_to_chat(chat_id, "Reply in this dialog").await?;
bot.send_text_to_user(user_id, "Send by global user ID").await?;
```

Use `SendMessageOptions::disable_link_preview` when the corresponding MAX query flag is needed.

## Formatting and links

```rust
use maxoxide::types::{MessageFormat, NewMessageBody};

let sent = bot.send_markdown_to_chat(chat_id, "**Markdown**").await?;
bot.send_html_to_chat(chat_id, "<strong>HTML</strong>").await?;
bot.send_message_to_chat(
    chat_id,
    NewMessageBody::text("Reply")
        .with_format(MessageFormat::Markdown)
        .with_reply_to(sent.message_id()),
).await?;
```

`with_forward_from` creates a forward link. Comments only support reply links, and `NewCommentBody::validate` rejects forwards.

MAX limits message/comment text to 4000 Unicode characters. All send/edit/callback/comment methods validate this before making a request. `send_long_text_to_chat` and `send_long_formatted_text_to_chat` split at Unicode character boundaries and return every sent `Message` in order.

## Keyboards

```rust
use maxoxide::types::{Button, KeyboardPayload, NewMessageBody};

let keyboard = KeyboardPayload {
    buttons: vec![
        vec![Button::callback("Confirm", "confirm")],
        vec![Button::link("Open", "https://max.ru")],
        vec![Button::request_contact("Phone")],
    ],
};
bot.send_message_to_chat(chat_id, NewMessageBody::text("Choose").with_keyboard(keyboard)).await?;
```

Validation enforces 30 rows, 7 buttons per normal row, 3 buttons in rows containing wide link/open-app/contact/location buttons, non-empty button text/payload, and the 2048-character link limit. The obsolete `intent` property is not emitted because official clients mark it unsupported.

Available button builders include callback, link, message, open app, clipboard, contact, geo location, and chat creation.

## Attachment requests

`NewAttachment` supports:

- image by uploaded token, URL, or MAX photo-token map;
- video, audio, and file by uploaded token;
- sticker by code;
- contact card;
- latitude/longitude location;
- shared URL or token;
- inline keyboard.

Payload validation catches missing tokens/URLs, empty sticker codes, invalid coordinates, and invalid keyboards. Received attachment enums preserve unknown future payloads.

## Streaming upload

`upload_file` never reads the complete file into memory. It obtains `/uploads`, opens the file asynchronously, and streams a multipart body when the endpoint has no pre-issued token:

```rust
let token = bot.upload_file(
    UploadType::File,
    "report.pdf",
    "report.pdf",
    "application/pdf",
).await?;
```

`upload_bytes` is intentionally memory-backed because the caller already owns a `Vec<u8>`.

## Resumable upload

When MAX returns an upload token, file upload automatically switches to raw `Content-Range` chunks using the headers expected by the official TypeScript SDK. Each chunk is sent sequentially. Only a failed chunk is replayed after a transport, 429, or 5xx failure; completed chunks are not resent.

```rust
use maxoxide::uploader::UploadOptions;

let cancellation = tokio_util::sync::CancellationToken::new();
let options = UploadOptions::default()
    .with_chunk_size(2 * 1024 * 1024)
    .with_timeout(std::time::Duration::from_secs(120))
    .with_cancellation_token(cancellation.clone())
    .with_progress(|event| {
        println!("{:.0}%", event.percent);
    });

let token = bot.upload_file_with_options(
    UploadType::Video,
    "video.mp4",
    "video.mp4",
    "video/mp4",
    options,
).await?;
```

Chunk size must be between 1 byte and 16 MiB. The default is 1 MiB with three attempts. Filenames are rejected if they could inject resumable-upload headers. Cancelling drops the active request and returns `MaxError::Cancelled`.

## Upload and send helpers

Helpers such as `send_image_to_chat`, `send_video_to_user`, and their byte variants perform upload, build the attachment, and send the message. Attachment processing races are retried by the central client policy when MAX returns `attachment.not.ready` or its older `.not.processed` message.

## Comments

The five typed methods are `get_comments`, `get_comment`, `create_comment`, `edit_comment`, and `delete_comment`. MAX currently labels all of them temporarily unavailable. Treat them as an experimental contract: test against mocks, handle API failures, and do not make a production workflow depend on their availability yet.
