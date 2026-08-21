# Getting started

## Requirements

- Rust 1.85 or newer.
- A bot created in MAX for Partners and its Bot API token.
- HTTPS webhook infrastructure for production. MAX describes long polling as a development and testing transport.

Add the crate and Tokio:

```toml
[dependencies]
maxoxide = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
```

Keep the token outside source control:

```bash
export MAX_BOT_TOKEN='...'
cargo run --example quickstart_bot
```

`Bot::from_env()` and every other constructor return `Result`. Empty tokens, invalid headers, unsafe base URLs, conflicting client/proxy settings, and zero timeouts fail before the first request.

## First dispatcher

```rust
use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut dispatcher = Dispatcher::new(Bot::from_env()?);
    dispatcher.on_command("/start", |context: Context| async move {
        if let Update::MessageCreated { message, .. } = &context.update {
            context.bot.send_text_to_chat(message.chat_id(), "Ready").await?;
        }
        Ok(())
    });
    dispatcher.start_polling().await
}
```

Command matching is token-exact. `/start`, `/start one`, and `/start@your_bot one` match `/start`; `/starter` does not. Read the remainder through `Context::command_arguments()`.

## `chat_id`, `user_id`, and `post_id`

- `chat_id` identifies a concrete private dialog, group, or channel. Use `send_*_to_chat` when replying to an update.
- `user_id` is a stable global MAX user ID. Use `send_*_to_user` when no dialog ID is available.
- `Recipient::post_id` identifies a channel post when MAX includes post context, especially around comments.

Do not substitute one identifier for another. A private message commonly contains both `message.sender.user_id` and `message.recipient.chat_id`.

## Bot information and commands

`get_me()` returns `BotInfo`, not a generic user:

```rust
let me = bot.get_me().await?;
println!("{} ({})", me.display_name(), me.user_id);
```

Replace the complete command menu through the documented PATCH endpoint:

```rust
use maxoxide::types::BotCommand;

bot.set_my_commands(vec![
    BotCommand::new("start", "Start the bot"),
    BotCommand::named("status"),
]).await?;
```

MAX allows at most 32 commands. maxoxide validates this locally.

## Update compatibility

Known events deserialize to `Update` variants. This includes message construction requests/completions and the optional `bot_stopped.payload`. Unknown future events become `Update::Unknown` with their raw JSON retained. Unknown string enum values are likewise preserved where the model permits it.

Use `get_updates_raw` or `Dispatcher::on_raw_update` when integrating a newly introduced MAX event before a typed maxoxide release.

## Errors

All SDK operations use `maxoxide::Result<T>`:

```rust
match bot.get_me().await {
    Ok(me) => println!("{}", me.display_name()),
    Err(maxoxide::MaxError::Api(error)) => {
        eprintln!("HTTP {}, MAX code {:?}: {}", error.status, error.code, error.message);
    }
    Err(error) => eprintln!("{error}"),
}
```

`ApiError::raw_response` contains at most 4 KiB and is not written to logs by the client. It is still application data: do not forward it to public logs without reviewing your own privacy policy.

## Next steps

- Configure transport behavior in [Client configuration](client-configuration.md).
- Build messages and uploads in [Messages and media](messages-and-media.md).
- Use production webhook delivery from [Webhooks](webhooks.md).
