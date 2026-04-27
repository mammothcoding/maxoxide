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

- ✅ Покрытие опубликованного REST API Max
- ✅ Long polling и опциональный **Webhook**-сервер на [axum](https://github.com/tokio-rs/axum)
- ✅ Типизированные события с fallback для неизвестных обновлений (`Update`, `Message`, `Callback`)
- ✅ `Dispatcher` с регистрацией хендлеров, составными фильтрами, startup hooks и периодическими задачами
- ✅ Inline-клавиатура (`callback`, `link`, `message`, `open_app`, `clipboard`, `request_contact`, `request_geo_location`)
- ✅ Загрузка файлов — multipart, `photos` payload для image, правильный порядок токенов для видео/аудио, helpers для image/video/audio/file
- ✅ Форматирование Markdown / HTML
- ✅ Верификация Webhook-секрета (`X-Max-Bot-Api-Secret`)
- ✅ Полностью async на Tokio

## Быстрый старт

```toml
[dependencies]
maxoxide = "2.0.0"
tokio    = { version = "1", features = ["full"] }

# Включить встроенный webhook-сервер на axum:
# maxoxide = { version = "2.0.0", features = ["webhook"] }
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

## Методы API

| Метод | Описание |
|-------|----------|
| `bot.get_me()` | Информация о боте |
| `bot.send_text_to_chat(chat_id, text)` | Отправить текст в диалог/группу/канал по `chat_id` |
| `bot.send_text_to_user(user_id, text)` | Отправить текст пользователю по глобальному MAX `user_id` |
| `bot.send_markdown_to_chat(chat_id, text)` | Отправить Markdown в диалог/группу/канал по `chat_id` |
| `bot.send_markdown_to_user(user_id, text)` | Отправить Markdown пользователю по глобальному MAX `user_id` |
| `bot.send_message_to_chat(chat_id, body)` | Отправить сообщение с вложениями / кнопками по `chat_id` |
| `bot.send_message_to_user(user_id, body)` | Отправить сообщение с вложениями / кнопками по глобальному MAX `user_id` |
| `bot.send_message_to_chat_with_options(chat_id, body, options)` | Отправить с query-настройками, например `disable_link_preview` |
| `bot.edit_message(mid, body)` | Редактировать сообщение |
| `bot.delete_message(mid)` | Удалить сообщение |
| `bot.get_messages_by_ids(ids, …)` | Получить одно или несколько сообщений по ID |
| `bot.get_video(video_token)` | Получить metadata и URL воспроизведения видео |
| `bot.answer_callback(body)` | Ответ на нажатие кнопки |
| `bot.get_chat(chat_id)` | Информация о чате |
| `bot.get_chats(…)` | Список групповых чатов |
| `bot.edit_chat(chat_id, body)` | Изменить название / описание чата |
| `bot.leave_chat(chat_id)` | Выйти из чата |
| `bot.get_members(…)` | Участники чата |
| `bot.get_members_by_ids(chat_id, user_ids)` | Получить выбранных участников |
| `bot.add_members(…)` | Добавить участников |
| `bot.remove_member(…)` | Удалить участника |
| `bot.get_admins(chat_id)` | Администраторы |
| `bot.add_admins(chat_id, admins)` | Выдать права администратора |
| `bot.remove_admin(chat_id, user_id)` | Снять права администратора |
| `bot.pin_message(…)` | Закрепить сообщение |
| `bot.unpin_message(…)` | Открепить |
| `bot.send_sender_action(chat_id, action)` | Отправить типизированное действие бота |
| `bot.subscribe(body)` | Подписаться на Webhook |
| `bot.get_upload_url(type)` | Получить URL загрузки |
| `bot.upload_file(type, path, name, mime)` | Загрузить файл (двухшаговый процесс) |
| `bot.upload_bytes(type, bytes, name, mime)` | То же, из байтов |
| `bot.send_image_to_chat(...)` | Загрузить и отправить изображение |
| `bot.send_video_to_chat(...)` | Загрузить и отправить видео |
| `bot.send_audio_to_chat(...)` | Загрузить и отправить аудио |
| `bot.send_file_to_chat(...)` | Загрузить и отправить обычный файл |
| `bot.set_my_commands(commands)` | Экспериментально: публичный MAX API сейчас отвечает `404` на `/me/commands` |

## user_id и chat_id

Эти два идентификатора разные, их нельзя использовать как взаимозаменяемые:

- `user_id` — это глобальный ID пользователя Max.
- `chat_id` — это ID конкретного диалога, группы или канала.
- В личном чате `message.sender.user_id` идентифицирует пользователя, а `message.chat_id()` идентифицирует конкретный диалог этого бота с пользователем.
- Используйте `send_text_to_chat(chat_id, ...)` / `send_message_to_chat(chat_id, ...)`, когда у вас уже есть ID диалога или группы.
- Используйте `send_text_to_user(user_id, ...)` / `send_message_to_user(user_id, ...)`, когда у вас есть только глобальный MAX `user_id` пользователя.

## Известные ограничения MAX

На 27 апреля 2026 года библиотека умеет отправлять эти запросы, но live-поведение на стороне MAX пока расходится с ожиданиями:

- `Button::RequestContact` задокументирована в MAX, но в live-тестах приходило contact-вложение с пустыми `contact_id` и `vcf_phone`. Кнопка отправляется корректно, но возврат номера телефона пользователя на стороне MAX пока не подтверждён.
- `Button::RequestGeoLocation` доставляет структурированное `Attachment::Location` с `latitude` и `longitude`; в клиенте та же отправленная позиция может отображаться как карточка Яндекс Карт.
- `bot.send_sender_action(chat_id, SenderAction::TypingOn)` получает успешный ответ API, но live-тесты MAX не подтвердили видимый индикатор набора текста в клиенте.
- `bot.set_my_commands` оставлен как экспериментальный helper, но в публичной REST-документации MAX нет write-эндпоинта для команд бота, а live-запросы `POST /me/commands` возвращают `404 Path /me/commands is not recognized`.

## Фильтры диспетчера

```rust
dp.on_command("/start", handler);              // конкретная команда
dp.on_message(handler);                        // любое новое сообщение
dp.on_edited_message(handler);                // редактирование
dp.on_callback(handler);                      // любой callback
dp.on_callback_payload("btn:ok", handler);    // конкретный payload
dp.on_bot_started(handler);                   // первый запуск бота
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
│   ├── bot.rs          — Bot + все HTTP-методы
│   ├── uploader.rs     — загрузка файлов
│   ├── dispatcher.rs   — Dispatcher, Filter, Context
│   ├── errors.rs       — MaxError
│   ├── webhook.rs      — axum webhook-сервер (feature = "webhook")
│   ├── tests.rs        — юнит-тесты
│   └── types.rs        — все типы (User, Chat, Message, Update, …)
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

- токен бота
- URL бота для тестера
- необязательные webhook URL и secret
- необязательный путь к локальному файлу для `upload_file`
- необязательные пути к изображению, видео и аудио для проверки media helpers
- HTTP timeout, polling timeout и задержку между запросами

Дальше harness пошагово ведёт тестера по действиям в клиенте Max и фиксирует `PASS` / `FAIL` / `SKIP` по реальным API-вызовам. Он заранее очищает backlog long polling, ставит небольшие паузы между запросами и требует явного подтверждения перед разрушительными или неоткатываемыми шагами, например:

- `set_my_commands`
- `delete_chat`
- `leave_chat`
- видимое изменение title у группы

Текущий прогон также проверяет неясное поведение MAX вокруг кнопок запроса контакта/геопозиции, message-кнопок, `open_app`, `clipboard`, sender actions, metadata загруженного видео, выборочного получения участников и временного изменения admin-прав.

## Лицензия

[MIT](https://choosealicense.com/licenses/mit/)
