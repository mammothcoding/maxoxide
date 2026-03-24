[![Crates.io](https://img.shields.io/crates/v/maxoxide.svg)](https://crates.io/crates/maxoxide)
[![docs.rs](https://docs.rs/maxoxide/badge.svg)](https://docs.rs/maxoxide/)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://choosealicense.com/licenses/mit/)
[![Build Status](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml/badge.svg?event=push)](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml)
[![made-with-rust](https://img.shields.io/badge/Made%20with-Rust-1f425f.svg)](https://www.rust-lang.org/)

Readme на разных языках:
[EN](README.md) · [RU](README.ru.md)

# 🦀 maxoxide

Async Rust-библиотека для создания ботов на платформе [Max мессенджер](https://max.ru),
вдохновлённая [teloxide](https://github.com/teloxide/teloxide).

## Возможности

- ✅ Полное покрытие REST API Max
- ✅ Long polling (для разработки) и **Webhook** через [axum](https://github.com/tokio-rs/axum) (для продакшена)
- ✅ Типизированные события (`Update`, `Message`, `Callback`, …)
- ✅ `Dispatcher` с регистрацией хендлеров и фильтрами
- ✅ Inline-клавиатура (все типы кнопок: `callback`, `link`, `message`, `request_contact`, `request_geo_location`)
- ✅ Загрузка файлов — multipart, правильный порядок токенов для видео/аудио
- ✅ Форматирование Markdown / HTML
- ✅ Верификация Webhook-секрета (`X-Max-Bot-Api-Secret`)
- ✅ Полностью async на Tokio

## Быстрый старт

```toml
[dependencies]
maxoxide = { git = "https://github.com/yourname/maxoxide" }
tokio    = { version = "1", features = ["full"] }

# Для поддержки Webhook (продакшен):
# maxoxide = { git = "...", features = ["webhook"] }
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
            ctx.bot.send_markdown(message.chat_id(), "Привет! 👋").await?;
        }
        Ok(())
    });

    dp.on_message(|ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            let text = message.text().unwrap_or("(без текста)").to_string();
            ctx.bot.send_text(message.chat_id(), text).await?;
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
| `bot.send_text(chat_id, text)` | Отправить текст |
| `bot.send_markdown(chat_id, text)` | Отправить Markdown |
| `bot.send_message(chat_id, body)` | Отправить сообщение с вложениями / кнопками |
| `bot.edit_message(mid, body)` | Редактировать сообщение |
| `bot.delete_message(mid)` | Удалить сообщение |
| `bot.answer_callback(body)` | Ответ на нажатие кнопки |
| `bot.get_chat(chat_id)` | Информация о чате |
| `bot.get_chats(…)` | Список групповых чатов |
| `bot.edit_chat(chat_id, body)` | Изменить название / описание чата |
| `bot.leave_chat(chat_id)` | Выйти из чата |
| `bot.get_members(…)` | Участники чата |
| `bot.add_members(…)` | Добавить участников |
| `bot.remove_member(…)` | Удалить участника |
| `bot.get_admins(chat_id)` | Администраторы |
| `bot.pin_message(…)` | Закрепить сообщение |
| `bot.unpin_message(…)` | Открепить |
| `bot.send_action(chat_id, "typing")` | Индикатор набора текста |
| `bot.subscribe(body)` | Подписаться на Webhook |
| `bot.get_upload_url(type)` | Получить URL загрузки |
| `bot.upload_file(type, path, name, mime)` | Загрузить файл (двухшаговый процесс) |
| `bot.upload_bytes(type, bytes, name, mime)` | То же, из байтов |
| `bot.set_my_commands(commands)` | Установить команды бота |

## Фильтры диспетчера

```rust
dp.on_command("/start", handler);              // конкретная команда
dp.on_message(handler);                        // любое новое сообщение
dp.on_edited_message(handler);                // редактирование
dp.on_callback(handler);                      // любой callback
dp.on_callback_payload("btn:ok", handler);    // конкретный payload
dp.on_bot_started(handler);                   // первый запуск бота
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
bot.send_message(chat_id, body).await?;
```

## Загрузка файлов

Max использует двухшаговый процесс загрузки. `upload_file` / `upload_bytes` делают его автоматически:

```rust
use maxoxide::types::{NewAttachment, NewMessageBody, UploadType, UploadedToken};

let token = bot
    .upload_file(UploadType::Image, "./photo.jpg", "photo.jpg", "image/jpeg")
    .await?;

let body = NewMessageBody {
    text: Some("Вот фото!".into()),
    attachments: Some(vec![NewAttachment::Image {
        payload: UploadedToken { token },
    }]),
    ..Default::default()
};
bot.send_message(chat_id, body).await?;
```

> **Важно:** тип `photo` удалён из API Max. Всегда используйте `UploadType::Image`.

> **Для видео и аудио:** токен выдаётся на первом шаге (`POST /uploads`), а не из ответа загрузки — библиотека учитывает это автоматически.

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
│   └── types/
│       └── mod.rs      — все типы (User, Chat, Message, Update, …)
└── examples/
    ├── echo_bot.rs
    ├── keyboard_bot.rs
    └── webhook_bot.rs  (feature = "webhook")
```

## Запуск тестов

```bash
cargo test
```

## Лицензия

[MIT](https://choosealicense.com/licenses/mit/)
