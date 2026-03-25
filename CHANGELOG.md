# Changelog

All notable changes to this project will be documented in this file.

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
