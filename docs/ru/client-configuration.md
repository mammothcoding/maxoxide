# Настройка клиента

## Безопасные значения по умолчанию

`Bot::new` и `Bot::from_env` создают `reqwest` client со следующими параметрами:

- обычная проверка сертификатов плюс встроенный Russian Trusted Root CA;
- timeout Bot API 30 секунд и connect timeout 10 секунд;
- отдельный timeout upload/чанка 30 минут;
- rolling-лимит 30 запросов в секунду;
- rolling-лимит 2 операции в секунду для каждой пары получатель/ресурс и класс операции;
- три попытки retryable-запроса с экспоненциальной задержкой от 1 до 30 секунд;
- поддержка числового `Retry-After` для HTTP 429.

Обычные POST не повторяются автоматически, так как это может создать дубликаты. Клиент повторяет POST только после 429 и документированной ошибки attachment-not-ready при отправке сообщения. Для GET, PUT, PATCH и DELETE разрешены retry после transport и 5xx ошибок.

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

Нулевые timeout и нулевое количество попыток считаются ошибкой конфигурации. `RateLimitConfig::disabled()` предназначен для детерминированных mock или gateway, который сам выполняет те же ограничения, а не для обхода production-лимитов MAX.

## TLS

Никогда не отключайте проверку сертификатов. Сейчас MAX требует российский корневой сертификат, которого может не быть в системном trust store. Клиент по умолчанию добавляет сертификат, поставляемый с maxoxide.

Для custom client:

```rust
use maxoxide::{Bot, RussianTlsExt};

let client = reqwest::Client::builder()
    .tcp_keepalive(std::time::Duration::from_secs(60))
    .russian_tls()?
    .build()?;
let bot = Bot::with_client("token", client)?;
```

`Bot::with_client` использует готовый client без изменений. maxoxide не может задним числом добавить proxy, connect timeout или trust roots в уже созданный client.

## Proxy

reqwest по умолчанию включает системные proxy. Он читает `HTTP_PROXY`, `HTTPS_PROXY` и `ALL_PROXY`, а также учитывает `NO_PROXY`. Поэтому `Bot::new`, `Bot::from_env` и default `BotBuilder` тоже используют эти variables.

### Прямое подключение

Отключите автоматические системные proxy для MAX client, если другой интеграции на том же сервере, например Telegram, нужен глобальный proxy, через который MAX недоступен:

```rust
let bot = Bot::builder("token")
    .no_proxy()
    .build()?;
```

Настройка действует только на этот bot client, но охватывает все его запросы, включая upload URLs, которые возвращает MAX. Конфигурация отдельного Telegram client не изменяется.

`ClientBuilder::no_proxy()` и `BotBuilder::no_proxy()` полностью отключают proxy для соответствующего client. Это не то же самое, что `Proxy::no_proxy(...)`, который добавляет исключения по адресам к одному явному proxy. Для системного proxy может быть достаточно selective exclusion вроде `NO_PROXY=.max.ru`. Перед использованием только доменного исключения проверьте upload hosts, доступные вашему аккаунту.

### Явный proxy и аутентификация

Для proxy без credentials используйте `BotBuilder::proxy_url`:

```rust
let bot = Bot::builder("token")
    .proxy_url("http://proxy.internal:3128")?
    .build()?;
```

Для Basic-аутентификации proxy настройте re-exported reqwest `Proxy`, а затем передайте его bot-у:

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

`Proxy` также поддерживает произвольное значение `Proxy-Authorization` и selective exclusions:

```rust
use maxoxide::reqwest::{NoProxy, Proxy};

let proxy = Proxy::all("http://proxy.internal:3128")?
    .no_proxy(NoProxy::from_string(".max.ru"));
```

Хотя reqwest принимает credentials в URL вида `http://user:password@proxy.internal:3128`, предпочитайте отдельные защищённые variables и `basic_auth`. Не выводите и не логируйте proxy URL, `Proxy` или `ClientBuilder`: эти значения могут раскрыть credentials.

Для `socks5://` включите feature `socks-proxy`. Готовый `http_client` нельзя сочетать с builder-level `proxy`, `proxy_url` или `no_proxy`, потому что maxoxide не может изменить transport settings уже созданного client. Для custom client вызывайте соответствующие методы reqwest до `build()`.

Системный, прямой и authenticated режимы с проверкой конфигурации показаны в [`custom_client_proxy`](../../examples/custom_client_proxy.rs).

## Свой base URL

`base_url` нужен для mock server и контролируемого gateway. HTTPS обязателен, кроме `localhost`, `127.0.0.1` и `::1`. Credentials, query и fragment запрещены. Пути typed/raw API должны быть относительными, поэтому параметр endpoint не сможет перенаправить авторизованный запрос на другой host.

```rust
let bot = Bot::builder("test-token")
    .base_url("http://127.0.0.1:8080/max/")
    .build()?;
```

## Raw API

По возможности используйте typed methods. `execute` предназначен для нового или приватного относительного endpoint и всё равно применяет Authorization, rate limit, retry, проверку пути и structured errors:

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

Не передавайте абсолютный URL. Upload URL, который вернул MAX, специально обрабатывается uploader-ом вне авторизованного Bot API pipeline.

## Логи и секреты

SDK пишет на debug-уровне method, относительный path, номер попытки, status и retry delay. Он не логирует токены, JSON request/response body, upload URL, proxy configuration, webhook secret и Digital ID credentials. Такие же правила редактирования нужны вашим handlers и reverse proxy.
