# Руководства maxoxide

Эти материалы дополняют справочник API на docs.rs. Каждому русскому руководству соответствует полноценная английская версия в [`docs/en`](../en/README.md).

1. [Начало работы](getting-started.md): установка, идентификаторы, первый бот, события и ошибки.
2. [Настройка клиента](client-configuration.md): TLS, таймауты, proxy, retry, rate limit, свой endpoint и raw-запросы.
3. [Сообщения и медиа](messages-and-media.md): форматирование, ответы, клавиатуры, все исходящие вложения, streaming и resumable upload.
4. [Dispatcher](dispatcher.md): точные команды, фильтры, middleware, state, concurrency, задачи и graceful shutdown.
5. [Webhook](webhooks.md): независимый сервис, Axum, Actix, безопасность, лимиты, backpressure и развёртывание.
6. [Mini Apps](mini-apps.md): проверка init data и `requestContact()`.
7. [Цифровой ID](digital-id.md): experimental partner integration, отдельные credentials, приватные onboarding-схемы и политика endpoint.
8. [Проверка реального API](live-api-test.md): безопасный запуск `live_api_test` со своим тестовым ботом и учётными данными.
9. [Production](production.md): transport, capacity, retry, idempotency и наблюдаемость.
10. [Рецепты](recipes/README.md): короткие готовые сценарии.

Дополнительные справочные документы:

- [Матрица поддержки API](../../API_SUPPORT.md)
- [Миграция с v2 на v3](../../MIGRATION.md)
- [Безопасность и рекомендации по развёртыванию](../../SECURITY.md)
- [История изменений](../../CHANGELOG.md)

Все примеры в репозитории являются запускаемыми Cargo targets. Используйте `cargo run --example <name>`; необходимые feature указаны в документации конкретного примера.
