# ![MAX logo](./max_logo.png "MAX logo") maxoxide

[![CI](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml/badge.svg)](https://github.com/mammothcoding/maxoxide/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/maxoxide.svg)](https://crates.io/crates/maxoxide)
[![docs.rs](https://docs.rs/maxoxide/badge.svg)](https://docs.rs/maxoxide)

Ориентированный на production асинхронный Rust SDK для Bot API мессенджера [MAX](https://max.ru).

[English README](README.md) | [Русские руководства](docs/ru/README.md) | [English guides](docs/en/README.md) | [Поддержка API](API_SUPPORT.md) | [Миграция на v3](MIGRATION.md)

## Установка

```toml
[dependencies]
maxoxide = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Опциональные возможности:

| Feature | Возможности |
|---|---|
| `webhook-axum` | Независимое webhook-ядро и адаптер Axum |
| `webhook-actix` | Независимое webhook-ядро и адаптер Actix Web |
| `digital-id` | Experimental partner client проверки возраста MAX Цифрового ID |
| `socks-proxy` | Поддержка SOCKS во встроенном клиенте `reqwest` |

Минимальная версия Rust — 1.85.

## Быстрый старт

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

Для production MAX рекомендует HTTPS Webhook. Long Polling предназначен для разработки и тестирования.

## Возможности

- Типизированные сообщения, чаты, участники, callback, constructed-события, комментарии, подписки и загрузки.
- Структурированный `ApiError`: код MAX, ограниченное raw-тело для диагностики и `Retry-After`.
- Безопасный builder: встроенный Russian Trusted Root CA, таймауты, HTTP/SOCKS proxy, retry и смена base URL.
- Локальные лимиты 30 RPS и ограничения операций с сообщениями по получателю.
- Multipart streaming без загрузки файла целиком и resumable `Content-Range`: retry чанка, progress и cancellation.
- Точный разбор команд, составные фильтры, middleware, типизированный state, ограничение concurrency, задачи и graceful shutdown.
- Единый webhook-сервис с полноценными адаптерами Axum и Actix.
- Проверка HMAC/срока Mini App init data и подписи `requestContact()`.
- Experimental feature-gated partner client Цифрового ID, изолированный от токена бота и Bot API.
- Сохранение неизвестных enum и update там, где MAX может расширять wire-формат.

## Руководства

| Тема | Русский | English |
|---|---|---|
| Начало работы и идентификаторы | [Руководство](docs/ru/getting-started.md) | [Guide](docs/en/getting-started.md) |
| Клиент, TLS, proxy, retry, лимиты | [Руководство](docs/ru/client-configuration.md) | [Guide](docs/en/client-configuration.md) |
| Сообщения, форматирование, вложения, upload | [Руководство](docs/ru/messages-and-media.md) | [Guide](docs/en/messages-and-media.md) |
| Dispatcher, middleware, state, shutdown | [Руководство](docs/ru/dispatcher.md) | [Guide](docs/en/dispatcher.md) |
| Webhook для Axum и Actix | [Руководство](docs/ru/webhooks.md) | [Guide](docs/en/webhooks.md) |
| Mini Apps | [Руководство](docs/ru/mini-apps.md) | [Guide](docs/en/mini-apps.md) |
| Цифровой ID | [Руководство](docs/ru/digital-id.md) | [Guide](docs/en/digital-id.md) |
| Проверка реального API | [Руководство](docs/ru/live-api-test.md) | [Guide](docs/en/live-api-test.md) |
| Production | [Руководство](docs/ru/production.md) | [Guide](docs/en/production.md) |
| Рецепты | [Список](docs/ru/recipes/README.md) | [Index](docs/en/recipes/README.md) |

## Примеры

В репозитории находятся 25 запускаемых примеров:

- Основа: `quickstart_bot`, `echo_bot`, `commands_bot`, `command_arguments`.
- Dispatcher: `dispatcher_filters_bot`, `dispatcher_middleware`, `scheduled_tasks`, `graceful_polling`.
- Сообщения: `keyboard_gallery`, `callback_actions`, `message_formatting`, `long_messages`, `all_attachments`.
- Upload и сеть: `streaming_upload`, `resumable_upload`, `custom_client_proxy`.
- Webhook: `webhook_axum_integration`, `webhook_actix`, `webhook_actix_integration`.
- API платформы: `comments_moderation`, `miniapp_validation`, `digital_id`, `raw_api`, `error_handling`.
- Расширенная интеграция: `live_api_test`.

`live_api_test` позволяет пользователям библиотеки проверить реальное поведение Bot API со своим отдельным тестовым ботом. Запускайте его из рабочей копии исходников maxoxide той же версии, что и зависимость приложения, и заранее ознакомьтесь с реальными побочными эффектами в [руководстве по проверке API](docs/ru/live-api-test.md).

Запуск обычного примера:

```bash
cargo run --example <name>
```

Для feature-примеров нужен соответствующий флаг:

```bash
cargo run --example webhook_actix --features webhook-actix
cargo run --example digital_id --features digital-id
```

## Важные ограничения

- `chat_id` обозначает конкретный диалог, группу или канал; `user_id` — глобального пользователя MAX.
- Нельзя передавать токен бота в query. maxoxide использует заголовок `Authorization`.
- Не отключайте проверку TLS. Для своего клиента используйте `RussianTlsExt::russian_tls()`.
- `Bot::add_members` помечен устаревшим: MAX удалил этот endpoint 30 сентября 2026 года без замены в Bot API.
- Digital ID является experimental partner integration без проверки на live service. Перед production сверяйте endpoint, authorization и пользовательские serde-модели с актуальной onboarding-документацией MAX.

Перед production-развёртыванием прочитайте [SECURITY.md](SECURITY.md), [API_SUPPORT.md](API_SUPPORT.md) и [CHANGELOG.md](CHANGELOG.md).

## Лицензия

MIT
