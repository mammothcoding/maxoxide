//! # English
//!
//! Shows the typed request used to read up to 50 comments for a channel post and
//! iterate over their IDs and text. The bot must have access to the channel and the
//! administrator permissions required by MAX.
//!
//! Requires `MAX_BOT_TOKEN` and the channel post ID in `MAX_POST_ID`.
//! Run: `MAX_BOT_TOKEN=... MAX_POST_ID=... cargo run --example comments_moderation`.
//!
//! # Русский
//!
//! Показывает типизированный запрос для чтения до 50 комментариев к посту канала и
//! обхода их ID и текста. Боту нужны доступ к каналу и требуемые MAX права
//! администратора.
//!
//! Нужны `MAX_BOT_TOKEN` и ID поста канала в `MAX_POST_ID`.
//! Запуск: `MAX_BOT_TOKEN=... MAX_POST_ID=... cargo run --example comments_moderation`.

use maxoxide::types::GetCommentsOptions;
use maxoxide::{Bot, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let post_id = std::env::var("MAX_POST_ID").expect("MAX_POST_ID is required");
    let comments = Bot::from_env()?
        .get_comments(
            &post_id,
            GetCommentsOptions {
                count: Some(50),
                ..Default::default()
            },
        )
        .await?;
    for comment in comments.messages {
        println!(
            "{}: {}",
            comment.body.mid,
            comment.body.text.unwrap_or_default()
        );
    }
    Ok(())
}
