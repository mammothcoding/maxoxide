# maxoxide guides

These guides complement the API reference on docs.rs. Every English guide has an equivalent Russian version under [`docs/ru`](../ru/README.md).

1. [Getting started](getting-started.md): installation, IDs, first bot, updates, and errors.
2. [Client configuration](client-configuration.md): TLS, timeouts, proxy, retries, rate limits, custom endpoint, and raw requests.
3. [Messages and media](messages-and-media.md): formatting, replies, keyboards, all attachment requests, streaming, and resumable uploads.
4. [Dispatcher](dispatcher.md): exact commands, filters, middleware, state, concurrency, scheduled work, and graceful shutdown.
5. [Webhooks](webhooks.md): neutral service, Axum, Actix, security, limits, backpressure, and deployment.
6. [Mini Apps](mini-apps.md): init data and `requestContact()` validation.
7. [Digital ID](digital-id.md): experimental partner integration, separate credentials, private onboarding schemas, and endpoint policy.
8. [Live API validation](live-api-test.md): run `live_api_test` safely with your own test bot and credentials.
9. [Production](production.md): transport, capacity, retries, idempotency, and observability.
10. [Recipes](recipes/README.md): focused copyable patterns.

Additional reference documents:

- [API support matrix](../../API_SUPPORT.md)
- [v2 to v3 migration](../../MIGRATION.md)
- [Security policy and deployment guidance](../../SECURITY.md)
- [Changelog](../../CHANGELOG.md)

All repository examples are runnable Cargo targets. Use `cargo run --example <name>`; feature-specific examples show the required feature in their module documentation.
