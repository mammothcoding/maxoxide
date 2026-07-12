# Changelog

All notable changes to this project will be documented in this file.

## [2.3.0] - 2026-07-13

### EN

#### Release summary

This compatible release adds a custom-client TLS helper for the current MAX API certificate chain and removes maxoxide's own live-test dependency on the deprecated `GET /chats` endpoint.

#### Added

- Added `RussianTlsExt::russian_tls()` for `reqwest::ClientBuilder`, so custom clients can keep settings such as `timeout(...)` and `no_proxy()` while adding the embedded `Russian Trusted Root CA`.
- Added `Update::chat_id()` to extract a chat ID from typed updates when one is present. This helps applications maintain their own chat registry after MAX deprecated `GET /chats`.

#### Changed

- Bumped the crate version to `2.3.0`.
- `examples/live_api_test.rs` no longer calls `bot.get_chats` at startup. The optional group phase now uses the `/group_live` update or manual `chat_id` entry.
- README and README.ru now document that `Bot::new()` and `Bot::from_env()` configure Russian TLS automatically, while `Bot::with_client(...)` custom clients should call `.russian_tls()` during `reqwest::ClientBuilder` setup.

#### Deprecated

- Deprecated `Bot::get_chats(...)`. MAX stopped supporting `GET /chats` in June 2026 and announced shutdown for August 2026. Store `chat_id` values from updates such as `bot_added`, `bot_started`, and message events in your own storage, remove them on `bot_removed`, and use chat-id-based methods.

### RU

#### Кратко о релизе

Совместимый релиз добавляет TLS-helper для custom clients под текущую цепочку сертификатов MAX API и убирает собственную зависимость live-теста maxoxide от deprecated endpoint `GET /chats`.

#### Добавлено

- Добавлен `RussianTlsExt::russian_tls()` для `reqwest::ClientBuilder`, чтобы custom clients сохраняли настройки вроде `timeout(...)` и `no_proxy()` и при этом добавляли встроенный `Russian Trusted Root CA`.
- Добавлен `Update::chat_id()` для извлечения chat ID из typed updates, если update его содержит. Это помогает приложениям вести собственный реестр чатов после deprecation `GET /chats`.

#### Изменено

- Версия крейта повышена до `2.3.0`.
- `examples/live_api_test.rs` больше не вызывает `bot.get_chats` на старте. Опциональный групповой этап использует update от `/group_live` или ручной ввод `chat_id`.
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
- Added optional `bot.get_chat_by_link` coverage to `examples/live_api_test.rs`.
- Added automatic `Russian Trusted Root CA` handling for the default clients created by `Bot::new()` and `Bot::from_env()`: maxoxide tries to download the fresh PEM from the official `gu-st.ru` URL and falls back to an embedded copy while keeping TLS verification enabled.

#### Changed

- Switched the hardcoded API host from deprecated `https://platform-api.max.ru` to current `https://platform-api2.max.ru`.
- Updated `examples/live_api_test.rs` to use the default bot client so it exercises automatic TLS trust setup and no longer asks for a custom HTTP timeout.
- `Bot::get_chat_by_link` now accepts full `max.ru` URLs, plain channel names, and `@channel` names; full URLs are safely encoded as a single path segment and channel-name fallbacks are tried on `404`.
- `examples/live_api_test.rs` now treats `bot.get_chat_by_link` `404 Chat not found by link` as an optional precondition skip because public links can be unavailable to the Bot API for a given bot/channel.

### RU

#### Кратко о релизе

Совместимый релиз, который следует актуальным официальным SDK и схеме MAX: переключает default API host на `platform-api2.max.ru` и добавляет новый endpoint получения канала по публичной ссылке.

#### Добавлено

- Добавлен `Bot::get_chat_by_link(chat_link)` для `GET /chats/{chatLink}`. Официальный API документирует этот endpoint для каналов по публичной ссылке / username, например `@channel`; live-доступность зависит от доступа MAX Bot API к этому каналу.
- Добавлены поля `Chat.participants` и `Chat.messages_count` из актуальной схемы `Chat`.
- Добавлены typed variants `ChatAdminPermission::Edit` и `ChatAdminPermission::Delete` для актуального enum прав администратора.
- Добавлена опциональная проверка `bot.get_chat_by_link` в `examples/live_api_test.rs`.
- Добавлена автоматическая поддержка `Russian Trusted Root CA` для default clients, созданных через `Bot::new()` и `Bot::from_env()`: maxoxide пытается скачать свежий PEM с официального URL `gu-st.ru` и fallback-ом использует встроенную копию, не отключая TLS verification.

#### Изменено

- Hardcoded API host переключён с deprecated `https://platform-api.max.ru` на актуальный `https://platform-api2.max.ru`.
- `examples/live_api_test.rs` переведён на default bot client, чтобы проверять автоматическую настройку TLS trust, и больше не спрашивает custom HTTP timeout.
- `Bot::get_chat_by_link` теперь принимает full `max.ru` URL, имя канала без префикса и `@channel`; full URL безопасно кодируется как один path segment, а варианты имени канала пробуются при `404`.
- `examples/live_api_test.rs` теперь помечает `bot.get_chat_by_link` `404 Chat not found by link` как пропущенное optional-предусловие, потому что публичная ссылка может быть недоступна Bot API для конкретного бота/канала.

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

- The live API harness now probes filtered polling, message markup, contact hash/max_info, optional dialog events, and opt-in chat-button chat creation with explicit cleanup choice.
- The live API harness now supports both update transports at startup: `long_polling` and `webhook`.
- In `long_polling` mode, the live API harness checks active webhook subscriptions, warns that they disable long polling, can temporarily unsubscribe them, and restores them at the end using the webhook secret entered during startup.
- In `webhook` mode, the live API harness starts a minimal local webhook receiver so manual waits can consume incoming webhook POSTs without enabling the optional crate `webhook` feature.
- The live API harness now treats MAX `ChatButton` send-time deserialization failures as an opt-in platform limitation, prints the outgoing JSON, and can capture raw `message_chat_created` updates for investigation.
- Live testing confirmed that the group-chat typing indicator is now visible for `typing_on`.
- The live group phase now exercises `remove_member_with_options` and asks before passing `block=true`.
- The live group phase now treats `add_admins` attempts for non-participant user IDs as skipped precondition failures instead of SDK/API failures.
- The crate version was bumped to `2.1.0`.

#### Live API observations

- Full long-polling live run completed with `89 PASS / 0 FAIL / 8 SKIP`.
- `request_contact` is live-confirmed to deliver `vcf_info`, a valid `hash`, and `max_info`; `vcf_phone` may still be empty, so `phones_from_vcf()` is the reliable fallback.
- `request_geo_location` is live-confirmed to deliver structured `Attachment::Location` coordinates.
- Webhook subscribe/unsubscribe and pre-polling restore were live-confirmed.
- `ChatButton` remains a MAX platform limitation in current live testing: documented `chat` button JSON is rejected by `POST /messages` with `400 Can't deserialize body`.
- `set_my_commands` remains a MAX platform limitation: public live `POST /me/commands` requests return `404`.

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

- Live API harness теперь проверяет filtered polling, message markup, contact hash/max_info, optional dialog events и opt-in создание чата через chat-кнопку с явным выбором cleanup.
- Live API harness теперь поддерживает оба транспорта updates на старте: `long_polling` и `webhook`.
- В режиме `long_polling` live API harness проверяет активные webhook subscriptions, предупреждает, что они отключают long polling, может временно отписать их и восстанавливает их в конце с webhook secret, введённым при старте.
- В режиме `webhook` live API harness запускает минимальный локальный webhook receiver, чтобы ручные ожидания читали входящие webhook POST без включения optional crate feature `webhook`.
- Live API harness теперь помечает send-time ошибку десериализации MAX `ChatButton` как opt-in ограничение платформы, печатает исходящий JSON и умеет ловить raw `message_chat_created` для расследования.
- Live-тест подтвердил, что индикатор набора текста в групповом чате теперь виден для `typing_on`.
- Групповой этап live harness проверяет `remove_member_with_options` и спрашивает перед `block=true`.
- Групповой этап live harness теперь помечает `add_admins` для user_id, который не является участником чата, как пропущенное предусловие, а не как ошибку SDK/API.
- Версия крейта повышена до `2.1.0`.

#### Наблюдения live API

- Полный live-прогон через long polling завершился с `89 PASS / 0 FAIL / 8 SKIP`.
- `request_contact` live-подтверждён: приходит `vcf_info`, валидный `hash` и `max_info`; `vcf_phone` всё ещё может быть пустым, поэтому `phones_from_vcf()` — надёжный fallback.
- `request_geo_location` live-подтверждён: приходят структурированные координаты `Attachment::Location`.
- Webhook subscribe/unsubscribe и восстановление перед long polling live-подтверждены.
- `ChatButton` остаётся ограничением платформы MAX в текущем live-тестировании: документированный JSON `chat`-кнопки отклоняется `POST /messages` с `400 Can't deserialize body`.
- `set_my_commands` остаётся ограничением платформы MAX: публичные live-запросы `POST /me/commands` возвращают `404`.

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
- Added `Button::Clipboard`, which is present in the official Go SDK and can be validated through the live harness.
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
- The live harness now treats empty contact phone payloads as a MAX platform gap, recognizes structured request-location attachments, logs non-matching updates during manual waits, and checks the bot's granular `add_admins` permission before probing admin-right changes.
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
- Добавлен `Button::Clipboard`, который есть в официальном Go SDK и проверяется через live harness.
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
- Live harness теперь помечает пустой телефон в contact payload как platform gap MAX, распознаёт структурированные request-location attachments, логирует неподходящие updates во время ручного ожидания и проверяет granular-право бота `add_admins` перед проверкой изменения admin-прав.
- Примеры README переведены на builders и новые media helpers.
- Версия крейта повышена до `2.0.0`.

## [1.0.0] - 2026-03-25

### EN

#### Release summary

This release promotes `maxoxide` from `0.1.0` to `1.0.0`, adds a real interactive live API test harness for MAX, fixes several real-API mismatches, and makes message delivery APIs explicit about whether they target a `chat_id` or a `user_id`.

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

- Added `examples/live_api_test.rs`, an interactive real-API harness with:
  - English and Russian language selection
  - runtime input for token, bot URL, webhook settings, file path, delays, and timeouts
  - manual tester-driven steps in the MAX client
  - optional group-chat phase
  - `PASS / FAIL / SKIP` summary
  - non-blocking manual waits with `continue / skip / fail`
- Added a `/get_my_id` live-test flow and sender `user_id` logging
- Added live coverage for both `*_to_chat` and `*_to_user` methods, including attachment sending via `user_id`
- Added tests that explicitly verify the difference between `chat_id` and `user_id`

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
- Updated action handling and live testing to use the real MAX action value `typing_on`

#### MAX platform gaps documented by live testing

- `request_contact` is documented by MAX, but live tests received a contact attachment with empty `contact_id` and empty `vcf_phone`
- `request_geo_location` is documented by MAX, and the mobile client shows a sent location card, but the bot did not receive a matching update in live polling tests
- `typing_on` returns a successful API response, but the client-side typing indicator was not reliably visible in live testing
- `set_my_commands` remains experimental: live `POST /me/commands` requests returned `404`, and the public MAX REST docs do not currently expose a documented write endpoint for command menu updates

### RU

#### Кратко о релизе

Этот релиз переводит `maxoxide` с ветки `0.1.0` на `1.0.0`, добавляет полноценный интерактивный live-тест на реальном API MAX, исправляет несколько несовпадений с реальным поведением платформы и делает API отправки сообщений явным по типу получателя: `chat_id` или `user_id`.

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

- Добавлен `examples/live_api_test.rs` — интерактивный harness для проверки реального API, который включает:
  - выбор языка English / Russian
  - ввод токена, URL бота, webhook-настроек, пути к файлу, задержек и таймаутов во время старта
  - ручные шаги тестера в клиенте MAX
  - необязательный этап группового чата
  - итоговую сводку `PASS / FAIL / SKIP`
  - ручное ожидание без `Ctrl+C` через `continue / skip / fail`
- Добавлен live-сценарий `/get_my_id` и вывод `sender.user_id`
- Добавлено live-покрытие новых методов `*_to_chat` и `*_to_user`, включая отправку вложения по `user_id`
- Добавлены тесты, которые явно проверяют различие между `chat_id` и `user_id`

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
- Для действий бота и live-теста закреплено реальное значение MAX `typing_on`

#### Ограничения платформы MAX, выявленные live-тестами

- `request_contact` задокументирован в MAX, но в live-тестах contact приходил с пустыми `contact_id` и `vcf_phone`
- `request_geo_location` задокументирован в MAX, мобильный клиент показывает отправленную карточку геопозиции, но бот не получил соответствующий update в live polling
- `typing_on` возвращает успешный API-ответ, но видимый индикатор набора текста в клиенте live-тестами не подтверждён
- `set_my_commands` остаётся experimental helper: live-запросы `POST /me/commands` возвращают `404`, а публичный REST MAX сейчас не показывает документированного write-эндпоинта для меню команд
