# ![MAX logo](./max_logo.png "MAX logo") maxoxide

[![CI](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml/badge.svg)](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/maxoxide.svg)](https://crates.io/crates/maxoxide)
[![docs.rs](https://docs.rs/maxoxide/badge.svg)](https://docs.rs/maxoxide)

Production-oriented async Rust SDK for the [MAX messenger](https://max.ru) Bot API.

[Russian README](README.ru.md) | [English guides](docs/en/README.md) | [Russian guides](docs/ru/README.md) | [API support](API_SUPPORT.md) | [v3 migration](MIGRATION.md)

## Install

```toml
[dependencies]
maxoxide = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Optional features:

| Feature | Adds |
|---|---|
| `webhook-axum` | Framework-neutral webhook core and Axum adapter |
| `webhook-actix` | Framework-neutral webhook core and Actix Web adapter |
| `digital-id` | Experimental MAX Digital ID age-verification partner client |
| `socks-proxy` | SOCKS support in the built-in `reqwest` client |

MSRV is Rust 1.85.

## Quick Start

```rust
use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let bot = Bot::from_env()?; // MAX_BOT_TOKEN
    let mut dispatcher = Dispatcher::new(bot);

    dispatcher.on_message(|context: Context| async move {
        if let Update::MessageCreated { message, .. } = &context.update {
            context
                .bot
                .send_text_to_chat(message.chat_id(), message.text().unwrap_or(""))
                .await?;
        }
        Ok(())
    });

    dispatcher.start_polling().await
}
```

```bash
MAX_BOT_TOKEN=... cargo run --example quickstart_bot
```

MAX recommends HTTPS webhooks for production. Long polling is intended for development and testing.

## Highlights

- Typed messages, chats, members, callbacks, constructed-message events, comments, subscriptions, and uploads.
- Structured `ApiError` with MAX code, bounded raw diagnostics, and `Retry-After` support.
- Secure builder with embedded Russian Trusted Root CA, configurable timeouts, HTTP/SOCKS proxy, retries, and base URL.
- Local 30 RPS global and per-recipient message-operation limits.
- Bounded-memory multipart streaming and resumable `Content-Range` uploads with chunk retry, progress, and cancellation.
- Exact command parsing, composable filters, middleware, typed state, bounded concurrency, scheduled tasks, and graceful shutdown.
- One webhook service shared by first-class Axum and Actix adapters.
- Mini App HMAC/freshness validation and `requestContact()` signature verification.
- Experimental feature-gated Digital ID partner client isolated from the bot token and Bot API client.
- Unknown enum/update preservation where MAX can extend wire values.

## Guides

| Topic | English | Russian |
|---|---|---|
| Getting started and IDs | [Guide](docs/en/getting-started.md) | [Руководство](docs/ru/getting-started.md) |
| Client, TLS, proxy, retry, limits | [Guide](docs/en/client-configuration.md) | [Руководство](docs/ru/client-configuration.md) |
| Messages, formatting, attachments, uploads | [Guide](docs/en/messages-and-media.md) | [Руководство](docs/ru/messages-and-media.md) |
| Dispatcher, middleware, state, shutdown | [Guide](docs/en/dispatcher.md) | [Руководство](docs/ru/dispatcher.md) |
| Axum and Actix webhooks | [Guide](docs/en/webhooks.md) | [Руководство](docs/ru/webhooks.md) |
| Mini Apps | [Guide](docs/en/mini-apps.md) | [Руководство](docs/ru/mini-apps.md) |
| Digital ID | [Guide](docs/en/digital-id.md) | [Руководство](docs/ru/digital-id.md) |
| Live API validation | [Guide](docs/en/live-api-test.md) | [Руководство](docs/ru/live-api-test.md) |
| Production | [Guide](docs/en/production.md) | [Руководство](docs/ru/production.md) |
| Recipes | [Index](docs/en/recipes/README.md) | [Список](docs/ru/recipes/README.md) |

## Examples

The repository contains 25 runnable examples:

- Basics: `quickstart_bot`, `echo_bot`, `commands_bot`, `command_arguments`.
- Dispatcher: `dispatcher_filters_bot`, `dispatcher_middleware`, `scheduled_tasks`, `graceful_polling`.
- Messages: `keyboard_gallery`, `callback_actions`, `message_formatting`, `long_messages`, `all_attachments`.
- Uploads and networking: `streaming_upload`, `resumable_upload`, `custom_client_proxy`.
- Webhooks: `webhook_axum_integration`, `webhook_actix`, `webhook_actix_integration`.
- Platform APIs: `comments_moderation`, `miniapp_validation`, `digital_id`, `raw_api`, `error_handling`.
- Advanced integration: `live_api_test`.

`live_api_test` lets library users validate real Bot API behavior with their own dedicated test bot. Run it from a maxoxide source checkout matching your dependency version and review its real side effects first; see the [live API validation guide](docs/en/live-api-test.md).

Run any non-feature example with:

```bash
cargo run --example <name>
```

Feature examples require the matching flag, for example:

```bash
cargo run --example webhook_actix --features webhook-actix
cargo run --example digital_id --features digital-id
```

## Important Constraints

- Use `chat_id` for a concrete dialog/group/channel and `user_id` for a global MAX user.
- Never send a bot token in a query parameter. maxoxide uses the `Authorization` header.
- Do not disable TLS verification. Custom clients can call `RussianTlsExt::russian_tls()`.
- `Bot::add_members` is deprecated because MAX removed that endpoint on September 30, 2026 without a Bot API replacement.
- Digital ID is an experimental partner integration without live service verification. Its payload schemas are issued during onboarding; validate the endpoint, authorization, and caller-owned serde models against the current partner documentation before production use.

See [SECURITY.md](SECURITY.md), [API_SUPPORT.md](API_SUPPORT.md), and [CHANGELOG.md](CHANGELOG.md) before production deployment.

## License

MIT
