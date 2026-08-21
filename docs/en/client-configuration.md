# Client configuration

## Secure defaults

`Bot::new` and `Bot::from_env` build a `reqwest` client with:

- normal certificate verification plus the embedded Russian Trusted Root CA;
- a 30-second Bot API timeout and 10-second connect timeout;
- a separate 30-minute upload/chunk timeout;
- a 30 requests/second rolling global limiter;
- a 2 operations/second rolling limiter for each recipient/resource and operation class;
- three attempts for retryable failures, exponential delay from 1 to 30 seconds;
- numeric `Retry-After` support on HTTP 429.

POST requests are not generically retried because they may create duplicates. The client only retries POST for a 429 response and the documented attachment-not-ready send response. GET, PUT, PATCH, and DELETE may retry transport and 5xx failures.

## Builder

```rust
use std::time::Duration;
use maxoxide::{Bot, RateLimitConfig, RetryPolicy};

let bot = Bot::builder("token")
    .request_timeout(Duration::from_secs(20))
    .connect_timeout(Duration::from_secs(5))
    .upload_timeout(Duration::from_secs(20 * 60))
    .rate_limits(RateLimitConfig {
        global_requests_per_second: Some(30),
        message_operations_per_second: Some(2),
    })
    .retry_policy(RetryPolicy {
        max_attempts: 4,
        initial_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(20),
    })
    .build()?;
```

Zero timeouts and zero retry attempts are configuration errors. `RateLimitConfig::disabled()` is intended for deterministic mocks or a gateway that enforces the same limits itself, not for bypassing MAX production limits.

## TLS

Never use certificate-verification bypasses. MAX currently requires a Russian trust root that is not present in every operating-system trust store. The default client merges the certificate shipped with maxoxide.

For a custom client:

```rust
use maxoxide::{Bot, RussianTlsExt};

let client = reqwest::Client::builder()
    .tcp_keepalive(std::time::Duration::from_secs(60))
    .russian_tls()?
    .build()?;
let bot = Bot::with_client("token", client)?;
```

`Bot::with_client` uses the client as-is. maxoxide cannot retroactively add proxy, connect timeout, or trust roots to a built client.

## Proxy

reqwest enables system proxies by default. It reads `HTTP_PROXY`, `HTTPS_PROXY`, and `ALL_PROXY`, and honors `NO_PROXY`. Consequently, `Bot::new`, `Bot::from_env`, and the default `BotBuilder` also use those variables.

### Direct connection

Disable automatic system proxies for the MAX client when another integration on the same server, such as Telegram, requires a global proxy that cannot reach MAX:

```rust
let bot = Bot::builder("token")
    .no_proxy()
    .build()?;
```

This setting applies only to this bot client, but it covers every request made by that client, including upload URLs returned by MAX. It does not change the proxy configuration of a separate Telegram client.

`ClientBuilder::no_proxy()` and `BotBuilder::no_proxy()` disable all proxies for their respective client. They are different from `Proxy::no_proxy(...)`, which adds destination exclusions to one explicit proxy. For a system proxy, a selective exclusion such as `NO_PROXY=.max.ru` may be enough. Check any upload hosts used by your account before relying on a domain-only exclusion.

### Explicit and authenticated proxy

Use `BotBuilder::proxy_url` for a proxy without credentials:

```rust
let bot = Bot::builder("token")
    .proxy_url("http://proxy.internal:3128")?
    .build()?;
```

For Basic proxy authentication, configure the re-exported reqwest `Proxy` before passing it to the bot:

```rust
use maxoxide::{Bot, reqwest::Proxy};

let proxy_url = std::env::var("MAX_PROXY_URL")?;
let proxy_username = std::env::var("MAX_PROXY_USERNAME")?;
let proxy_password = std::env::var("MAX_PROXY_PASSWORD")?;
let proxy = Proxy::all(proxy_url)?
    .basic_auth(&proxy_username, &proxy_password);

let bot = Bot::builder("token")
    .proxy(proxy)
    .build()?;
```

`Proxy` also supports a custom `Proxy-Authorization` value and selective exclusions:

```rust
use maxoxide::reqwest::{NoProxy, Proxy};

let proxy = Proxy::all("http://proxy.internal:3128")?
    .no_proxy(NoProxy::from_string(".max.ru"));
```

Although reqwest accepts credentials in a URL such as `http://user:password@proxy.internal:3128`, prefer separate protected variables and `basic_auth`. Do not print or debug the proxy URL, `Proxy`, or `ClientBuilder`: those values may expose credentials.

Enable `socks-proxy` for `socks5://` URLs. A prebuilt `http_client` cannot be combined with builder-level `proxy`, `proxy_url`, or `no_proxy`, because maxoxide cannot change the transport settings of an already built client. Call the corresponding reqwest methods before building a custom client instead.

See [`custom_client_proxy`](../../examples/custom_client_proxy.rs) for system, direct, and authenticated modes with configuration validation.

## Custom base URL

`base_url` supports mock servers and controlled gateways. HTTPS is mandatory except for `localhost`, `127.0.0.1`, or `::1`. Credentials, query strings, and fragments are rejected. All typed and raw API paths must remain relative, preventing an endpoint argument from redirecting authenticated requests to another host.

```rust
let bot = Bot::builder("test-token")
    .base_url("http://127.0.0.1:8080/max/")
    .build()?;
```

## Raw API escape hatch

Use typed methods whenever available. `execute` is for a newly introduced or private relative endpoint and still applies authorization, rate limiting, retries, path validation, and structured errors:

```rust
let value = bot
    .execute::<serde_json::Value>(
        reqwest::Method::GET,
        "/me",
        &[],
        None,
    )
    .await?;
```

Do not construct an absolute URL. Upload URLs returned by MAX are intentionally handled by the uploader instead of this authenticated Bot API pipeline.

## Logging and secrets

The SDK logs method, relative path, attempt, status, and retry delay at debug level. It does not log tokens, JSON request bodies, response bodies, upload URLs, proxy configuration, webhook secrets, or Digital ID credentials. Your handlers and reverse proxy need equivalent redaction rules.
