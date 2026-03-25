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

- ✅ Full coverage of the Max Bot REST API
- ✅ Long polling (dev & test) and **Webhook** via [axum](https://github.com/tokio-rs/axum) (production)
- ✅ Strongly-typed events (`Update`, `Message`, `Callback`, …)
- ✅ `Dispatcher` with fluent handler registration and filters
- ✅ Inline keyboards (all button types: `callback`, `link`, `message`, `request_contact`, `request_geo_location`)
- ✅ File uploads — multipart, correct token flow for video/audio
- ✅ Markdown / HTML message formatting
- ✅ Webhook secret verification (`X-Max-Bot-Api-Secret`)
- ✅ Tokio async throughout

## Quick start

```toml
[dependencies]
maxoxide = "0.1"
tokio    = { version = "1", features = ["full"] }

# For webhook support (production):
# maxoxide = { version = "0.1", features = ["webhook"] }
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
            ctx.bot.send_markdown(message.chat_id(), "Hello! 👋").await?;
        }
        Ok(())
    });

    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(no text)").to_string();
            ctx.bot.send_text(message.chat_id(), text).await?;
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
| `bot.send_text(chat_id, text)` | Send plain text |
| `bot.send_markdown(chat_id, text)` | Send Markdown |
| `bot.send_message(chat_id, body)` | Send message with attachments / keyboard |
| `bot.edit_message(mid, body)` | Edit a message |
| `bot.delete_message(mid)` | Delete a message |
| `bot.answer_callback(body)` | Answer an inline button press |
| `bot.get_chat(chat_id)` | Chat info |
| `bot.get_chats(…)` | List all group chats |
| `bot.edit_chat(chat_id, body)` | Edit chat title / description |
| `bot.leave_chat(chat_id)` | Leave a chat |
| `bot.get_members(…)` | List members |
| `bot.add_members(…)` | Add members |
| `bot.remove_member(…)` | Remove a member |
| `bot.get_admins(chat_id)` | List admins |
| `bot.pin_message(…)` | Pin a message |
| `bot.unpin_message(…)` | Unpin |
| `bot.send_action(chat_id, "typing")` | Typing indicator |
| `bot.subscribe(body)` | Register a webhook |
| `bot.get_upload_url(type)` | Get upload URL |
| `bot.upload_file(type, path, name, mime)` | Full two-step file upload |
| `bot.upload_bytes(type, bytes, name, mime)` | Same, from bytes |
| `bot.set_my_commands(commands)` | Set bot commands |

## Dispatcher filters

```rust
dp.on_command("/start", handler);             // specific command
dp.on_message(handler);                       // any new message
dp.on_edited_message(handler);               // edited message
dp.on_callback(handler);                     // any callback button
dp.on_callback_payload("btn:ok", handler);   // specific payload
dp.on_bot_started(handler);                  // user starts bot
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
bot.send_message(chat_id, body).await?;
```

## File upload

Max uses a two-step upload flow. `upload_file` / `upload_bytes` handle it automatically:

```rust
use maxoxide::types::{NewAttachment, NewMessageBody, UploadType, UploadedToken};

let token = bot
    .upload_file(UploadType::Image, "./photo.jpg", "photo.jpg", "image/jpeg")
    .await?;

let body = NewMessageBody {
    text: Some("Here's the photo!".into()),
    attachments: Some(vec![NewAttachment::Image {
        payload: UploadedToken { token },
    }]),
    ..Default::default()
};
bot.send_message(chat_id, body).await?;
```

> **Note:** `type=photo` was removed from the Max API. Always use `UploadType::Image`.

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
│   └── types/
│       └── mod.rs      — all types (User, Chat, Message, Update, …)
└── examples/
    ├── echo_bot.rs
    ├── keyboard_bot.rs
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
cargo кг
```

At startup it asks in the terminal for:

- bot token
- bot URL for the tester
- optional webhook URL and secret
- optional local file path for `upload_file`
- HTTP timeout, polling timeout and delay between requests

The harness then walks the tester through Max-client actions and records `PASS` / `FAIL` / `SKIP` for real API calls. It uses small delays between requests, drains the long-poll backlog before the run, and asks for explicit confirmation before destructive or non-reversible steps such as:

- `set_my_commands`
- `delete_chat`
- `leave_chat`
- visible group title edits

## License

[MIT](https://choosealicense.com/licenses/mit/)
