# Changelog

All notable changes to this project will be documented in this file.

## [3.0.0] - 2026-08-21

### EN

#### Release summary

This major release turns `maxoxide` into a production-oriented MAX SDK with validated client configuration, structured failures, retries and rate limits, bounded-memory uploads, a composable dispatcher, framework-neutral webhooks, and current MAX wire contracts. Obsolete compatibility APIs are intentionally removed; see `MIGRATION.md` when upgrading from 2.x.

#### Breaking changes

- `Bot::new`, `Bot::from_env`, and `Bot::with_client` now return `Result<Bot>` instead of constructing or panicking unconditionally.
- `MaxError::Api { code, message }` was replaced by `MaxError::Api(ApiError)`. `ApiError` separates HTTP `status` from the optional string MAX `code` and also exposes a bounded `raw_response` and `retry_after`.
- `Bot::get_me()` now returns `BotInfo`. `BotCommand::description` is optional, and `Bot::set_my_commands` now uses the documented `PATCH /me/commands` route and returns `BotCommands`.
- Removed `Bot::edit_my_info` and `EditMyInfoBody`; the current public API documents command replacement rather than generic bot-profile editing.
- Removed `Bot::get_chats` and `ChatList` because MAX no longer supports `GET /chats`. Applications must persist `chat_id` values received from updates.
- Removed `ButtonIntent` and all button `intent` fields because current official clients mark them unsupported.
- `Recipient` adds optional `post_id`, and `Message::constructor` is now `Option<User>` instead of arbitrary JSON. Existing struct literals and direct field handling may need updates.
- `Dispatcher::start_polling`, `dispatch`, and `dispatch_raw` now return `Result<()>`; `on_error` receives `&MaxError`.
- Removed the old `webhook` feature and `WebhookServer`. Applications must select `webhook-axum` and/or `webhook-actix` and host the shared `WebhookService` themselves.
- The minimum supported Rust version is explicitly Rust 1.85.

#### Added

- Added `BotBuilder` with validated base URLs, request/connect/upload timeouts, custom `reqwest::Client`, HTTP/HTTPS proxy support, optional SOCKS proxy support, secure TLS roots, and `RetryPolicy`. `BotBuilder::no_proxy()` can disable automatic system proxies for the MAX client, while `BotBuilder::proxy` accepts authenticated and selectively bypassed reqwest proxies.
- Added global and recipient/resource-specific MAX rate limiting, bounded limiter eviction, retry backoff, and `Retry-After` handling.
- Added `Bot::execute` as a guarded escape hatch for relative Bot API endpoints not yet represented by typed methods.
- Added current MAX models and wire values including `Recipient.post_id`, typed message constructors, `MessageConstructionRequest`, `MessageConstructed`, `bot_stopped.payload`, and `ChatAdminPermission::ViewStats`.
- Added typed outgoing sticker, contact, location, and share attachments together with request validation for messages, keyboards, comments, and attachment payloads.
- Added five typed experimental comment methods. MAX still marks the comments API temporarily unavailable, so applications must handle availability failures.
- Added HTML send helpers and Unicode-safe long-message helpers that split text at MAX's 4000-character limit.
- Added streaming multipart uploads and resumable `Content-Range` uploads with configurable chunk size, per-chunk retry, timeout, progress callbacks, and cancellation.
- Added Dispatcher middleware, typed application state, exact command matching and arguments, construction-event filters, bounded concurrency, programmatic shutdown, Ctrl+C handling, and drain timeouts.
- Added framework-neutral webhook processing with constant-time secret verification, body/in-flight limits, dispatch timeout and backpressure, plus optional Axum and Actix adapters.
- Added strict Mini App init-data and contact validation, plus an experimental feature-gated Digital ID partner client with credentials isolated from the Bot API client. The integration requires validation against the private onboarding contract before production use.
- Added 25 runnable examples with bilingual usage and prerequisite notes, paired English/Russian guides and recipes, `MIGRATION.md`, `API_SUPPORT.md`, and `SECURITY.md`.

#### Changed

- All Bot API requests now share one execution path for authorization, secure endpoint resolution, rate limiting, retries, structured errors, and bounded diagnostics without logging response bodies or signed upload URLs.
- File-based upload helpers no longer buffer complete files in memory; `upload_bytes` remains memory-backed by design.
- Unknown update, attachment, markup, and extensible string-enum values are preserved where practical for forward compatibility.
- Command filters now match the exact first token instead of arbitrary prefixes, and `Context::command_arguments` exposes the remaining text.
- The crate version was bumped to `3.0.0`.

### RU

#### Кратко о релизе

Этот major-релиз превращает `maxoxide` в production-oriented SDK для MAX: добавляет проверяемую конфигурацию клиента, structured errors, retry и rate limits, загрузки с ограниченным потреблением памяти, составной Dispatcher, framework-neutral Webhook и актуальные wire-контракты MAX. Устаревшие compatibility APIs намеренно удалены; при переходе с 2.x используйте `MIGRATION.md`.

#### Ломающие изменения

- `Bot::new`, `Bot::from_env` и `Bot::with_client` теперь возвращают `Result<Bot>` вместо безусловного создания или panic.
- `MaxError::Api { code, message }` заменён на `MaxError::Api(ApiError)`. В `ApiError` HTTP `status` отделён от optional строкового MAX `code`, также доступны ограниченный `raw_response` и `retry_after`.
- `Bot::get_me()` теперь возвращает `BotInfo`. Поле `BotCommand::description` стало optional, а `Bot::set_my_commands` использует документированный route `PATCH /me/commands` и возвращает `BotCommands`.
- Удалены `Bot::edit_my_info` и `EditMyInfoBody`: актуальный публичный API документирует замену команд, а не общий edit профиля бота.
- Удалены `Bot::get_chats` и `ChatList`, потому что MAX больше не поддерживает `GET /chats`. Приложение должно сохранять `chat_id` из полученных updates.
- Удалены `ButtonIntent` и все поля `intent`, которые актуальные официальные клиенты помечают как неподдерживаемые.
- В `Recipient` добавлен optional `post_id`, а `Message::constructor` теперь имеет тип `Option<User>` вместо произвольного JSON. Struct literals и прямую обработку полей может потребоваться обновить.
- `Dispatcher::start_polling`, `dispatch` и `dispatch_raw` теперь возвращают `Result<()>`; `on_error` получает `&MaxError`.
- Удалены старые feature `webhook` и `WebhookServer`. Приложение должно выбрать `webhook-axum` и/или `webhook-actix` и самостоятельно разместить общий `WebhookService`.
- Минимальная поддерживаемая версия Rust явно установлена на Rust 1.85.

#### Добавлено

- Добавлен `BotBuilder` с проверкой base URL, request/connect/upload timeouts, custom `reqwest::Client`, HTTP/HTTPS proxy, optional SOCKS proxy, безопасными TLS roots и `RetryPolicy`. `BotBuilder::no_proxy()` позволяет отключить автоматические системные proxy для MAX client, а `BotBuilder::proxy` принимает authenticated reqwest proxy и proxy с selective exclusions.
- Добавлены глобальный и recipient/resource-specific MAX rate limiting, ограниченное вытеснение limiter-ов, retry backoff и поддержка `Retry-After`.
- Добавлен `Bot::execute` как защищённый escape hatch для относительных Bot API endpoints, ещё не представленных typed methods.
- Добавлены актуальные модели и wire-значения MAX: `Recipient.post_id`, typed constructor сообщения, `MessageConstructionRequest`, `MessageConstructed`, `bot_stopped.payload` и `ChatAdminPermission::ViewStats`.
- Добавлены исходящие sticker, contact, location и share attachments, а также validation сообщений, клавиатур, комментариев и attachment payloads.
- Добавлены пять typed experimental methods комментариев. MAX всё ещё помечает comments API временно недоступным, поэтому приложение должно обрабатывать ошибки доступности.
- Добавлены HTML helpers и Unicode-safe helpers длинных сообщений с разделением по лимиту MAX в 4000 символов.
- Добавлены streaming multipart и resumable `Content-Range` uploads с настройкой chunk size, retry текущего chunk, timeout, progress callbacks и cancellation.
- Dispatcher получил middleware, typed state приложения, точное сопоставление команд и arguments, filters construction events, bounded concurrency, programmatic shutdown, обработку Ctrl+C и drain timeout.
- Добавлена framework-neutral обработка webhook с constant-time проверкой secret, лимитами body/in-flight, dispatch timeout и backpressure, а также optional Axum и Actix adapters.
- Добавлены строгая проверка Mini App init data/contact и experimental feature-gated partner client Digital ID с credentials, изолированными от Bot API client. Перед production интеграцию необходимо проверить по private onboarding contract.
- Добавлены 25 запускаемых примеров с двуязычными пояснениями по применению и prerequisites, парные EN/RU guides и recipes, `MIGRATION.md`, `API_SUPPORT.md` и `SECURITY.md`.

#### Изменено

- Все Bot API requests теперь проходят через единый execution path с авторизацией, безопасным разрешением endpoint, rate limiting, retries, structured errors и bounded diagnostics без логирования response body или signed upload URL.
- File-based upload helpers больше не буферизуют файлы целиком в памяти; `upload_bytes` по определению остаётся memory-backed.
- Неизвестные значения update, attachments, markup и расширяемых строковых enum по возможности сохраняются для forward compatibility.
- Command filters теперь сравнивают точный первый token вместо произвольного prefix, а `Context::command_arguments` возвращает оставшийся текст.
- Версия крейта повышена до `3.0.0`.

## [2.3.0] - 2026-07-13

### EN

#### Release summary

This compatible release adds a custom-client TLS helper for the current MAX API certificate chain and prepares applications for removal of the deprecated `GET /chats` endpoint.

#### Added

- Added `RussianTlsExt::russian_tls()` for `reqwest::ClientBuilder`, so custom clients can keep settings such as `timeout(...)` and `no_proxy()` while adding the embedded `Russian Trusted Root CA`.
- Added `Update::chat_id()` to extract a chat ID from typed updates when one is present. This helps applications maintain their own chat registry after MAX deprecated `GET /chats`.

#### Changed

- Bumped the crate version to `2.3.0`.
- Updated `live_api_test` to obtain group chat IDs from updates or explicit input instead of the deprecated `GET /chats` method.
- README and README.ru now document that `Bot::new()` and `Bot::from_env()` configure Russian TLS automatically, while `Bot::with_client(...)` custom clients should call `.russian_tls()` during `reqwest::ClientBuilder` setup.

#### Deprecated

- Deprecated `Bot::get_chats(...)`. MAX stopped supporting `GET /chats` in June 2026 and announced shutdown for August 2026. Store `chat_id` values from updates such as `bot_added`, `bot_started`, and message events in your own storage, remove them on `bot_removed`, and use chat-id-based methods.

### RU

#### Кратко о релизе

Совместимый релиз добавляет TLS-helper для custom clients под текущую цепочку сертификатов MAX API и подготавливает приложения к удалению deprecated endpoint `GET /chats`.

#### Добавлено

- Добавлен `RussianTlsExt::russian_tls()` для `reqwest::ClientBuilder`, чтобы custom clients сохраняли настройки вроде `timeout(...)` и `no_proxy()` и при этом добавляли встроенный `Russian Trusted Root CA`.
- Добавлен `Update::chat_id()` для извлечения chat ID из typed updates, если update его содержит. Это помогает приложениям вести собственный реестр чатов после deprecation `GET /chats`.

#### Изменено

- Версия крейта повышена до `2.3.0`.
- `live_api_test` теперь получает ID группового чата из updates или явного ввода вместо deprecated метода `GET /chats`.
- README и README.ru теперь документируют, что `Bot::new()` и `Bot::from_env()` настраивают Russian TLS автоматически, а custom clients для `Bot::with_client(...)` должны вызывать `.russian_tls()` на этапе настройки `reqwest::ClientBuilder`.

#### Deprecated

- Deprecated `Bot::get_chats(...)`. MAX перестал поддерживать `GET /chats` с июня 2026 года и объявил отключение в августе 2026. Сохраняйте `chat_id` из updates вроде `bot_added`, `bot_started` и message events в собственной БД, удаляйте их на `bot_removed` и используйте методы по `chat_id`.

## [2.2.0] - 2026-07-05

### EN

#### Release summary

This compatible release follows the current official MAX SDKs and schema by switching the default API host to `platform-api2.max.ru` and adding the newly documented channel lookup endpoint.

#### Added

- Added `Bot::get_chat_by_link(chat_link)` for `GET /chats/{chatLink}`. The official API documents this endpoint for channels by public link / username, for example `@channel`; live availability depends on MAX Bot API access to that channel.
- Added `Chat.participants` and `Chat.messages_count` fields from the current `Chat` schema.
- Added typed `ChatAdminPermission::Edit` and `ChatAdminPermission::Delete` variants for the current admin permission enum.
- Added automatic `Russian Trusted Root CA` handling for the default clients created by `Bot::new()` and `Bot::from_env()`: maxoxide tries to download the fresh PEM from the official `gu-st.ru` URL and falls back to an embedded copy while keeping TLS verification enabled.

#### Changed

- Switched the hardcoded API host from deprecated `https://platform-api.max.ru` to current `https://platform-api2.max.ru`.
- `Bot::get_chat_by_link` now accepts full `max.ru` URLs, plain channel names, and `@channel` names; full URLs are safely encoded as a single path segment and channel-name fallbacks are tried on `404`.
- Updated `live_api_test` to use the default client and include an optional `get_chat_by_link` scenario.

### RU

#### Кратко о релизе

Совместимый релиз, который следует актуальным официальным SDK и схеме MAX: переключает default API host на `platform-api2.max.ru` и добавляет новый endpoint получения канала по публичной ссылке.

#### Добавлено

- Добавлен `Bot::get_chat_by_link(chat_link)` для `GET /chats/{chatLink}`. Официальный API документирует этот endpoint для каналов по публичной ссылке / username, например `@channel`; live-доступность зависит от доступа MAX Bot API к этому каналу.
- Добавлены поля `Chat.participants` и `Chat.messages_count` из актуальной схемы `Chat`.
- Добавлены typed variants `ChatAdminPermission::Edit` и `ChatAdminPermission::Delete` для актуального enum прав администратора.
- Добавлена автоматическая поддержка `Russian Trusted Root CA` для default clients, созданных через `Bot::new()` и `Bot::from_env()`: maxoxide пытается скачать свежий PEM с официального URL `gu-st.ru` и fallback-ом использует встроенную копию, не отключая TLS verification.

#### Изменено

- Hardcoded API host переключён с deprecated `https://platform-api.max.ru` на актуальный `https://platform-api2.max.ru`.
- `Bot::get_chat_by_link` теперь принимает full `max.ru` URL, имя канала без префикса и `@channel`; full URL безопасно кодируется как один path segment, а варианты имени канала пробуются при `404`.
- `live_api_test` переведён на default client и получил optional-сценарий `get_chat_by_link`.

## [2.1.0] - 2026-05-20

### EN

#### Release summary

This compatible release tracks the May 2026 MAX Bot API updates without changing the existing `2.0.0` message/update method signatures.

#### Added

- Added typed update support for `bot_stopped`, `dialog_cleared`, `dialog_muted`, `dialog_unmuted`, `dialog_removed`, experimental `message_chat_created`, and nullable `message_edited` payloads via `Update::MessageEditedMissing`.
- Added `MarkupElement` parsing for strong, emphasized, monospaced, link, strikethrough, underline, user mention, heading, highlighted, and quote markup.
- Added `Button::Chat` plus builders for chat buttons.
- Added contact payload fields `hash` and `max_info`, `tam_info` alias compatibility, VCF phone extraction, and `ContactPayload::validate_hash(token)`.
- Added received `share` and `data` attachment variants and extra media fields such as video thumbnail/dimensions/duration and audio transcription.
- Added `Bot::edit_my_info`, `Bot::get_updates_with_types`, `Bot::get_updates_raw_with_types`, and `Bot::remove_member_with_options`.
- Added dispatcher filters and handler helpers for the newly typed updates.

#### Changed

- Expanded `live_api_test` with long-polling/webhook modes and optional scenarios for markup, contacts, dialog events, chat buttons, and group administration.
- The crate version was bumped to `2.1.0`.

#### Known MAX platform behavior

- `request_contact` can deliver `vcf_info`, a valid `hash`, and `max_info`; `vcf_phone` may still be empty, so `phones_from_vcf()` is the reliable fallback.
- `request_geo_location` delivers structured `Attachment::Location` coordinates.
- Documented `ChatButton` JSON is rejected by `POST /messages` with `400 Can't deserialize body`.
- `set_my_commands` remains a MAX platform limitation: public `POST /me/commands` requests return `404`.

### RU

#### Кратко о релизе

Совместимый релиз, который подтягивает изменения MAX Bot API за май 2026 без изменения существующих сигнатур сообщений, updates и методов из `2.0.0`.

#### Добавлено

- Добавлен typed-разбор updates `bot_stopped`, `dialog_cleared`, `dialog_muted`, `dialog_unmuted`, `dialog_removed`, experimental `message_chat_created` и nullable `message_edited` через `Update::MessageEditedMissing`.
- Добавлен `MarkupElement` для strong, emphasized, monospaced, link, strikethrough, underline, user mention, heading, highlighted и quote markup.
- Добавлен `Button::Chat` и builders для chat-кнопок.
- Добавлены поля contact payload `hash` и `max_info`, alias `tam_info`, извлечение телефонов из VCF и `ContactPayload::validate_hash(token)`.
- Добавлены variants вложений `share` и `data`, а также дополнительные поля media: thumbnail/размеры/duration для video и transcription для audio.
- Добавлены `Bot::edit_my_info`, `Bot::get_updates_with_types`, `Bot::get_updates_raw_with_types` и `Bot::remove_member_with_options`.
- Добавлены dispatcher filters и handler helpers для новых typed updates.

#### Изменено

- `live_api_test` расширен режимами long polling/webhook и optional-сценариями для markup, contacts, dialog events, chat buttons и администрирования групп.
- Версия крейта повышена до `2.1.0`.

#### Известное поведение платформы MAX

- `request_contact` может передавать `vcf_info`, валидный `hash` и `max_info`; `vcf_phone` всё ещё может быть пустым, поэтому `phones_from_vcf()` — надёжный fallback.
- `request_geo_location` передаёт структурированные координаты `Attachment::Location`.
- Документированный JSON `ChatButton` отклоняется `POST /messages` с `400 Can't deserialize body`.
- `set_my_commands` остаётся ограничением платформы MAX: публичные запросы `POST /me/commands` возвращают `404`.

## [2.0.0] - 2026-04-27

### EN

#### Release summary

This release aligns `maxoxide` with the current public MAX REST API, adds convenience helpers for media sending, makes update parsing more forward-compatible, and expands the dispatcher into a more practical routing layer.

#### Breaking changes

- `User::name` was replaced with MAX-style profile fields:
  - `first_name`
  - `last_name`
  - `username`
  - `description`
  - `avatar_url`
  - `full_avatar_url`
  - `commands`
- Use `User::display_name()` when the old code needs a single printable name.
- `Update::timestamp()` now returns `Option<i64>` because unknown future updates may omit a timestamp.
- Use `Update::timestamp_or_default()` when the previous `0` fallback behavior is desired.
- `MessageFormat::Plain` was removed. Plain text is represented by leaving `NewMessageBody::format` as `None`.
- `Button::open_app(...)` now follows the official Go SDK wire model with `web_app`, optional `payload`, and optional `contact_id` fields instead of an opaque JSON payload.
- `NewAttachment::Image` now carries `ImageAttachmentPayload` instead of `UploadedToken`, so it can serialize the official MAX `photos` token map returned by image uploads. `NewAttachment::image(token)` remains available for the simple token form.
- Public enums that mirror MAX wire values are now `#[non_exhaustive]`; downstream exhaustive matches need a wildcard arm.
- `src/types/mod.rs` was replaced by `src/types.rs`. The public path remains `maxoxide::types`.

#### Added

- Added typed fallback support for unknown `Update` and unknown attachments, preserving raw JSON for later inspection.
- Added attachment deserialization for both wrapped `payload` objects and flat attachment objects, so `Button::RequestGeoLocation` updates deserialize as `Attachment::Location` with `latitude` and `longitude`. The client can render the same shared position as a Yandex Maps card.
- Added typed string enums with unknown-value preservation:
  - `ChatType`
  - `ChatStatus`
  - `MessageFormat`
  - `ButtonIntent`
  - `LinkType`
  - `ChatAdminPermission`
  - `SenderAction`
- Added more complete MAX models for users, chats, members, admins, video metadata, photo payloads, and partial success results.
- Added `Button::OpenApp` using the official Go SDK fields `web_app`, `payload`, and `contact_id`.
- Added `Button::Clipboard`, which is present in the official Go SDK.
- Added builders for `NewMessageBody`, `NewAttachment`, and `UploadedToken`.
- Added `SendMessageOptions` with `disable_link_preview`.
- Added message, video, member, and admin endpoints:
  - `get_messages_by_ids`
  - `get_video`
  - `get_members_by_ids`
  - `add_admins`
  - `remove_admin`
- Added typed sender action methods:
  - `send_sender_action`
  - `send_typing_on`
  - `send_sending_image`
  - `send_sending_video`
  - `send_sending_audio`
  - `send_sending_file`
  - `mark_seen`
- Added upload-and-send helpers for both chat and user recipients:
  - `send_image_to_chat` / `send_image_to_user`
  - `send_video_to_chat` / `send_video_to_user`
  - `send_audio_to_chat` / `send_audio_to_user`
  - `send_file_to_chat` / `send_file_to_user`
  - byte-based variants for the same media types
- Added `Dispatcher::on_update`, composable `Filter` values, regex text filters, media/file attachment filters, `on_start`, `task`, `on_raw_update`, and raw polling via `get_updates_raw`.
- Added `examples/media_bot.rs` and `examples/dispatcher_filters_bot.rs`.

#### Changed

- `get_upload_url` now serializes upload types using the documented lowercase wire values.
- Long polling now receives raw update JSON first, then dispatches through raw and typed handlers.
- Webhook handling now dispatches raw JSON through the same dispatcher path as long polling.
- Upload helpers now accept attachment tokens from either the upload endpoint response or multipart upload response, preserve the MAX `photos` token map for image send helpers, and retry briefly while MAX reports an uploaded attachment as not processed yet.
- README examples now use builders and the new media helpers.
- The crate version was bumped to `2.0.0`.

### RU

#### Кратко о релизе

Этот релиз синхронизирует `maxoxide` с текущим публичным REST API MAX, добавляет helpers для отправки медиа, делает разбор обновлений устойчивее к будущим типам MAX и расширяет `Dispatcher` до более практичного роутинга.

#### Ломающие изменения

- `User::name` заменён на поля профиля в стиле MAX:
  - `first_name`
  - `last_name`
  - `username`
  - `description`
  - `avatar_url`
  - `full_avatar_url`
  - `commands`
- Если старому коду нужна одна строка для отображения имени, используйте `User::display_name()`.
- `Update::timestamp()` теперь возвращает `Option<i64>`, потому что неизвестные будущие update могут не содержать timestamp.
- Для старого поведения с fallback в `0` используйте `Update::timestamp_or_default()`.
- `MessageFormat::Plain` удалён. Обычный текст задаётся отсутствием `format` в `NewMessageBody`.
- `Button::open_app(...)` теперь следует wire-модели официального Go SDK с полями `web_app`, optional `payload` и optional `contact_id`, а не opaque JSON payload.
- `NewAttachment::Image` теперь хранит `ImageAttachmentPayload` вместо `UploadedToken`, чтобы сериализовать официальный MAX `photos` token map, который возвращают image uploads. `NewAttachment::image(token)` остаётся доступным для простой token-формы.
- Публичные enum, отражающие wire-значения MAX, теперь `#[non_exhaustive]`; во внешнем коде exhaustive `match` должны иметь wildcard arm.
- `src/types/mod.rs` заменён на `src/types.rs`. Публичный путь остаётся прежним: `maxoxide::types`.

#### Добавлено

- Добавлен fallback для неизвестных `Update` и неизвестных вложений с сохранением raw JSON.
- Добавлен разбор вложений как в wrapped `payload` форме, так и в плоской форме attachment object, поэтому updates от `Button::RequestGeoLocation` десериализуются как `Attachment::Location` с `latitude` и `longitude`. В клиенте та же отправленная позиция может отображаться как карточка Яндекс Карт.
- Добавлены типизированные строковые enum с сохранением неизвестных значений:
  - `ChatType`
  - `ChatStatus`
  - `MessageFormat`
  - `ButtonIntent`
  - `LinkType`
  - `ChatAdminPermission`
  - `SenderAction`
- Расширены модели MAX для пользователей, чатов, участников, администраторов, video metadata, photo payloads и частично успешных результатов.
- Добавлен `Button::OpenApp` с полями официального Go SDK: `web_app`, `payload`, `contact_id`.
- Добавлен `Button::Clipboard`, который есть в официальном Go SDK.
- Добавлены builders для `NewMessageBody`, `NewAttachment` и `UploadedToken`.
- Добавлен `SendMessageOptions` с `disable_link_preview`.
- Добавлены методы для сообщений, видео, участников и администраторов:
  - `get_messages_by_ids`
  - `get_video`
  - `get_members_by_ids`
  - `add_admins`
  - `remove_admin`
- Добавлены типизированные действия отправителя:
  - `send_sender_action`
  - `send_typing_on`
  - `send_sending_image`
  - `send_sending_video`
  - `send_sending_audio`
  - `send_sending_file`
  - `mark_seen`
- Добавлены helpers загрузки и отправки для chat/user адресатов:
  - `send_image_to_chat` / `send_image_to_user`
  - `send_video_to_chat` / `send_video_to_user`
  - `send_audio_to_chat` / `send_audio_to_user`
  - `send_file_to_chat` / `send_file_to_user`
  - byte-based варианты для тех же типов медиа
- Добавлены `Dispatcher::on_update`, составные `Filter`, regex-фильтры текста, фильтры media/file вложений, `on_start`, `task`, `on_raw_update` и raw polling через `get_updates_raw`.
- Добавлены `examples/media_bot.rs` и `examples/dispatcher_filters_bot.rs`.

#### Изменено

- `get_upload_url` теперь сериализует типы загрузки документированными lowercase wire-значениями.
- Long polling сначала получает raw JSON update, затем dispatch проходит через raw и typed handlers.
- Webhook теперь dispatchит raw JSON тем же путём, что и long polling.
- Upload helpers принимают attachment token как из ответа upload endpoint, так и из multipart upload response, сохраняют MAX `photos` token map для image send helpers и коротко ретраят отправку, пока MAX сообщает, что вложение ещё не обработано.
- Примеры README переведены на builders и новые media helpers.
- Версия крейта повышена до `2.0.0`.

## [1.0.0] - 2026-03-25

### EN

#### Release summary

This release promotes `maxoxide` from `0.1.0` to `1.0.0`, fixes several API mismatches, and makes message delivery APIs explicit about whether they target a `chat_id` or a `user_id`.

#### Breaking changes

- Removed the old shorthand methods:
  - `send_text`
  - `send_markdown`
  - `send_message`
- Added explicit recipient-specific methods:
  - `send_text_to_chat(chat_id, text)`
  - `send_text_to_user(user_id, text)`
  - `send_markdown_to_chat(chat_id, text)`
  - `send_markdown_to_user(user_id, text)`
  - `send_message_to_chat(chat_id, body)`
  - `send_message_to_user(user_id, body)`
- Migration for apps still on `0.1.0`:
  - Replace `send_text(chat_id, text)` with `send_text_to_chat(chat_id, text)`
  - Replace `send_markdown(chat_id, text)` with `send_markdown_to_chat(chat_id, text)`
  - Replace `send_message(chat_id, body)` with `send_message_to_chat(chat_id, body)`
  - If you only know a global MAX `user_id`, use the new `*_to_user(...)` methods

#### Added

- Added `live_api_test`, an interactive advanced example for exercising real Bot API behavior with a controlled test bot and chats.

#### Changed

- Clarified throughout the docs that:
  - `user_id` is the global MAX user identifier
  - `chat_id` is the identifier of a concrete dialog, group, or channel
- Updated README, README.ru, examples, and crate-level docs to use only the explicit `*_to_chat` / `*_to_user` APIs
- Reworked API tables so chat-targeted and user-targeted methods are listed side by side
- Bumped the crate version to `1.0.0`

#### Fixed

- Fixed `answer_callback` to send `callback_id` as a query parameter, matching the real MAX API
- Fixed `edit_message` to return `SimpleResult` instead of incorrectly deserializing a `Message`
- Switched HTTP response parsing to `bytes + String::from_utf8_lossy` to avoid crashes on invalid UTF-8
- Added lossy attachment deserialization so malformed or unknown attachments do not break entire update/message parsing
- Updated action handling to use the MAX action value `typing_on`

#### Known MAX platform behavior

- `request_contact` may deliver a contact attachment with empty `contact_id` and `vcf_phone`.
- `request_geo_location` may display a location card in the mobile client without delivering a matching update to the bot.
- `typing_on` may return a successful API response without a visible client-side typing indicator.
- `set_my_commands` remains experimental: `POST /me/commands` returns `404`, and the public MAX REST docs do not expose a documented write endpoint for command menu updates.

### RU

#### Кратко о релизе

Этот релиз переводит `maxoxide` с ветки `0.1.0` на `1.0.0`, исправляет несколько несовпадений с API и делает отправку сообщений явной по типу получателя: `chat_id` или `user_id`.

#### Ломающие изменения

- Удалены старые сокращённые методы:
  - `send_text`
  - `send_markdown`
  - `send_message`
- Добавлены явные методы по типу адресата:
  - `send_text_to_chat(chat_id, text)`
  - `send_text_to_user(user_id, text)`
  - `send_markdown_to_chat(chat_id, text)`
  - `send_markdown_to_user(user_id, text)`
  - `send_message_to_chat(chat_id, body)`
  - `send_message_to_user(user_id, body)`
- Миграция приложений со старой `0.1.0`:
  - Заменить `send_text(chat_id, text)` на `send_text_to_chat(chat_id, text)`
  - Заменить `send_markdown(chat_id, text)` на `send_markdown_to_chat(chat_id, text)`
  - Заменить `send_message(chat_id, body)` на `send_message_to_chat(chat_id, body)`
  - Если приложению известен только глобальный MAX `user_id`, использовать новые методы `*_to_user(...)`

#### Добавлено

- Добавлен `live_api_test`, интерактивный расширенный пример для проверки поведения реального Bot API с контролируемыми тестовым ботом и чатами.

#### Изменено

- Во всей документации явно зафиксировано:
  - `user_id` — глобальный идентификатор пользователя MAX
  - `chat_id` — идентификатор конкретного диалога, группы или канала
- README, README.ru, примеры и crate docs переведены только на явные методы `*_to_chat` / `*_to_user`
- Таблицы API перестроены так, чтобы chat-методы и user-методы стояли рядом
- Версия крейта повышена до `1.0.0`

#### Исправлено

- Исправлен `answer_callback`: теперь `callback_id` отправляется query-параметром, как требует реальный MAX API
- Исправлен `edit_message`: теперь метод возвращает `SimpleResult`, а не пытается неверно десериализовать `Message`
- Разбор HTTP-ответов переведён на `bytes + String::from_utf8_lossy`, чтобы не падать на невалидном UTF-8
- Добавлена lossy-десериализация вложений: неизвестный или кривой attachment больше не валит весь update или message
- Для действий бота закреплено значение MAX `typing_on`

#### Известное поведение платформы MAX

- `request_contact` может передать contact attachment с пустыми `contact_id` и `vcf_phone`.
- `request_geo_location` может показать карточку геопозиции в мобильном клиенте без доставки соответствующего update боту.
- `typing_on` может вернуть успешный API-ответ без видимого индикатора набора текста в клиенте.
- `set_my_commands` остаётся experimental helper: `POST /me/commands` возвращает `404`, а публичный REST MAX не показывает документированного write-endpoint для меню команд.
