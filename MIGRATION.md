# Migrating from maxoxide 2.x to 3.0

## English

Version 3.0 intentionally removes obsolete compatibility APIs and makes configuration failures explicit. MSRV is now Rust 1.85.

### Constructors are fallible

Before:

```rust
let bot = Bot::from_env();
let bot = Bot::new(token);
let bot = Bot::with_client(token, client);
```

After:

```rust
let bot = Bot::from_env()?;
let bot = Bot::new(token)?;
let bot = Bot::with_client(token, client)?;
```

This catches empty/invalid tokens, unsafe URLs, zero timeouts, invalid proxy settings, and TLS/client build failures before an API call.

### Structured API errors

Before:

```rust
MaxError::Api { code, message }
```

After:

```rust
MaxError::Api(ApiError {
    status,
    code,          // Option<String> MAX machine code
    message,
    raw_response,  // bounded to 4 KiB
    retry_after,
})
```

Match `error.status == 404` instead of a numeric `code` field. MAX machine codes such as `attachment.not.ready` are strings.

### Bot info and commands

- `get_me()` returns `BotInfo`.
- `BotCommand::description` is `Option<String>`.
- Use `BotCommand::new(name, description)` or `BotCommand::named(name)`.
- `set_my_commands` now uses documented `PATCH /me/commands` and returns `BotCommands`.
- The 32-command limit is validated locally.
- `edit_my_info` and `EditMyInfoBody` were removed because the current public Bot API documents command editing, not a generic bot profile PATCH.

### Removed chat list API

`Bot::get_chats` and `ChatList` were removed. MAX stopped supporting `GET /chats`. Persist `chat_id` from `bot_added`, `bot_started`, message/lifecycle updates, and remove records according to your own membership lifecycle.

### Buttons and messages

- `ButtonIntent` and all `intent` fields were removed. Official clients mark them unsupported.
- `Recipient` adds optional `post_id`; update struct literals with `post_id: None`.
- `Message::constructor` is now `Option<User>`, not arbitrary JSON.
- `NewAttachment` adds outgoing sticker, contact, location, and share variants.
- Send/edit/callback/comment methods validate text, attachment, and keyboard constraints.
- Use HTML helpers and long-message helpers for the corresponding behavior.

### Updates

Handle the new variants if matching exhaustively inside the crate version:

- `MessageConstructionRequest`
- `MessageConstructed`
- `BotStopped` now includes `payload: Option<String>`

`Update` remains `#[non_exhaustive]`; external matches already require `_`.

### Uploads

File-based helpers no longer load complete files into a `Vec<u8>`. They stream multipart data and automatically use resumable `Content-Range` chunks when MAX supplies a token. Configure progress, cancellation, timeout, chunk size, and chunk attempts with `UploadOptions`.

`upload_bytes` remains memory-backed by design.

### Dispatcher

- `start_polling()` returns `Result<()>`; propagate it with `.await?` or return it.
- `dispatch()` and `dispatch_raw()` return `Result<()>`.
- `on_error` receives `&MaxError` instead of owning the error.
- Command filters use exact first-token matching, not arbitrary string prefix matching.
- Middleware uses `Next::run(context)`.
- Typed state is registered with `with_state` and retrieved with `Context::state`.
- Polling handles Ctrl+C and supports `shutdown_handle`, bounded concurrency, and a drain timeout.

### Webhook features

The old `webhook` feature and `WebhookServer` were removed.

Choose one or both adapters:

```toml
maxoxide = { version = "3", features = ["webhook-axum"] }
# or
maxoxide = { version = "3", features = ["webhook-actix"] }
```

Create `WebhookService`, configure secret/limits once, then call `axum_adapter::router` or `actix_adapter::scope`. Server binding and TLS termination remain under application control.

### New modules

- `miniapp`: strict init data and `requestContact()` verification.
- `digital_id` behind `digital-id`: experimental partner client with separate credentials; validate the private onboarding contract before production use.
- Comments are typed but remain experimental because MAX marks the endpoints temporarily unavailable.

## Русский

Версия 3.0 намеренно удаляет устаревшие compatibility APIs и делает ошибки конфигурации явными. MSRV повышен до Rust 1.85.

### Fallible constructors

Было:

```rust
let bot = Bot::from_env();
let bot = Bot::new(token);
let bot = Bot::with_client(token, client);
```

Стало:

```rust
let bot = Bot::from_env()?;
let bot = Bot::new(token)?;
let bot = Bot::with_client(token, client)?;
```

До API request обнаруживаются пустой/некорректный token, unsafe URL, нулевой timeout, неверный proxy и ошибки TLS/client build.

### Structured API errors

Было `MaxError::Api { code, message }`. Стало `MaxError::Api(ApiError)`, где есть HTTP `status`, строковый optional MAX `code`, `message`, ограниченный 4 КиБ `raw_response` и `retry_after`. Для 404 проверяйте `error.status`, а не старое числовое поле `code`.

### Bot info и команды

- `get_me()` возвращает `BotInfo`.
- `BotCommand::description` стал `Option<String>`; используйте `BotCommand::new` или `BotCommand::named`.
- `set_my_commands` вызывает `PATCH /me/commands`, возвращает `BotCommands` и проверяет лимит 32.
- Удалены `edit_my_info` и `EditMyInfoBody`: публичный API документирует изменение команд, а не общий PATCH профиля.

### Удалённый список чатов

Удалены `Bot::get_chats` и `ChatList`. MAX прекратил поддержку `GET /chats`. Храните `chat_id` из `bot_added`, `bot_started`, message/lifecycle updates и обновляйте собственный membership lifecycle.

### Кнопки и сообщения

- Удалены `ButtonIntent` и поля `intent`: официальные клиенты считают их неподдерживаемыми.
- В `Recipient` добавлен optional `post_id`; добавьте `post_id: None` в struct literals.
- `Message::constructor` теперь `Option<User>`, а не произвольный JSON.
- `NewAttachment` получил исходящие sticker, contact, location и share.
- Send/edit/callback/comment methods локально проверяют text, attachments и keyboard.
- Добавлены HTML и long-message helpers.

### Updates

Добавлены `MessageConstructionRequest`, `MessageConstructed`; в `BotStopped` появилось `payload: Option<String>`. `Update` остаётся `#[non_exhaustive]`.

### Upload

File helpers больше не читают файл целиком в `Vec<u8>`. Multipart отправляется stream-ом, а при token от MAX автоматически используется resumable `Content-Range`. Progress, cancellation, timeout, chunk size и attempts задаются через `UploadOptions`. `upload_bytes` по определению остаётся memory-backed.

### Dispatcher

- `start_polling()`, `dispatch()` и `dispatch_raw()` возвращают `Result<()>`.
- `on_error` получает `&MaxError`.
- Command filter сравнивает точный первый token, а не любой prefix.
- Middleware продолжает цепочку через `Next::run(context)`.
- State регистрируется `with_state` и читается `Context::state`.
- Polling обрабатывает Ctrl+C, bounded concurrency, `shutdown_handle` и drain timeout.

### Webhook features

Удалены старые feature `webhook` и `WebhookServer`. Включите `webhook-axum` и/или `webhook-actix`, создайте общий `WebhookService`, затем используйте `axum_adapter::router` или `actix_adapter::scope`. Binding server и TLS остаются под контролем приложения.

### Новые модули

- `miniapp`: строгая проверка init data и `requestContact()`.
- `digital_id` за feature `digital-id`: experimental partner client с независимыми credentials; перед production проверьте private onboarding contract.
- Comments типизированы, но остаются experimental, пока MAX помечает endpoints временно недоступными.
