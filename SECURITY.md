# Security

## English

### Reporting

Do not open a public issue containing bot tokens, Digital ID or proxy credentials, webhook secrets, init data, phone numbers, QR/NFC payloads, or raw personal data. Contact the repository maintainer privately through the security channel configured on GitHub. Revoke exposed credentials before waiting for a code fix.

### Credential handling

- Load Bot API, webhook, Digital ID, and proxy secrets from a secret manager or protected environment.
- Keep credential classes separate and rotate them independently.
- Never pass a bot token in a query string; maxoxide uses `Authorization`.
- Avoid calling `Bot::token()` outside trusted server-side code.
- Keep proxy credentials outside proxy URLs when possible, and never log proxy or client builders that contain them.
- Redact request/response bodies and signed upload URLs in reverse proxies and APM.

### TLS and endpoints

- Never disable certificate verification.
- The default client merges the embedded Russian Trusted Root CA with normal verification.
- Custom clients should call `RussianTlsExt::russian_tls()`.
- Bot and Digital ID custom endpoints require HTTPS except for loopback mock servers.
- Terminate public webhooks with a trusted, non-self-signed certificate on HTTPS port 443.

### Webhooks

- Configure a high-entropy subscription secret and `WebhookService::secret`.
- Preserve but never log `X-Max-Bot-Api-Secret`.
- Keep body, in-flight, dispatch, and downstream limits bounded.
- Treat 5xx redelivery as normal and make side effects idempotent.
- Do not return detailed handler errors to the public caller.

### Mini Apps and identity

- Validate original init data only on a trusted backend.
- Bind validated user ID to a server session and enforce freshness/replay policy.
- Never ship the bot token to browser code.
- Verify `requestContact()` before using a phone number.
- Treat Digital ID payload schemas and responses as sensitive partner data; persist the minimum legally required data.

## Русский

### Сообщение об уязвимости

Не создавайте public issue с bot token, Digital ID или proxy credentials, webhook secret, init data, phone, QR/NFC payload или raw personal data. Свяжитесь с maintainer приватно через security channel GitHub. При утечке сначала отзовите credential, не дожидаясь исправления кода.

### Работа с credentials

- Загружайте Bot API, webhook, Digital ID и proxy secrets из secret manager или защищённого environment.
- Разделяйте классы credentials и ротируйте независимо.
- Никогда не передавайте bot token в query; maxoxide использует `Authorization`.
- Не вызывайте `Bot::token()` вне trusted server-side кода.
- По возможности не помещайте proxy credentials в proxy URL и никогда не логируйте содержащие их proxy или client builders.
- Удаляйте request/response body и signed upload URL из reverse proxy/APM logs.

### TLS и endpoints

- Никогда не отключайте certificate verification.
- Default client добавляет embedded Russian Trusted Root CA к обычной проверке.
- Custom client должен вызвать `RussianTlsExt::russian_tls()`.
- Custom endpoint Bot/Digital ID требует HTTPS, кроме loopback mock server.
- Public Webhook должен завершать TLS доверенным, не self-signed сертификатом на HTTPS port 443.

### Webhook

- Настройте случайный subscription secret и `WebhookService::secret`.
- Сохраняйте, но никогда не логируйте `X-Max-Bot-Api-Secret`.
- Ограничивайте body, in-flight, dispatch и downstream resources.
- Считайте 5xx redelivery нормой и делайте side effects idempotent.
- Не возвращайте подробности handler error публичному caller.

### Mini Apps и identity

- Проверяйте исходную init data только на trusted backend.
- Привязывайте validated user ID к server session и применяйте freshness/replay policy.
- Никогда не передавайте bot token в browser code.
- Проверяйте `requestContact()` до использования телефона.
- Считайте payload schema/response Digital ID чувствительными partner data и храните только юридически необходимый минимум.
