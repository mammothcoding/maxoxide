# Mini Apps

На сервере нельзя доверять `window.WebApp.initDataUnsafe`. Отправляйте на backend исходную URL-encoded строку `window.WebApp.initData` и проверяйте её там с помощью токена бота.

## Проверка запуска

```rust
use maxoxide::miniapp::MiniAppValidator;

let validator = MiniAppValidator::new(bot_token)?;
let data = validator.validate(init_data)?;
if let Some(user) = data.user {
    println!("Проверен пользователь MAX {}", user.id);
}
```

Validator реализует текущий алгоритм MAX:

1. разобрать URL-encoded key/value и отклонить любое повторение ключа;
2. извлечь ровно один обязательный `hash`;
3. отсортировать decoded values по key и объединить строки `key=value` через `\n`;
4. получить `secret_key = HMAC-SHA256(key="WebAppData", data=bot_token)`;
5. вычислить `HMAC-SHA256(secret_key, launch_params)`;
6. декодировать переданный 64-символьный hex hash и constant-time сравнить массивы фиксированного размера;
7. потребовать `auth_date`, отклонить слишком большую положительную clock skew и проверить срок;
8. преобразовать optional JSON `user` и `chat` только после успешной аутентичности.

По умолчанию максимальный возраст равен рекомендованному MAX одному часу, а допустимое расхождение часов вперёд — 30 секунд:

```rust
let validator = MiniAppValidator::new(token)?
    .max_age(std::time::Duration::from_secs(15 * 60))
    .future_tolerance(std::time::Duration::from_secs(10));
```

`validate_at` принимает явный Unix timestamp и удобен в детерминированных тестах. В production нельзя передавать в него время от клиента.

## Возвращаемые поля

`MiniAppInitData` содержит разобранные `query_id`, `ip`, `auth_date`, `user`, `chat`, `start_param`. Полная decoded map остаётся в `fields`, поэтому новые параметры MAX не теряются.

Даже после корректной подписи считайте IP, username, photo URL, membership чата и optional fields атрибутами приложения, а не автоматическим разрешением. Подпись доказывает, что payload выдал MAX, но не выдаёт права в вашей системе.

## `requestContact()`

MAX Bridge возвращает phone, `authDate` и hash. Проверьте, что телефон принадлежит текущему аккаунту MAX, используя тот же bot token и известный validated user ID:

```rust
use maxoxide::miniapp::MiniAppContact;

let contact = MiniAppContact {
    phone: response.phone,
    auth_date: response.auth_date,
    hash: response.hash,
};
if !validator.validate_contact(&contact, validated_user_id) {
    return Err("некорректная подпись contact".into());
}
```

Подписывается строка `auth_date=...\nphone=...\nuser_id=...`; ведущий `+` телефона удаляется по требованию MAX. Подпись — HMAC-SHA256 непосредственно с bot token в качестве key.

Проверяйте срок `auth_date` contact отдельно по правилам своего процесса. `validate_contact` подтверждает только ownership/signature, потому что Bridge документирует поле строкой, а допустимое окно зависит от приложения.

## Checklist безопасности

- Проверять только на trusted backend; никогда не помещать bot token в browser code.
- При риске replay принимать init data один раз на login/session.
- Использовать HTTPS и привязку CSRF/session на endpoint приёма init data.
- Привязать проверенный `user.id` к server session, а не принимать user ID в следующих запросах.
- Логировать категории причин, но не raw init data, телефоны, hashes и tokens.
- Тестировать duplicate keys, malformed hash, tampering, expiration, future timestamps и ошибки типов JSON.

Запускаемый verifier находится в `miniapp_validation`.
