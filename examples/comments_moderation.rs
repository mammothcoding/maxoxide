//! # English
//!
//! Shows the typed request used to read up to 50 comments for a channel post and
//! iterate over their IDs and text. This is a preparation and mock-integration
//! example for future moderation tools; MAX currently marks all comments endpoints
//! temporarily unavailable, so a real request is expected to fail until the
//! platform enables them. Do not build a production workflow that depends on it.
//!
//! Requires `MAX_BOT_TOKEN` and the channel post ID in `MAX_POST_ID`.
//! Run: `MAX_BOT_TOKEN=... MAX_POST_ID=... cargo run --example comments_moderation`.
//!
//! # Русский
//!
//! Показывает typed request для чтения до 50 комментариев к посту канала и обхода
//! их ID и текста. Это подготовительный пример для mock-интеграции и будущих tools
//! модерации: MAX сейчас помечает все endpoints комментариев временно недоступными,
//! поэтому реальный запрос ожидаемо завершится ошибкой до включения API платформой.
//! Не стройте production-процесс, зависящий от его доступности.
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
