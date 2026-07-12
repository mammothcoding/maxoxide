[![Crates.io](https://img.shields.io/crates/v/maxoxide.svg)](https://crates.io/crates/maxoxide)
[![docs.rs](https://docs.rs/maxoxide/badge.svg)](https://docs.rs/maxoxide/)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://choosealicense.com/licenses/mit/)
[![Build Status](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml/badge.svg?event=push)](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)

Readme на разных языках:
[EN](README.md) · [RU](README.ru.md)

# ![alt text](./max_logo.png "max_logo") maxoxide

🦀 Async Rust-библиотека для создания ботов на платформе [Max мессенджер](https://max.ru),
вдохновлённая [teloxide](https://github.com/teloxide/teloxide).

## Возможности

- ✅ Покрытие опубликованного REST API Max на актуальном host `platform-api2.max.ru`
- ✅ Автоматическая поддержка Russian Trusted Root CA для текущей TLS-цепочки MAX API
- ✅ Long polling и опциональный **Webhook**-сервер на [axum](https://github.com/tokio-rs/axum)
- ✅ Типизированные события с fallback для неизвестных обновлений (`Update`, `Message`, `Callback`)
- ✅ `Dispatcher` с регистрацией хендлеров, составными фильтрами, startup hooks и периодическими задачами
- ✅ Inline-клавиатура (`callback`, `link`, `message`, `chat`, `open_app`, `clipboard`, `request_contact`, `request_geo_location`)
- ✅ Разбор markup текста сообщений, включая quote markup и fallback для неизвестной разметки
- ✅ Загрузка файлов — multipart, `photos` payload для image, правильный порядок токенов для видео/аудио, helpers для image/video/audio/file
- ✅ Форматирование Markdown / HTML
- ✅ Верификация Webhook-секрета (`X-Max-Bot-Api-Secret`)
- ✅ Полностью async на Tokio

## Быстрый старт

```toml
[dependencies]
maxoxide = "2.3.0"
tokio    = { version = "1", features = ["full"] }

# Включить встроенный webhook-сервер на axum:
# maxoxide = { version = "2.3.0", features = ["webhook"] }
```

```rust
use maxoxide::{Bot, Context, Dispatcher};
use maxoxide::types::Update;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();     // читает переменную окружения MAX_BOT_TOKEN
    let mut dp = Dispatcher::new(bot);

    dp.on_command("/start", |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_markdown_to_chat(message.chat_id(), "Привет! 👋")
                .await?;
        }
        Ok(())
    });

    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(без текста)").to_string();
            ctx.bot.send_text_to_chat(message.chat_id(), text).await?;
        }
        Ok(())
    });

    dp.start_polling().await;
}
```

```bash
MAX_BOT_TOKEN=ваш_токен cargo run --example echo_bot
```

## TLS trust для `platform-api2.max.ru`

Текущий официальный host MAX API использует цепочку сертификатов до `Russian Trusted Root CA`. `Bot::new()` и `Bot::from_env()` оставляют TLS verification включённой и готовят доверие автоматически:

- сначала пытается скачать свежий PEM с официального URL `gu-st.ru`;
- если скачать не получилось, использует встроенную копию `Russian Trusted Root CA`, поставляемую вместе с crate;
- CA добавляется к обычным trust roots, а не отключает проверку сертификатов.

Если передать собственный `reqwest::Client` через `Bot::with_client(...)`, используется его TLS-конфигурация без неявной подмены. Добавьте встроенный Russian root CA в custom client через `RussianTlsExt::russian_tls()`:

```rust
use maxoxide::{Bot, RussianTlsExt, reqwest::Client};
use std::time::Duration;

# fn example(token: String) -> Result<Bot, reqwest::Error> {
let client = Client::builder()
    .timeout(Duration::from_secs(30))
    .no_proxy()
    .russian_tls()?
    .build()?;

let bot = Bot::with_client(token, client);
# Ok(bot)
# }
```

## Методы API

| Метод | Описание |
|-------|----------|
| `bot.get_me()` | Информация о боте |
| `bot.edit_my_info(body)` | Изменить профиль, команды или аватар бота через `PATCH /me` |
| `bot.send_text_to_chat(chat_id, text)` | Отправить текст в диалог/группу/канал по `chat_id` |
| `bot.send_text_to_user(user_id, text)` | Отправить текст пользователю по глобальному MAX `user_id` |
| `bot.send_markdown_to_chat(chat_id, text)` | Отправить Markdown в диалог/группу/канал по `chat_id` |
| `bot.send_markdown_to_user(user_id, text)` | Отправить Markdown пользователю по глобальному MAX `user_id` |
| `bot.send_message_to_chat(chat_id, body)` | Отправить сообщение с вложениями / кнопками по `chat_id` (`request_contact` / `request_geo_location` live-подтверждены; `chat`-кнопка сейчас ограничена платформой) |
| `bot.send_message_to_user(user_id, body)` | Отправить сообщение с вложениями / кнопками по глобальному MAX `user_id` (`request_contact` / `request_geo_location` live-подтверждены; `chat`-кнопка сейчас ограничена платформой) |
| `bot.send_message_to_chat_with_options(chat_id, body, options)` | Отправить с query-настройками, например `disable_link_preview` |
| `bot.edit_message(mid, body)` | Редактировать сообщение |
| `bot.delete_message(mid)` | Удалить сообщение |
| `bot.get_messages_by_ids(ids, …)` | Получить одно или несколько сообщений по ID |
| `bot.get_video(video_token)` | Получить metadata и URL воспроизведения видео |
| `bot.answer_callback(body)` | Ответ на нажатие кнопки |
| `bot.get_chat(chat_id)` | Информация о чате |
| `bot.get_chat_by_link(chat_link)` | Информация о канале по публичной ссылке / username, например `https://max.ru/channel`, `channel` или `@channel` (может вернуть `404 Chat not found by link`, если канал недоступен боту) |
| `bot.get_chats(…)` | Deprecated: MAX больше не поддерживает `GET /chats`; сохраняйте `chat_id` из updates самостоятельно |
| `bot.edit_chat(chat_id, body)` | Изменить название / описание чата |
| `bot.leave_chat(chat_id)` | Выйти из чата |
| `bot.get_members(…)` | Участники чата |
| `bot.get_members_by_ids(chat_id, user_ids)` | Получить выбранных участников |
| `bot.add_members(…)` | Добавить участников |
| `bot.remove_member(…)` | Удалить участника |
| `bot.remove_member_with_options(…, options)` | Удалить участника с настройками, например `block=true` |
| `bot.get_admins(chat_id)` | Администраторы |
| `bot.add_admins(chat_id, admins)` | Выдать права администратора |
| `bot.remove_admin(chat_id, user_id)` | Снять права администратора |
| `bot.pin_message(…)` | Закрепить сообщение |
| `bot.unpin_message(…)` | Открепить |
| `bot.send_sender_action(chat_id, action)` | Отправить типизированное действие бота (`typing_on` live-подтверждён как видимый в групповых чатах) |
| `bot.get_updates_with_types(…, types)` | Long polling только для выбранных типов update |
| `bot.get_updates_raw_with_types(…, types)` | Raw JSON long polling только для выбранных типов update |
| `bot.subscribe(body)` | Подписаться на Webhook |
| `bot.get_upload_url(type)` | Получить URL загрузки |
| `bot.upload_file(type, path, name, mime)` | Загрузить файл (двухшаговый процесс) |
| `bot.upload_bytes(type, bytes, name, mime)` | То же, из байтов |
| `bot.send_image_to_chat(...)` | Загрузить и отправить изображение |
| `bot.send_video_to_chat(...)` | Загрузить и отправить видео |
| `bot.send_audio_to_chat(...)` | Загрузить и отправить аудио |
| `bot.send_file_to_chat(...)` | Загрузить и отправить обычный файл |
| `bot.set_my_commands(commands)` | Экспериментально: публичный write endpoint не документирован; live API сейчас отвечает `404` на `/me/commands` |

## user_id и chat_id

Эти два идентификатора разные, их нельзя использовать как взаимозаменяемые:

- `user_id` — это глобальный ID пользователя Max.
- `chat_id` — это ID конкретного диалога, группы или канала.
- В личном чате `message.sender.user_id` идентифицирует пользователя, а `message.chat_id()` идентифицирует конкретный диалог этого бота с пользователем.
- Используйте `send_text_to_chat(chat_id, ...)` / `send_message_to_chat(chat_id, ...)`, когда у вас уже есть ID диалога или группы.
- Используйте `send_text_to_user(user_id, ...)` / `send_message_to_user(user_id, ...)`, когда у вас есть только глобальный MAX `user_id` пользователя.

## Замена deprecated `get_chats`

MAX перестал поддерживать `GET /chats` с июня 2026 года и объявил отключение в августе 2026. Endpoint-замены, который возвращает полный список чатов/каналов бота, нет. Сохраняйте `chat_id` из updates в собственной БД, удаляйте их на `bot_removed`, затем используйте `get_chat(chat_id)` и остальные методы по `chat_id`.

```rust
use maxoxide::{Context, Dispatcher};
use maxoxide::types::Update;

# fn configure(dp: &mut Dispatcher) {
dp.on_bot_added(|ctx: Context| async move {
    if let Update::BotAdded { chat_id, .. } = &ctx.update {
        // Сохранить chat_id в своей БД.
    }
    Ok(())
});

dp.on_bot_removed(|ctx: Context| async move {
    if let Update::BotRemoved { chat_id, .. } = &ctx.update {
        // Удалить chat_id из своей БД.
    }
    Ok(())
});
# }
```

Для общих handlers можно использовать `Update::chat_id()`: он возвращает ID чата, если update его содержит.

## Фильтры диспетчера

```rust
dp.on_command("/start", handler);              // конкретная команда
dp.on_message(handler);                        // любое новое сообщение
dp.on_edited_message(handler);                // редактирование
dp.on_callback(handler);                      // любой callback
dp.on_callback_payload("btn:ok", handler);    // конкретный payload
dp.on_bot_started(handler);                   // первый запуск бота
dp.on_bot_stopped(handler);                   // пользователь остановил бота
dp.on_dialog_muted(handler);                  // диалог заглушён
dp.on_message_chat_created(handler);          // chat-кнопка создала чат
dp.on_update(
    Filter::message() & Filter::chat(chat_id) & Filter::text_contains("ping"),
    handler,
);                                            // составные фильтры
dp.on_start(handler);                         // перед стартом polling
dp.task(Duration::from_secs(60), handler);    // периодическая задача
dp.on_raw_update(handler);                    // raw JSON каждого update
dp.on_filter(|u| { … }, handler);             // кастомный предикат
dp.on(handler);                               // все обновления
```

Срабатывает первый подходящий хендлер. Более специфичные фильтры регистрируйте раньше.

## Inline-клавиатура

```rust
use maxoxide::types::{Button, KeyboardPayload, NewMessageBody};

let keyboard = KeyboardPayload {
    buttons: vec![
        vec![
            Button::callback("Да ✅", "answer:yes"),
            Button::callback("Нет ❌", "answer:no"),
        ],
        vec![Button::link("🌐 Сайт", "https://max.ru")],
    ],
};

let body = NewMessageBody::text("Вы уверены?").with_keyboard(keyboard);
bot.send_message_to_chat(chat_id, body).await?;
```

Clipboard-кнопка копирует свой `payload` в клиенте MAX. Она не отправляет
callback update боту:

```rust
let keyboard = KeyboardPayload {
    buttons: vec![vec![Button::clipboard(
        "Скопировать код",
        "MAXOXIDE-2026",
    )]],
};

let body = NewMessageBody::empty().with_keyboard(keyboard);
bot.send_message_to_chat(chat_id, body).await?;
```

Кнопки запроса контакта live-подтверждены: приходят `vcf_info`, `hash` и `max_info` (`ContactPayload::validate_hash(token)` проверяет VCF hash; если `vcf_phone` пустой, используйте `phones_from_vcf()`). Кнопки запроса геопозиции live-подтверждены: приходит структурированный `Attachment::Location`. `Button::chat(...)` оставлен для документированной схемы MAX, но текущий live `POST /messages` отклоняет документированный JSON `chat`-кнопки с `400 Can't deserialize body`.

## Загрузка файлов

Max использует двухшаговый процесс загрузки. `upload_file` / `upload_bytes` делают его автоматически:

```rust
use maxoxide::types::{NewAttachment, NewMessageBody, UploadType};

let token = bot
    .upload_file(UploadType::Image, "./photo.jpg", "photo.jpg", "image/jpeg")
    .await?;

let body = NewMessageBody {
    text: Some("Вот фото!".into()),
    attachments: Some(vec![NewAttachment::image(token)]),
    ..Default::default()
};
bot.send_message_to_chat(chat_id, body).await?;
// или:
// bot.send_message_to_user(user_id, body).await?;
```

> **Важно:** тип `photo` удалён из API Max. Всегда используйте `UploadType::Image`.

Для частых случаев есть helpers, которые сразу загружают и отправляют файл:

```rust
bot.send_image_to_chat(chat_id, "./photo.jpg", "photo.jpg", "image/jpeg", None).await?;
bot.send_video_to_chat(chat_id, "./clip.mp4", "clip.mp4", "video/mp4", None).await?;
bot.send_audio_to_chat(chat_id, "./track.mp3", "track.mp3", "audio/mpeg", None).await?;
bot.send_file_to_chat(chat_id, "./report.pdf", "report.pdf", "application/pdf", None).await?;
```

Есть такие же `*_to_user` и `*_bytes_*` helpers.

После успешного upload MAX может ещё несколько секунд обрабатывать вложение. Helpers загрузки и отправки коротко ретраят отправку, если API отвечает, что вложение ещё не обработано.

Image upload может вернуть MAX `photos` token map вместо одного `token`. Helpers `send_image_*` автоматически сохраняют и отправляют этот payload.

## Webhook-сервер (`features = ["webhook"]`)

```rust
use maxoxide::webhook::WebhookServer;
use maxoxide::types::SubscribeBody;

// Зарегистрировать webhook в Max
bot.subscribe(SubscribeBody {
    url: "https://your-domain.com/webhook".into(),
    update_types: None,
    version: None,
    secret: Some("my_secret_123".into()),
}).await?;

// Запустить сервер (поставьте nginx/Caddy перед ним на порту 443)
WebhookServer::new(dp)
    .secret("my_secret_123")
    .path("/webhook")
    .serve("0.0.0.0:8443")
    .await;
```

Используйте webhook, когда у бота есть публичный HTTPS endpoint и вы хотите, чтобы MAX доставлял обновления входящими запросами. Для локальной разработки и простых запусков обычно достаточно long polling.

> Max требует HTTPS на порту 443. Самоподписанные сертификаты **не поддерживаются**.

## Структура проекта

```
maxoxide/
├── Cargo.toml
├── src/
│   ├── lib.rs          — публичный API и ре-экспорты
│   ├── bot.rs          — Bot, HTTP-методы и TLS helpers
│   ├── uploader.rs     — загрузка файлов
│   ├── dispatcher.rs   — Dispatcher, Filter, Context
│   ├── errors.rs       — MaxError
│   ├── webhook.rs      — axum webhook-сервер (feature = "webhook")
│   ├── tests.rs        — юнит-тесты
│   ├── types.rs        — все типы (User, Chat, Message, Update, …)
│   └── certs/
│       └── russian_trusted_root_ca.pem
└── examples/
    ├── echo_bot.rs
    ├── dispatcher_filters_bot.rs
    ├── keyboard_bot.rs
    ├── live_api_test.rs
    ├── media_bot.rs
    └── webhook_bot.rs  (feature = "webhook")
```

## Запуск тестов

```bash
cargo test
```

## Live API тест

Для проверки на реальных данных добавлен отдельный интерактивный harness:

```bash
cargo run --example live_api_test
```

В начале он спрашивает прямо в терминале:

- транспорт updates: `long_polling` или `webhook`
- токен бота
- URL бота для тестера
- необязательная публичная ссылка канала для `bot.get_chat_by_link` (`https://max.ru/channel`, `channel` или `@channel`)
- необязательные webhook URL и secret
- локальный адрес для приёма webhook, если выбран транспорт `webhook`
- необязательный путь к локальному файлу для `upload_file`
- необязательные пути к изображению, видео и аудио для проверки media helpers
- polling timeout и задержку между запросами

Дальше harness пошагово ведёт тестера по действиям в клиенте Max и фиксирует `PASS` / `FAIL` / `SKIP` по реальным API-вызовам. Он заранее очищает backlog long polling, ставит небольшие паузы между запросами и требует явного подтверждения перед разрушительными или неоткатываемыми шагами, например:

- временное отключение и восстановление webhook перед long-polling ожиданиями
- `set_my_commands`
- `delete_chat`
- `leave_chat`
- видимое изменение title у группы
- `remove_member_with_options(..., block=true)`

Long polling и webhooks в MAX нельзя использовать одновременно. В режиме `long_polling` harness проверяет активные webhook subscriptions, может временно отписать их и восстанавливает в конце с webhook secret, введённым при старте. В режиме `webhook` он запускает минимальный локальный receiver, чтобы ручные ожидания читали входящие webhook POST вместо `GET /updates`.

Текущий прогон проверяет кнопки запроса контакта/геопозиции, contact hash, text markup, message-кнопки, chat-кнопки, `open_app`, `clipboard`, sender actions, filtered polling, metadata загруженного видео, выборочное получение участников, временное изменение admin-прав, настройки удаления участников и subscribe/unsubscribe/restore для webhook.

## Лицензия

[MIT](https://choosealicense.com/licenses/mit/)
