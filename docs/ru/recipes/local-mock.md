# Тест с local mock

Запустите server на loopback и замените `base_url`; отключайте local rate limit только для детерминированного времени теста:

```rust
let bot = Bot::builder("test-token")
    .base_url(format!("http://{address}"))
    .rate_limits(RateLimitConfig::disabled())
    .retry_policy(RetryPolicy::disabled())
    .build()?;
```

Проверяйте HTTP method, relative path, query, JSON body, Authorization header, retry и response decoding. Не направляйте тест на публичный non-HTTPS host: builder отклоняет такую конфигурацию.
