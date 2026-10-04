# Начало работы

## Требования

- Rust 1.85 или новее.
- Бот, созданный на платформе MAX для партнёров, и его Bot API token.
- HTTPS-инфраструктура для Webhook в production. MAX описывает Long Polling как транспорт для разработки и тестирования.

Добавьте crate и Tokio:

```toml
[dependencies]
maxoxide = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "signal"] }
```

Не храните токен в исходном коде:

```bash
export MAX_BOT_TOKEN='...'
cargo run --example quickstart_bot
```

`Bot::from_env()` и остальные конструкторы возвращают `Result`. Пустой токен, недопустимый HTTP-заголовок, небезопасный base URL, конфликт custom client/proxy и нулевой timeout обнаруживаются до первого запроса.

## Первый Dispatcher

```rust
use maxoxide::types::Update;
use maxoxide::{Bot, Context, Dispatcher, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut dispatcher = Dispatcher::new(Bot::from_env()?);
    dispatcher.on_command("/start", |context: Context| async move {
        if let Update::MessageCreated { message, .. } = &context.update {
            context.bot.send_text_to_chat(message.chat_id(), "Готово").await?;
        }
        Ok(())
    });
    dispatcher.start_polling().await
}
```

Команды сравниваются по точному первому токену. `/start`, `/start one` и `/start@your_bot one` соответствуют `/start`, а `/starter` — нет. Остаток строки доступен через `Context::command_arguments()`.

## `chat_id`, `user_id` и `post_id`

- `chat_id` обозначает конкретный личный диалог, группу или канал. Для ответа на update используйте `send_*_to_chat`.
- `user_id` — стабильный глобальный ID пользователя MAX. Используйте `send_*_to_user`, если ID диалога неизвестен.
- `Recipient::post_id` обозначает пост канала, когда MAX передаёт контекст поста, в частности для комментариев.

Не подменяйте один идентификатор другим. Личное сообщение обычно одновременно содержит `message.sender.user_id` и `message.recipient.chat_id`.

## Информация о боте и команды

`get_me()` возвращает специальный `BotInfo`, а не обычного пользователя:

```rust
let me = bot.get_me().await?;
println!("{} ({})", me.display_name(), me.user_id);
```

Полностью заменить меню команд можно документированным PATCH-методом:

```rust
use maxoxide::types::BotCommand;

bot.set_my_commands(vec![
    BotCommand::new("start", "Запустить бота"),
    BotCommand::named("status"),
]).await?;
```

MAX разрешает не более 32 команд. maxoxide проверяет ограничение локально.

## Совместимость событий

Известные события преобразуются в варианты `Update`, включая создание, изменение и удаление комментария, смену прав администратора бота, запрос и завершение конструктора сообщения, а также необязательный `bot_stopped.payload`. Будущие неизвестные события и некорректные известные payload преобразуются в `Update::Unknown` с сохранённым исходным JSON. Неизвестные строковые значения enum также сохраняются там, где это позволяет модель.

Для только что добавленного события MAX до выхода типизированной версии maxoxide используйте `get_updates_raw` или `Dispatcher::on_raw_update`.

## Ошибки

Все операции SDK используют `maxoxide::Result<T>`:

```rust
match bot.get_me().await {
    Ok(me) => println!("{}", me.display_name()),
    Err(maxoxide::MaxError::Api(error)) => {
        eprintln!("HTTP {}, код MAX {:?}: {}", error.status, error.code, error.message);
    }
    Err(error) => eprintln!("{error}"),
}
```

`ApiError::raw_response` содержит не более 4 КиБ и не записывается клиентом в лог. Это всё равно данные приложения: не отправляйте поле в публичные логи без учёта своей privacy policy.

## Дальше

- Транспортные параметры описаны в [Настройке клиента](client-configuration.md).
- Сообщения и upload — в [Сообщениях и медиа](messages-and-media.md).
- Production-доставка событий — в [Webhook](webhooks.md).
