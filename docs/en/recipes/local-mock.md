# Test against a local mock

Bind a server to loopback and override `base_url`; disable local limiting only to keep deterministic test timing:

```rust
let bot = Bot::builder("test-token")
    .base_url(format!("http://{address}"))
    .rate_limits(RateLimitConfig::disabled())
    .retry_policy(RetryPolicy::disabled())
    .build()?;
```

Assert HTTP method, relative path, query, JSON body, Authorization header, retries, and response decoding. Never point tests at a public non-HTTPS host; the builder rejects that configuration.
