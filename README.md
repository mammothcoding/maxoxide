[![Crates.io](https://img.shields.io/crates/v/maxoxide.svg)](https://crates.io/crates/maxoxide)
[![docs.rs](https://docs.rs/maxoxide/badge.svg)](https://docs.rs/maxoxide/)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://choosealicense.com/licenses/mit/)
[![Build Status](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml/badge.svg?event=push)](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)

Readme in different languages:
[EN](README.md) · [RU](README.ru.md)

# ![alt text](./max_logo.png "max_logo") maxoxide

🦀 An async Rust library for building bots on the [Max messenger](https://max.ru) platform,
inspired by [teloxide](https://github.com/teloxide/teloxide).

## Features

- ✅ Coverage of the published Max Bot REST API
- ✅ Long polling and optional **Webhook** server via [axum](https://github.com/tokio-rs/axum)
- ✅ Forward-compatible typed events (`Update`, `Message`, `Callback`, unknown update fallback)
- ✅ `Dispatcher` with fluent handler registration, composable filters, startup hooks and scheduled tasks
- ✅ Inline keyboards (`callback`, `link`, `message`, `open_app`, `clipboard`, `request_contact`, `request_geo_location`)
- ✅ File uploads — multipart, image photo-token payloads, correct token flow for video/audio, send helpers for image/video/audio/file
- ✅ Markdown / HTML message formatting
- ✅ Webhook secret verification (`X-Max-Bot-Api-Secret`)
- ✅ Tokio async throughout

## Quick start

```toml
[dependencies]
maxoxide = "2.0.0"
tokio    = { version = "1", features = ["full"] }

# Enable the built-in axum webhook server:
# maxoxide = { version = "2.0.0", features = ["webhook"] }
```

```rust
use maxoxide::{Bot, Context, Dispatcher};
use maxoxide::types::Update;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();     // reads MAX_BOT_TOKEN env var
    let mut dp = Dispatcher::new(bot);

    dp.on_command("/start", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_markdown_to_chat(message.chat_id(), "Hello! 👋")
                .await?;
        }
        Ok(())
    });

    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(no text)").to_string();
            ctx.bot.send_text_to_chat(message.chat_id(), text).await?;
        }
        Ok(())
    });

    dp.start_polling().await;
}
```

```bash
MAX_BOT_TOKEN=your_token cargo run --example echo_bot
```

## API methods

| Method | Description |
|--------|-------------|
| `bot.get_me()` | Bot info |
| `bot.send_text_to_chat(chat_id, text)` | Send plain text to a dialog/group/channel by `chat_id` |
| `bot.send_text_to_user(user_id, text)` | Send plain text to a user by global MAX `user_id` |
| `bot.send_markdown_to_chat(chat_id, text)` | Send Markdown to a dialog/group/channel by `chat_id` |
| `bot.send_markdown_to_user(user_id, text)` | Send Markdown to a user by global MAX `user_id` |
| `bot.send_message_to_chat(chat_id, body)` | Send message with attachments / keyboard by `chat_id` |
| `bot.send_message_to_user(user_id, body)` | Send message with attachments / keyboard by global MAX `user_id` |
| `bot.send_message_to_chat_with_options(chat_id, body, options)` | Send with query options such as `disable_link_preview` |
| `bot.edit_message(mid, body)` | Edit a message |
| `bot.delete_message(mid)` | Delete a message |
| `bot.get_messages_by_ids(ids, …)` | Get one or more messages by message IDs |
| `bot.get_video(video_token)` | Get video metadata and playback URLs |
| `bot.answer_callback(body)` | Answer an inline button press |
| `bot.get_chat(chat_id)` | Chat info |
| `bot.get_chats(…)` | List all group chats |
| `bot.edit_chat(chat_id, body)` | Edit chat title / description |
| `bot.leave_chat(chat_id)` | Leave a chat |
| `bot.get_members(…)` | List members |
| `bot.get_members_by_ids(chat_id, user_ids)` | Get selected members |
| `bot.add_members(…)` | Add members |
| `bot.remove_member(…)` | Remove a member |
| `bot.get_admins(chat_id)` | List admins |
| `bot.add_admins(chat_id, admins)` | Grant admin rights |
| `bot.remove_admin(chat_id, user_id)` | Revoke admin rights |
| `bot.pin_message(…)` | Pin a message |
| `bot.unpin_message(…)` | Unpin |
| `bot.send_sender_action(chat_id, action)` | Send a typed sender action |
| `bot.subscribe(body)` | Register a webhook |
| `bot.get_upload_url(type)` | Get upload URL |
| `bot.upload_file(type, path, name, mime)` | Full two-step file upload |
| `bot.upload_bytes(type, bytes, name, mime)` | Same, from bytes |
| `bot.send_image_to_chat(...)` | Upload and send an image |
| `bot.send_video_to_chat(...)` | Upload and send a video |
| `bot.send_audio_to_chat(...)` | Upload and send audio |
| `bot.send_file_to_chat(...)` | Upload and send a generic file |
| `bot.set_my_commands(commands)` | Experimental: public MAX API currently returns `404` for `/me/commands` |

## User ID vs Chat ID

These two IDs are different and should not be used interchangeably:

- `user_id` is the global MAX ID of a user.
- `chat_id` is the ID of a concrete dialog, group, or channel.
- In a private chat, `message.sender.user_id` identifies the user, while `message.chat_id()` identifies that specific dialog with the bot.
- Use `send_text_to_chat(chat_id, ...)` / `send_message_to_chat(chat_id, ...)` when you already know the dialog or group.
- Use `send_text_to_user(user_id, ...)` / `send_message_to_user(user_id, ...)` when you only know the user's global MAX ID.

## Known MAX platform gaps

As of April 27, 2026, the crate can send these requests, but live behavior on the MAX side is still inconsistent:

- `Button::RequestContact` is documented by MAX, but live tests received a contact attachment with empty `contact_id` and `vcf_phone`. Sending the button works; receiving the user's phone number is not confirmed on the MAX side.
- `Button::RequestGeoLocation` delivers a structured `Attachment::Location` with `latitude` and `longitude`; the client can render the same shared position as a Yandex Maps card.
- `bot.send_sender_action(chat_id, SenderAction::TypingOn)` returns success from the API, but live MAX tests did not confirm a visible typing indicator in the client.
- `bot.set_my_commands` is kept as an experimental helper, but the public MAX REST docs do not list a write endpoint for bot commands, and live `POST /me/commands` requests return `404 Path /me/commands is not recognized`.

## Dispatcher filters

```rust
dp.on_command("/start", handler);             // specific command
dp.on_message(handler);                       // any new message
dp.on_edited_message(handler);               // edited message
dp.on_callback(handler);                     // any callback button
dp.on_callback_payload("btn:ok", handler);   // specific payload
dp.on_bot_started(handler);                  // user starts bot
dp.on_update(
    Filter::message() & Filter::chat(chat_id) & Filter::text_contains("ping"),
    handler,
);                                           // composable filters
dp.on_start(handler);                        // runs before polling starts
dp.task(Duration::from_secs(60), handler);   // periodic task
dp.on_raw_update(handler);                   // raw JSON for every update
dp.on_filter(|u| { … }, handler);            // custom predicate
dp.on(handler);                              // every update
```

First matching handler wins. Register more specific filters before general ones.

## Inline keyboard

```rust
use maxoxide::types::{Button, KeyboardPayload, NewMessageBody};

let keyboard = KeyboardPayload {
    buttons: vec![
        vec![
            Button::callback("Yes ✅", "answer:yes"),
            Button::callback("No ❌",  "answer:no"),
        ],
        vec![Button::link("🌐 Website", "https://max.ru")],
    ],
};

let body = NewMessageBody::text("Are you sure?").with_keyboard(keyboard);
bot.send_message_to_chat(chat_id, body).await?;
```

Clipboard buttons copy their `payload` in the MAX client. They do not send a
callback update to the bot:

```rust
let keyboard = KeyboardPayload {
    buttons: vec![vec![Button::clipboard(
        "Copy invite code",
        "MAXOXIDE-2026",
    )]],
};

let body = NewMessageBody::empty().with_keyboard(keyboard);
bot.send_message_to_chat(chat_id, body).await?;
```

## File upload

Max uses a two-step upload flow. `upload_file` / `upload_bytes` handle it automatically:

```rust
use maxoxide::types::{NewAttachment, NewMessageBody, UploadType};

let token = bot
    .upload_file(UploadType::Image, "./photo.jpg", "photo.jpg", "image/jpeg")
    .await?;

let body = NewMessageBody {
    text: Some("Here's the photo!".into()),
    attachments: Some(vec![NewAttachment::image(token)]),
    ..Default::default()
};
bot.send_message_to_chat(chat_id, body).await?;
// or:
// bot.send_message_to_user(user_id, body).await?;
```

> **Note:** `type=photo` was removed from the Max API. Always use `UploadType::Image`.

For common cases, use upload-and-send helpers:

```rust
bot.send_image_to_chat(chat_id, "./photo.jpg", "photo.jpg", "image/jpeg", None).await?;
bot.send_video_to_chat(chat_id, "./clip.mp4", "clip.mp4", "video/mp4", None).await?;
bot.send_audio_to_chat(chat_id, "./track.mp3", "track.mp3", "audio/mpeg", None).await?;
bot.send_file_to_chat(chat_id, "./report.pdf", "report.pdf", "application/pdf", None).await?;
```

There are matching `*_to_user` and `*_bytes_*` helpers.

MAX can take a short time to finish processing uploaded attachments after the upload request succeeds. The upload-and-send helpers retry briefly when the API reports that an attachment is not processed yet.

Image uploads can return a MAX `photos` token map instead of a single token. The `send_image_*` helpers preserve that payload automatically.

## Webhook server (`features = ["webhook"]`)

```rust
use maxoxide::webhook::WebhookServer;
use maxoxide::types::SubscribeBody;

bot.subscribe(SubscribeBody {
    url: "https://your-domain.com/webhook".into(),
    update_types: None,
    version: None,
    secret: Some("my_secret_123".into()),
}).await?;

WebhookServer::new(dp)
    .secret("my_secret_123")
    .path("/webhook")
    .serve("0.0.0.0:8443")
    .await;
```

> Max requires HTTPS on port 443 and does **not** support self-signed certificates.

Use webhooks when your bot has a public HTTPS endpoint and you want MAX to deliver updates via incoming requests. For local development and simple launches, long polling is usually enough.

## Project layout

```
maxoxide/
├── Cargo.toml
├── src/
│   ├── lib.rs          — public API & re-exports
│   ├── bot.rs          — Bot + all HTTP methods
│   ├── uploader.rs     — two-step file upload helpers
│   ├── dispatcher.rs   — Dispatcher, Filter, Context
│   ├── errors.rs       — MaxError
│   ├── webhook.rs      — axum webhook server (feature = "webhook")
│   ├── tests.rs        — unit tests
│   └── types.rs        — all types (User, Chat, Message, Update, …)
└── examples/
    ├── echo_bot.rs
    ├── dispatcher_filters_bot.rs
    ├── keyboard_bot.rs
    ├── media_bot.rs
    ├── live_api_test.rs
    └── webhook_bot.rs  (feature = "webhook")
```

## Running tests

```bash
cargo test
```

## Live API test

For real-data verification there is a separate interactive harness:

```bash
cargo run --example live_api_test
```

At startup it asks in the terminal for:

- bot token
- bot URL for the tester
- optional webhook URL and secret
- optional local file path for `upload_file`
- optional image, video and audio paths for media helper checks
- HTTP timeout, polling timeout and delay between requests

The harness then walks the tester through Max-client actions and records `PASS` / `FAIL` / `SKIP` for real API calls. It uses small delays between requests, drains the long-poll backlog before the run, and asks for explicit confirmation before destructive or non-reversible steps such as:

- `set_my_commands`
- `delete_chat`
- `leave_chat`
- visible group title edits

The current run also probes the unclear MAX behavior around contact/location request buttons, message buttons, `open_app`, `clipboard`, sender actions, uploaded video metadata, selected members, and temporary admin rights changes.

## License

[MIT](https://choosealicense.com/licenses/mit/)
