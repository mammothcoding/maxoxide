# Mount a webhook in an existing application

Create Dispatcher once, wrap it in `WebhookService`, and merge only the adapter route:

```rust
let service = WebhookService::new(dispatcher).secret(secret)?;
let webhook = axum_adapter::router(service, "/integrations/max")?;
let app = application_router.merge(webhook);
```

For Actix, register `actix_adapter::scope(service, "/integrations/max")?` on the existing `App`. Keep application authentication middleware from consuming or replacing `X-Max-Bot-Api-Secret` before the adapter sees it.
