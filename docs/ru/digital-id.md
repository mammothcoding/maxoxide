# Цифровой ID (experimental)

> **Статус: experimental partner integration.** Клиент не проверен на live-сервисе Digital ID. Перед production сверяйте endpoint, формат authorization, модели запроса и ответа с актуальной onboarding-документацией, которую MAX выдал вашей организации.

MAX Цифровой ID — отдельный партнёрский продукт для подходящих юридических лиц и ИП РФ. Он не использует токен бота и не относится к Bot API domain.

Включите client явно:

```toml
maxoxide = { version = "3", features = ["digital-id"] }
```

## Credentials и endpoint

Production endpoint проверки возраста:

```text
https://ext-api2.max.ru/v2/business/pos/age-verification
```

Создайте `DigitalIdClient` с отдельным годовым partner token, который выдан при подключении:

```rust
use maxoxide::digital_id::DigitalIdClient;

let client = DigitalIdClient::new(digital_id_token)?;
```

Не передавайте `Bot::token()`, не используйте общую переменную secret и не применяйте wrapper, автоматически добавляющий Bot API Authorization. Credentials Цифрового ID должны храниться и ротироваться независимо.

## Partner-owned schemas

Публичная страница MAX не публикует body schema запроса/ответа POS: MAX выдаёт её в партнёрской инструкции после подключения сервиса. Поэтому maxoxide не выдумывает struct, который мог бы неверно сериализовать юридически значимые identity data.

Используйте serde-модели из своей onboarding-документации:

```rust
#[derive(serde::Serialize)]
struct PartnerAgeRequest {
    // Поля из актуальной партнёрской спецификации MAX.
}

#[derive(serde::Deserialize)]
struct PartnerAgeResponse {
    // Поля из актуальной партнёрской спецификации MAX.
}

let response: PartnerAgeResponse = client.verify_age(&request).await?;
```

`verify_age_raw` принимает и возвращает `serde_json::Value` для controlled discovery/migration. В production лучше собственные строгие модели: исчезновение или переименование поля станет явной ошибкой.

## Сетевая политика

Клиент по умолчанию не отключает TLS verification и добавляет тот же российский trust root, что используется для API MAX. Request timeout — 30 секунд. Custom `reqwest::Client` подходит для enterprise proxy, mTLS gateway или observability:

```rust
let client = DigitalIdClient::with_client(token, http_client)?
    .timeout(std::time::Duration::from_secs(10))?;
```

Endpoint override требует HTTPS, кроме loopback host. Это разрешает локальные mock tests, но не упрощает случайную настройку plaintext production endpoint.

## Обработка ошибок

Неуспешные ответы используют тот же контракт `MaxError::Api(ApiError)`, что Bot API. Доступны HTTP status, optional machine code, human message и не более 4 КиБ raw diagnostics. Не логируйте raw response: проверка личности может содержать персональные данные даже в ошибке.

## Готовность к production

Generic transport не гарантирует совместимость с private production contract. Для реальной интеграции нужны partner credentials, актуальная приватная payload schema, consent и контролируемые POS-данные.

Перед production-проверкой:

- получите свежую integration schema непосредственно у MAX;
- создайте строгие required fields и документированные enums;
- согласуйте timeout и fail-closed behavior с владельцем процесса;
- не храните QR/NFC payload и raw identity response без юридической необходимости;
- удаляйте credentials и personal data из tracing, proxy, APM и panic reports;
- ротируйте Digital ID token по партнёрской процедуре с учётом всех подключённых касс.

Experimental feature-gated skeleton находится в `digital_id`.
