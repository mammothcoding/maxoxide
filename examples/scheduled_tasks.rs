//! # English
//!
//! Runs a periodic task together with long polling. Every 60 seconds the task
//! calls `get_me` as a simple health check, and it stops automatically when the
//! dispatcher shuts down. Use this pattern for lightweight cleanup, cache refresh,
//! reminders, or health checks that belong to the bot process. Long or critical
//! jobs are usually better handled by a dedicated job queue.
//!
//! Run: `MAX_BOT_TOKEN=... cargo run --example scheduled_tasks`.
//!
//! # Русский
//!
//! Запускает periodic task вместе с long polling. Каждые 60 секунд task вызывает
//! `get_me` как простой health check и автоматически останавливается вместе с
//! Dispatcher. Шаблон подходит для лёгкого cleanup, обновления cache, напоминаний
//! и проверок состояния внутри процесса бота. Долгие и критичные jobs лучше
//! переносить в отдельную очередь задач.
//!
//! Запуск: `MAX_BOT_TOKEN=... cargo run --example scheduled_tasks`.

use std::time::Duration;

use maxoxide::{Bot, Dispatcher, Result, ScheduledTaskContext};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut dispatcher = Dispatcher::new(Bot::from_env()?);
    dispatcher.task(
        Duration::from_secs(60),
        |context: ScheduledTaskContext| async move {
            let me = context.bot.get_me().await?;
            tracing::info!(bot_id = me.user_id, "periodic health check");
            Ok(())
        },
    );
    dispatcher.start_polling().await
}
