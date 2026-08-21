# Mini Apps

Never trust `window.WebApp.initDataUnsafe` on a server. Send the original URL-encoded `window.WebApp.initData` string to your backend and validate it with the bot token there.

## Launch validation

```rust
use maxoxide::miniapp::MiniAppValidator;

let validator = MiniAppValidator::new(bot_token)?;
let data = validator.validate(init_data)?;
if let Some(user) = data.user {
    println!("Authenticated MAX user {}", user.id);
}
```

The validator follows the current MAX algorithm:

1. parse URL-encoded key/value pairs and reject every duplicate key;
2. remove exactly one required `hash`;
3. sort decoded values by key and join `key=value` lines with `\n`;
4. derive `secret_key = HMAC-SHA256(key="WebAppData", data=bot_token)`;
5. calculate `HMAC-SHA256(secret_key, launch_params)`;
6. decode the supplied 64-character hex hash and compare fixed-size bytes in constant time;
7. require `auth_date`, reject excessive future clock skew, and enforce freshness;
8. deserialize optional `user` and `chat` JSON only after authenticity succeeds.

The default maximum age is one hour, matching the MAX recommendation, with 30 seconds of positive clock-skew tolerance:

```rust
let validator = MiniAppValidator::new(token)?
    .max_age(std::time::Duration::from_secs(15 * 60))
    .future_tolerance(std::time::Duration::from_secs(10));
```

`validate_at` accepts an explicit Unix timestamp and is useful for deterministic tests. Do not use it with a client-provided time in production.

## Returned fields

`MiniAppInitData` exposes parsed `query_id`, `ip`, `auth_date`, `user`, `chat`, and `start_param`. The complete decoded map remains in `fields` so newly added MAX parameters are not lost.

Treat IP, username, photo URL, chat membership, and optional fields as untrusted application attributes even after the signature is valid. The signature proves MAX issued the payload; it does not grant authorization in your own system.

## `requestContact()`

MAX Bridge returns phone, `authDate`, and hash. Verify that the phone belongs to the current MAX account using the same bot token and the known validated user ID:

```rust
use maxoxide::miniapp::MiniAppContact;

let contact = MiniAppContact {
    phone: response.phone,
    auth_date: response.auth_date,
    hash: response.hash,
};
if !validator.validate_contact(&contact, validated_user_id) {
    return Err("invalid contact signature".into());
}
```

The signed string is `auth_date=...\nphone=...\nuser_id=...`, and the leading `+` is removed from the phone as required by MAX. The signature is HMAC-SHA256 with the bot token directly as key.

Apply a separate freshness check to contact `auth_date` according to your business workflow; `validate_contact` verifies only ownership/signature because the bridge documents the field as a string and applications may use different acceptance windows.

## Security checklist

- Validate only on a trusted backend; never embed the bot token in browser code.
- Accept init data only once per login/session if replay matters to your application.
- Enforce HTTPS and CSRF/session binding on the endpoint receiving init data.
- Bind the validated `user.id` to the server session rather than accepting a later user ID field.
- Log validation reason categories, not raw init data, phone numbers, hashes, or tokens.
- Test duplicate keys, malformed hashes, tampering, expiration, future timestamps, and JSON type errors.

See `miniapp_validation` for a runnable command-line verifier.
