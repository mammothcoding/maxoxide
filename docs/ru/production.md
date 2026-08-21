# Production

## Выбор транспорта

В production используйте HTTPS Webhook. MAX прямо указывает, что Long Polling не подходит для production из-за ограничений скорости и срока хранения событий. Оставьте polling для local development, ручного test bot и recovery diagnostics.

Webhook instances должны безопасно работать горизонтально: handlers могут получить retry и duplicate delivery. До побочного эффекта используйте стабильный message ID, callback ID, session ID или domain idempotency key.

## Capacity

Совместно действуют три ограничения:

- `WebhookService::max_in_flight` ограничивает HTTP work;
- `Dispatcher::max_concurrent_handlers` ограничивает application handlers;
- `RateLimitConfig` ограничивает исходящие запросы MAX.

Согласуйте с ними pools базы, очередей и third-party clients. Concurrency Dispatcher выше downstream pool только создаёт ожидающие tasks и повышает риск timeout.

Карта recipient limiters ограничена по размеру и удаляет idle entries, поэтому множество одноразовых user/resource IDs не растёт весь срок жизни процесса.

## Retry и idempotency

- GET, PUT, PATCH и DELETE повторяют transport/5xx/429 по `RetryPolicy`.
- POST повторяется только после 429 и attachment-not-ready при отправке сообщения.
- Числовой `Retry-After` имеет приоритет, но ограничен `RetryPolicy::max_delay`.
- Resumable upload повторяет только активный chunk.
- После webhook 5xx возможна повторная доставка; side effects приложения должны быть idempotent.

Не включайте общий proxy retry для POST без idempotency contract.

## Наблюдаемость

Рекомендуемые metrics:

- webhook requests по status и outcome;
- dispatch duration, active handlers и saturation;
- Bot API requests по method/path template/status/attempt;
- время ожидания rate limiter;
- upload bytes, duration, chunk retries, cancellation и failures;
- количество unknown update и enum;
- handler failures по стабильной категории, но не полному payload.

Никогда не используйте token, secret, upload URL, phone, init data, QR/NFC payload или raw identity response как label. Message/user IDs с высокой cardinality тоже не должны быть metric labels.
