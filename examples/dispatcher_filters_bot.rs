//! Demonstrates composable Dispatcher filters, startup hooks, raw update hooks,
//! and scheduled tasks.
//!
//! Run:
//!   MAX_BOT_TOKEN=your_token cargo run --example dispatcher_filters_bot

use std::time::Duration;

use maxoxide::types::{AttachmentKind, Update};
use maxoxide::{
    Bot, Context, Dispatcher, Filter, RawUpdateContext, Result, ScheduledTaskContext, StartContext,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let bot = Bot::from_env();
    let mut dp = Dispatcher::new(bot);

    dp.on_start(|ctx: StartContext| async move {
        let me = ctx.bot.get_me().await?;
        tracing::info!(
            user_id = me.user_id,
            name = %me.display_name(),
            "bot polling started"
        );
        Ok(())
    });

    dp.on_raw_update(|ctx: RawUpdateContext| async move {
        let update_type = ctx
            .raw
            .get("update_type")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        tracing::debug!(update_type, "raw update received");
        Ok(())
    });

    dp.task(
        Duration::from_secs(300),
        |ctx: ScheduledTaskContext| async move {
            let me = ctx.bot.get_me().await?;
            tracing::debug!(user_id = me.user_id, "scheduled health check");
            Ok(())
        },
    );

    dp.on_update(
        Filter::message() & Filter::text_contains("ping"),
        |ctx: Context| async move {
            if let Update::MessageCreated { message, .. } = &ctx.update {
                ctx.bot.send_text_to_chat(message.chat_id(), "pong").await?;
            }
            Ok(())
        },
    );

    dp.on_update(
        Filter::message() & Filter::has_media(),
        |ctx: Context| async move {
            if let Update::MessageCreated { message, .. } = &ctx.update {
                ctx.bot
                    .send_text_to_chat(message.chat_id(), "media attachment received")
                    .await?;
            }
            Ok(())
        },
    );

    dp.on_update(
        Filter::message() & Filter::has_attachment_type(AttachmentKind::File),
        |ctx: Context| async move {
            if let Update::MessageCreated { message, .. } = &ctx.update {
                ctx.bot
                    .send_text_to_chat(message.chat_id(), "file attachment received")
                    .await?;
            }
            Ok(())
        },
    );

    let issue_filter = Filter::message() & Filter::text_regex(r"(?i)\b(issue|bug|error)\b")?;
    dp.on_update(issue_filter, |ctx: Context| async move {
        if let Update::MessageCreated { message, .. } = &ctx.update {
            ctx.bot
                .send_text_to_chat(message.chat_id(), "I noticed an issue report")
                .await?;
        }
        Ok(())
    });

    dp.on_update(Filter::unknown_update(), |ctx: Context| async move {
        tracing::warn!(
            update_type = ctx.update.update_type().unwrap_or("unknown"),
            "unsupported update type"
        );
        Ok(())
    });

    dp.start_polling().await;
    Ok(())
}
