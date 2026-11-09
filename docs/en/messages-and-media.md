# Messages and media

## Addressing

Every send helper has an explicit target kind:

```rust
bot.send_text_to_chat(chat_id, "Reply in this dialog").await?;
bot.send_text_to_user(user_id, "Send by global user ID").await?;
```

Use `SendMessageOptions::disable_link_preview` when the corresponding MAX query flag is needed.

For a replacement message sent while answering a callback, use `answer_callback_with_options` and `AnswerCallbackOptions::disable_link_preview`. The flag is encoded as a `POST /answers` query parameter, not in the JSON body.

`AnswerCallbackBody::notification` remains available only for compatibility with older wire contracts and is deprecated in 4.0. Use a replacement `message` for current integrations:

```rust
use maxoxide::types::{AnswerCallbackBody, NewMessageBody};

bot.answer_callback(AnswerCallbackBody {
    callback_id,
    message: Some(NewMessageBody::text("Confirmed")),
    ..Default::default()
}).await?;
```

## Message pagination

Use `GetMessagesOptions` for the current `before`/`after` contract. Timestamps are Unix time in milliseconds; `count` must be between 1 and 100, and timestamps cannot be negative.

```rust
use maxoxide::types::GetMessagesOptions;

let page = bot.get_messages_with_options(
    chat_id,
    GetMessagesOptions {
        count: Some(50),
        before: Some(before_timestamp),
        after: None,
    },
).await?;

let selected = bot.get_messages_by_ids_with_options(
    [message_id],
    GetMessagesOptions::default(),
).await?;
```

The older `get_messages` and `get_messages_by_ids` signatures retain the deprecated MAX `from`/`to` query parameters for source compatibility. New code should use the options methods.

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

`NewMessageBody::with_notify(false)` suppresses the normal push notification in dialogs and group chats. Channels accept only `true` or an omitted field. Exact presentation remains platform-dependent, and maxoxide does not infer the chat type from a `chat_id` or reject the request locally.

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

## Received media

Received media follows the MAX response shape rather than the outgoing upload shape:

- `Attachment::Image` contains `PhotoAttachmentPayload`, including `photo_id`, dimensions, and unknown extra fields;
- `Attachment::Video` contains `MediaAttachmentPayload` plus top-level `thumbnail`, dimensions, duration, and optional `VideoType`;
- `Attachment::Audio` contains `MediaAttachmentPayload` plus top-level transcription;
- `VideoType` distinguishes regular `video`, round `video_message`, and unknown future values.

```rust
use maxoxide::types::{Attachment, VideoType};

match attachment {
    Attachment::Video {
        payload,
        duration,
        video_type,
        ..
    } => {
        println!("token={:?}, duration={duration:?}", payload.token);
        println!("type={:?}", video_type.as_ref().map(VideoType::as_str));
    }
    _ => {}
}
```

The media variants are non-exhaustive, so external patterns need `..`. Deserialization accepts the old 3.x form where video/audio metadata was nested inside `payload`; serialization emits the canonical top-level form. Unknown nested media fields are preserved in `MediaAttachmentPayload::extra`.

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

Before requesting `/uploads`, all file and byte helpers reject empty content and enforce MAX's decimal size limits: image 50 MB, video 250 MB, audio 256 MB, and file 4 GB. Image dimensions up to 7680x7680 and the 60-minute audio duration limit remain server-side checks.

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

The five typed methods are `get_comments`, `get_comment`, `create_comment`, `edit_comment`, and `delete_comment`. The bot needs the channel access and administrator permissions required by MAX. Incoming changes are available as the typed `CommentCreated`, `CommentEdited`, and `CommentRemoved` updates.
