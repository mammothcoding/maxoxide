# Webhooks

MAX recommends webhook delivery for production and requires a publicly reachable HTTPS endpoint with a trusted certificate. TLS is normally terminated by a reverse proxy or managed load balancer; maxoxide adapters serve ordinary HTTP behind it.

## Neutral core

`WebhookService` is independent of Axum and Actix. It applies the same sequence for both:

1. reject a body larger than the configured limit;
2. hash and constant-time compare `X-Max-Bot-Api-Secret` when configured;
3. require a JSON object;
4. reject excess in-flight work with 503;
5. dispatch raw and typed handlers within a timeout;
6. return an outcome mapped to HTTP status.

Defaults are a 1 MiB body limit, 25-second dispatch timeout, and 64 in-flight requests. MAX expects a response within 30 seconds, so keep application timeout below that limit.

```rust
let service = WebhookService::new(dispatcher)
    .secret(std::env::var("WEBHOOK_SECRET")?)?
    .max_body_size(256 * 1024)?
    .dispatch_timeout(std::time::Duration::from_secs(20))?
    .max_in_flight(32)?;
```

The secret is stored only as SHA-256 digest. Failed checks are logged without the supplied or expected value.

## Axum

Enable `webhook-axum`:

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

The adapter adds Axum's body limit as an early framework guard and retains the core check for direct service use.

## Actix Web

Enable `webhook-actix`:

```toml
maxoxide = { version = "3", features = ["webhook-actix"] }
```

```rust
use actix_web::App;
use maxoxide::webhook::actix_adapter;

let scope = actix_adapter::scope(service.clone(), "/webhook")?;
let app = App::new().service(scope);
```

The returned `Scope` can be combined with application data, middleware, metrics, and unrelated routes in the normal Actix configuration.

## Subscription

Register the same path and secret through the Bot API:

```rust
bot.subscribe(SubscribeBody {
    url: "https://bot.example/webhook".into(),
    update_types: None,
    version: None,
    secret: Some(secret.clone()),
}).await?;
```

Do not run long polling and webhook delivery simultaneously for the same bot. Store the secret in a secret manager and rotate it by replacing the subscription and deployment configuration together.

## Status policy

| Outcome | Status | Meaning |
|---|---:|---|
| Accepted | 200 | Handlers completed |
| Bad request | 400 | Invalid JSON/non-object payload |
| Unauthorized | 401 | Missing or wrong shared secret |
| Too large | 413 | Body exceeded the configured limit |
| Busy | 503 | In-flight capacity is exhausted |
| Timeout | 503 | Dispatcher did not complete in time |
| Handler failure | 500 | Application handling returned an error |

5xx responses allow platform retry. Handlers therefore need idempotency using a stable update/message/callback identity where available.

## Deployment checklist

- Public HTTPS port 443 with a trusted, non-self-signed certificate.
- Reverse-proxy request limit no lower than the service limit.
- Preserve `X-Max-Bot-Api-Secret`; never log it.
- Proxy timeout above service dispatch timeout but below infrastructure hard limits.
- Health endpoint must not execute bot handlers.
- Bound instance concurrency and downstream pools consistently.
- Graceful server shutdown should stop accepting requests and wait for in-flight responses.
- Monitor 401, 413, 5xx, latency, and saturation separately.

See `webhook_axum_integration`, `webhook_actix`, and `webhook_actix_integration` for complete servers.
