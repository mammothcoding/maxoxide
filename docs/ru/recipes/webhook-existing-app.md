# Webhook в существующем приложении

Создайте Dispatcher один раз, оберните его в `WebhookService` и добавьте только route адаптера:

```rust
let service = WebhookService::new(dispatcher).secret(secret)?;
let webhook = axum_adapter::router(service, "/integrations/max")?;
let app = application_router.merge(webhook);
```

В Actix зарегистрируйте `actix_adapter::scope(service, "/integrations/max")?` в существующем `App`. Middleware авторизации приложения не должно удалять или заменять `X-Max-Bot-Api-Secret` до адаптера.
