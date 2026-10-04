# Dispatcher

`Dispatcher` routes updates in registration order. The first matching typed handler runs. Raw handlers run before typed deserialization and are useful for audits or newly introduced events.

## Filters

Register convenience handlers or compose `Filter` values:

```rust
use maxoxide::{Context, Filter};

dispatcher.on_command("/start", handler);
dispatcher.on_update(
    Filter::message() & Filter::chat(chat_id) & Filter::has_media(),
    |context: Context| async move {
        // Handle media from one chat.
        Ok(())
    },
);
```

Built-ins cover messages, edits, comments, callbacks and exact callback payloads, bot/user/chat lifecycle events, bot administrator permission changes, constructed-message events, exact/contains/regex text, chat/sender IDs, attachment kinds, unknown updates, AND, OR, NOT, and custom predicates. Use `on_comment_created`, `on_comment_edited`, `on_comment_removed`, and `on_bot_admin_permissions_changed` for the corresponding events.

Regex construction is fallible and returns a local `ValidationError` instead of an API-shaped error.

## Exact commands

The command parser compares only the first whitespace-delimited token. A configured `/status` accepts `/status`, `/status verbose`, and `/status@bot verbose`, but rejects `/status_all`. If the configured string itself contains `@`, the token must match that complete mention.

```rust
dispatcher.on_command("/say", |context: Context| async move {
    let text = context.command_arguments().unwrap_or("Nothing supplied");
    // ...
    Ok(())
});
```

## Typed state

State is keyed by Rust `TypeId`, shared through `Arc`, and retrieved without string keys or downcasts in application code:

```rust
struct DatabasePool(/* ... */);

let mut dispatcher = Dispatcher::new(bot).with_state(DatabasePool(/* ... */));
dispatcher.on_message(|context: Context| async move {
    let pool = context.state::<DatabasePool>().expect("registered state");
    // use pool
    Ok(())
});
```

Register at most one value of each concrete type. Wrap same-typed resources in distinct newtypes.

## Middleware

Middleware runs in registration order and receives a `Next` chain:

```rust
dispatcher.middleware(|context, next| async move {
    let started = std::time::Instant::now();
    let result = next.run(context).await;
    tracing::info!(elapsed = ?started.elapsed(), "handler completed");
    result
});
```

Middleware can reject an update by returning an error without calling `next`, enrich external tracing context, enforce authorization, or perform post-processing. It must not retain `Context` indefinitely because that also retains the update payload.

## Concurrency and backpressure

`max_concurrent_handlers(64)` is the default. Polling spawns update tasks, while a semaphore prevents more than the configured count from executing handlers. Webhook delivery has a separate in-flight limit at the HTTP layer and then uses the same Dispatcher semaphore.

Choose a value based on downstream connection pools and rate limits, not CPU count alone. MAX Bot API calls are also constrained by the client rate limiters.

## Errors

`dispatch` and `dispatch_raw` return `Result<()>`. This lets webhook adapters return 500 and request a platform retry when application handling fails. During polling, `on_error` observes handler/task failures:

```rust
let dispatcher = Dispatcher::new(bot).on_error(|error| {
    tracing::error!(%error, "dispatcher failure");
});
```

The callback borrows the error, preventing accidental loss of the original result before transport policy is applied.

## Startup and scheduled work

`on_start` runs before polling begins. `task(interval, handler)` starts periodic asynchronous work after startup. Scheduled loops observe the same shutdown token and stop without another interval delay.

## Graceful shutdown

`start_polling()` returns `Result<()>`, handles Ctrl+C, stops fetching updates, cancels scheduled loops, and waits for in-flight handlers. The default drain timeout is 30 seconds:

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

After the timeout, remaining handler tasks are aborted. Make handlers cancellation-safe and keep external operations idempotent.
