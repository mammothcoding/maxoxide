# Webhook

MAX рекомендует Webhook для production и требует публичный HTTPS endpoint с доверенным сертификатом. Обычно TLS завершается на reverse proxy или managed load balancer, а адаптер maxoxide обслуживает обычный HTTP за ним.

## Независимое ядро

`WebhookService` не зависит от Axum и Actix. Для обоих framework выполняется одна последовательность:

1. отклонить body больше заданного лимита;
2. при наличии secret вычислить hash и constant-time сравнить `X-Max-Bot-Api-Secret`;
3. потребовать JSON object;
4. при исчерпании in-flight capacity ответить 503;
5. выполнить raw и typed handlers в пределах timeout;
6. преобразовать outcome в HTTP status.

По умолчанию body limit равен 1 МиБ, dispatch timeout — 25 секунд, in-flight limit — 64. MAX ожидает ответ не дольше 30 секунд, поэтому timeout приложения должен быть ниже.

```rust
let service = WebhookService::new(dispatcher)
    .secret(std::env::var("WEBHOOK_SECRET")?)?
    .max_body_size(256 * 1024)?
    .dispatch_timeout(std::time::Duration::from_secs(20))?
    .max_in_flight(32)?;
```

Secret хранится только как SHA-256 digest. При ошибке в лог не попадает ни полученное, ни ожидаемое значение.

## Axum

Включите `webhook-axum`:

```toml
maxoxide = { version = "3", features = ["webhook-axum"] }
```

```rust
use axum::{Router, routing::get};
use maxoxide::webhook::{WebhookService, axum_adapter};

let webhook = axum_adapter::router(service, "/webhook")?;
let app = Router::new()
    .route("/health", get(|| async { "ok" }))
    .merge(webhook);
```

Adapter добавляет body limit Axum как раннюю framework-защиту, а core сохраняет собственную проверку для прямого использования service.

## Actix Web

Включите `webhook-actix`:

```toml
maxoxide = { version = "3", features = ["webhook-actix"] }
```

```rust
use actix_web::App;
use maxoxide::webhook::actix_adapter;

let scope = actix_adapter::scope(service.clone(), "/webhook")?;
let app = App::new().service(scope);
```

Возвращаемый `Scope` объединяется с application data, middleware, metrics и другими routes обычным способом Actix.

## Подписка

Зарегистрируйте тот же path и secret через Bot API:

```rust
bot.subscribe(SubscribeBody {
    url: "https://bot.example/webhook".into(),
    update_types: None,
    version: None,
    secret: Some(secret.clone()),
}).await?;
```

Не запускайте одновременно Long Polling и Webhook для одного бота. Храните secret в secret manager и меняйте subscription и deployment configuration вместе.

## Политика статусов

| Outcome | Status | Значение |
|---|---:|---|
| Accepted | 200 | Handlers завершились |
| Bad request | 400 | Некорректный JSON или не object |
| Unauthorized | 401 | Нет shared secret или он неверен |
| Too large | 413 | Body превысил лимит |
| Busy | 503 | Исчерпана in-flight capacity |
| Timeout | 503 | Dispatcher не завершился вовремя |
| Handler failure | 500 | Код приложения вернул ошибку |

После 5xx платформа может повторить доставку. Поэтому handlers должны быть idempotent по стабильному ID update/message/callback, если он доступен.

## Checklist развёртывания

- Публичный HTTPS port 443 с доверенным, не self-signed сертификатом.
- Request limit reverse proxy не ниже лимита service.
- Сохранять `X-Max-Bot-Api-Secret` и никогда его не логировать.
- Proxy timeout выше dispatch timeout service, но ниже жёсткого лимита инфраструктуры.
- Health endpoint не должен запускать bot handlers.
- Согласовать instance concurrency с downstream pools.
- Graceful server shutdown прекращает принимать requests и ждёт активные ответы.
- Отдельно мониторить 401, 413, 5xx, latency и saturation.

Полные servers находятся в `webhook_axum_integration`, `webhook_actix` и `webhook_actix_integration`.
