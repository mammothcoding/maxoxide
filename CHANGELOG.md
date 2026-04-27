# Changelog

All notable changes to this project will be documented in this file.

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

#### Verification

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo test`
- `cargo test --features webhook`
- `cargo clippy --all-targets --all-features -- -D warnings`

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

#### Проверка

- `cargo fmt --all`
- `cargo check --all-targets --all-features`
- `cargo test`
- `cargo test --features webhook`
- `cargo clippy --all-targets --all-features -- -D warnings`

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

#### Verification

- `cargo fmt --all`
- `cargo check --example live_api_test`
- `cargo test`
- The live API test was successfully completed against a real MAX bot during this release cycle

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

#### Проверка

- `cargo fmt --all`
- `cargo check --example live_api_test`
- `cargo test`
- Живой тест на реальном MAX-боте был успешно пройден в рамках этого релиза
