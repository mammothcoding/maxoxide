# Production

## Transport choice

Use HTTPS webhook delivery in production. MAX explicitly describes long polling as unsuitable for production due to rate and retention constraints. Keep long polling for local development, manual test bots, and recovery diagnostics.

Production webhook instances should be horizontally safe: handlers may receive retries and duplicate delivery. Use a stable message ID, callback ID, session ID, or domain idempotency key before committing side effects.

## Capacity

Three limits work together:

- `WebhookService::max_in_flight` bounds buffered HTTP work;
- `Dispatcher::max_concurrent_handlers` bounds application handlers;
- `RateLimitConfig` bounds outgoing MAX requests.

Also align database, queue, and third-party client pools. Increasing Dispatcher concurrency above a downstream pool only creates waiting tasks and raises timeout risk.

The recipient limiter map is bounded and evicts idle entries, preventing an unbounded set of one-off user/resource IDs from growing for the process lifetime.

## Retry and idempotency

- GET, PUT, PATCH, and DELETE retry transport/5xx/429 failures according to `RetryPolicy`.
- POST retries only 429 and attachment-not-ready message sends.
- Numeric `Retry-After` takes precedence but is capped by `RetryPolicy::max_delay`.
- Resumable uploads retry only the active chunk.
- Webhook 5xx can cause redelivery; application side effects must be idempotent.

Do not enable a broad proxy retry for POST without an idempotency contract.

## Observability

Recommended metrics:

- webhook requests by status and outcome;
- dispatch duration, active handlers, and saturation;
- Bot API requests by method/path template/status/attempt;
- rate-limiter wait duration;
- upload bytes, duration, chunk retries, cancellation, and failures;
- unknown update and unknown enum counts;
- handler failures by stable category, not full payload.

Never use token, secret, upload URL, phone, init data, QR/NFC payload, or raw identity response as a label. High-cardinality message/user IDs should not be metric labels either.
