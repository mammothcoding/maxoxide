# Digital ID (experimental)

> **Status: experimental partner integration.** The client has not been verified against the live Digital ID service. Before production use, confirm the endpoint, authorization format, request model, and response model against the current onboarding documentation issued to your organization by MAX.

MAX Digital ID is a separate partner product for eligible Russian legal entities and individual entrepreneurs. It is not authenticated with a bot token and is not part of the Bot API domain.

Enable the client explicitly:

```toml
maxoxide = { version = "3", features = ["digital-id"] }
```

## Credentials and endpoint

The production age-verification endpoint is:

```text
https://ext-api2.max.ru/v2/business/pos/age-verification
```

Create `DigitalIdClient` with the separate annual partner token issued during onboarding:

```rust
use maxoxide::digital_id::DigitalIdClient;

let client = DigitalIdClient::new(digital_id_token)?;
```

Do not pass `Bot::token()`, reuse a bot secret variable, or share a client wrapper that automatically inserts Bot API authorization. Store and rotate Digital ID credentials independently.

## Partner-owned schemas

The public MAX product page does not publish the POS request/response body schema; MAX provides it in the partner integration instructions after service activation. maxoxide therefore does not invent a struct that could serialize incorrect legal/identity data.

Use your onboarding models with serde:

```rust
#[derive(serde::Serialize)]
struct PartnerAgeRequest {
    // Fields from your current MAX partner specification.
}

#[derive(serde::Deserialize)]
struct PartnerAgeResponse {
    // Fields from your current MAX partner specification.
}

let response: PartnerAgeResponse = client.verify_age(&request).await?;
```

`verify_age_raw` accepts/returns `serde_json::Value` for schema discovery and controlled migrations. Prefer owned typed models in production so missing/renamed fields fail visibly.

## Network policy

The default client keeps TLS verification enabled and merges the same Russian trust root used for MAX APIs. It has a 30-second request timeout. A custom `reqwest::Client` can be supplied for enterprise proxy, mTLS gateway, or observability settings:

```rust
let client = DigitalIdClient::with_client(token, http_client)?
    .timeout(std::time::Duration::from_secs(10))?;
```

Endpoint override requires HTTPS except for loopback hosts, which permits local mock tests without making arbitrary plaintext production endpoints easy to configure.

## Error handling

Non-success responses use the same `MaxError::Api(ApiError)` contract as Bot API calls. The HTTP status, optional machine code, human message, and at most 4 KiB of raw diagnostics are available. Do not log raw responses: identity verification may involve personal data even when the API returns an error.

## Production readiness

The generic transport does not establish compatibility with the private production contract. A real integration requires partner credentials, the current private payload specification, consent, and controlled POS data.

Before enabling a production check:

- obtain the latest integration schema directly from MAX;
- model strict required fields and documented enums;
- define timeout/fail-closed behavior with the business owner;
- avoid persisting QR/NFC payloads or raw identity responses unless legally required;
- redact credentials and personal data from tracing, proxy, APM, and panic reports;
- rotate the Digital ID token according to the partner procedure and account for all connected POS devices.

See `digital_id` for the experimental feature-gated runnable skeleton.
