# Dispatcher

`Dispatcher` проверяет handlers в порядке регистрации. Запускается первый подходящий typed handler. Raw handlers выполняются до typed deserialization и подходят для аудита или только что добавленных событий.

## Фильтры

Используйте готовые handlers или составляйте `Filter`:

```rust
use maxoxide::{Context, Filter};

dispatcher.on_command("/start", handler);
dispatcher.on_update(
    Filter::message() & Filter::chat(chat_id) & Filter::has_media(),
    |context: Context| async move {
        // Обработать медиа из одного чата.
        Ok(())
    },
);
```

Встроены фильтры сообщений, изменений, комментариев, callback и точного callback payload, жизненного цикла бота, пользователя и чата, смены прав администратора бота, constructed-message events, точного текста, подстроки и регулярного выражения, ID чата и отправителя, типов вложений, неизвестных updates, AND, OR, NOT и пользовательских предикатов. Для новых событий используйте `on_comment_created`, `on_comment_edited`, `on_comment_removed` и `on_bot_admin_permissions_changed`.

Создание regex может завершиться ошибкой и возвращает локальный `ValidationError`, а не искусственную API error.

## Точные команды

Парсер сравнивает только первый токен до whitespace. Для `/status` подходят `/status`, `/status verbose` и `/status@bot verbose`, но не `/status_all`. Если зарегистрированная команда содержит `@`, весь токен должен точно совпасть с этим mention.

```rust
dispatcher.on_command("/say", |context: Context| async move {
    let text = context.command_arguments().unwrap_or("Аргументы не переданы");
    // ...
    Ok(())
});
```

## Типизированный state

State индексируется Rust `TypeId`, разделяется через `Arc` и извлекается без строковых ключей и downcast в коде приложения:

```rust
struct DatabasePool(/* ... */);

let mut dispatcher = Dispatcher::new(bot).with_state(DatabasePool(/* ... */));
dispatcher.on_message(|context: Context| async move {
    let pool = context.state::<DatabasePool>().expect("state зарегистрирован");
    // использовать pool
    Ok(())
});
```

Регистрируйте не более одного значения каждого конкретного типа. Для нескольких однотипных ресурсов используйте разные newtype wrappers.

## Middleware

Middleware выполняются в порядке регистрации и получают цепочку `Next`:

```rust
dispatcher.middleware(|context, next| async move {
    let started = std::time::Instant::now();
    let result = next.run(context).await;
    tracing::info!(elapsed = ?started.elapsed(), "handler завершён");
    result
});
```

Middleware может отклонить update, вернув ошибку без `next`, добавить tracing context, проверить авторизацию или выполнить post-processing. Не храните `Context` бесконечно: вместе с ним остаётся полный payload события.

## Concurrency и backpressure

По умолчанию `max_concurrent_handlers(64)`. Polling создаёт задачи для updates, а semaphore не разрешает одновременно выполнять больше handlers. У Webhook есть отдельный HTTP in-flight limit, после которого применяется тот же semaphore Dispatcher.

Выбирайте значение по размеру downstream connection pools и rate limits, а не только по числу CPU. Вызовы MAX Bot API дополнительно ограничены клиентскими rate limiters.

## Ошибки

`dispatch` и `dispatch_raw` возвращают `Result<()>`. Поэтому webhook adapter может ответить 500 и запросить retry платформы при ошибке приложения. В polling `on_error` наблюдает ошибки handlers/tasks:

```rust
let dispatcher = Dispatcher::new(bot).on_error(|error| {
    tracing::error!(%error, "ошибка dispatcher");
});
```

Callback получает borrowed error, поэтому исходный result не теряется до применения transport policy.

## Startup и задачи

`on_start` выполняется перед polling. `task(interval, handler)` запускает периодическую async-работу после startup. Scheduled loops слушают общий shutdown token и завершаются без ожидания следующего interval.

## Graceful shutdown

`start_polling()` возвращает `Result<()>`, обрабатывает Ctrl+C, прекращает получение updates, отменяет scheduled loops и ждёт активные handlers. Drain timeout по умолчанию — 30 секунд:

```rust
let dispatcher = Dispatcher::new(bot)
    .shutdown_timeout(std::time::Duration::from_secs(15));
let shutdown = dispatcher.shutdown_handle();

tokio::spawn(async move {
    application_shutdown_signal().await;
    shutdown.shutdown();
});

dispatcher.start_polling().await?;
```

После timeout оставшиеся handler tasks прерываются. Делайте handlers cancellation-safe, а внешние операции — idempotent.
