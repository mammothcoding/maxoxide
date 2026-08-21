use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

type RecipientLimiterMap = HashMap<(MessageOperation, RateLimitKey), (Arc<SlidingWindow>, Instant)>;

/// Documented MAX request limits used by a [`crate::Bot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitConfig {
    /// Maximum requests to `platform-api2.max.ru` in a rolling second.
    /// `None` disables this limiter.
    pub global_requests_per_second: Option<u32>,
    /// Maximum message operations in a rolling second for one recipient.
    /// `None` disables this limiter.
    pub message_operations_per_second: Option<u32>,
}

impl RateLimitConfig {
    /// Disables all local rate limiting.
    ///
    /// This is primarily useful for deterministic tests and custom gateways
    /// that implement their own limits.
    pub const fn disabled() -> Self {
        Self {
            global_requests_per_second: None,
            message_operations_per_second: None,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_requests_per_second: Some(30),
            message_operations_per_second: Some(2),
        }
    }
}

/// Recipient key used for per-dialog message operation limits.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    /// A concrete dialog, group, or channel ID.
    Chat(i64),
    /// A global MAX user ID used by a user-addressed send method.
    User(i64),
    /// An application-defined key for operations where MAX only exposes a message ID.
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MessageOperation {
    Send,
    Edit,
    Delete,
    Callback,
    Comment,
}

#[derive(Debug)]
struct SlidingWindow {
    events: Mutex<VecDeque<Instant>>,
}

impl SlidingWindow {
    fn new() -> Self {
        Self {
            events: Mutex::new(VecDeque::new()),
        }
    }

    async fn acquire(&self, limit: Option<u32>) {
        let Some(limit) = limit.filter(|limit| *limit > 0) else {
            return;
        };
        let window = Duration::from_secs(1);

        loop {
            let wait = {
                let now = Instant::now();
                let mut events = self.events.lock().await;
                while events
                    .front()
                    .is_some_and(|instant| now.duration_since(*instant) >= window)
                {
                    events.pop_front();
                }

                if events.len() < limit as usize {
                    events.push_back(now);
                    None
                } else {
                    events
                        .front()
                        .map(|instant| (*instant + window).saturating_duration_since(now))
                }
            };

            match wait {
                Some(duration) if !duration.is_zero() => tokio::time::sleep(duration).await,
                Some(_) => tokio::task::yield_now().await,
                None => return,
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct RateLimiters {
    config: RateLimitConfig,
    global: SlidingWindow,
    recipients: Mutex<RecipientLimiterMap>,
}

impl RateLimiters {
    pub(crate) fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            global: SlidingWindow::new(),
            recipients: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn acquire_global(&self) {
        self.global
            .acquire(self.config.global_requests_per_second)
            .await;
    }

    pub(crate) async fn acquire_recipient(&self, operation: MessageOperation, key: RateLimitKey) {
        if self.config.message_operations_per_second.is_none() {
            return;
        }

        let limiter = {
            let mut recipients = self.recipients.lock().await;
            let now = Instant::now();
            if recipients.len() >= 8192 {
                recipients.retain(|_, (_, touched)| {
                    now.duration_since(*touched) < Duration::from_secs(60)
                });
                if recipients.len() >= 8192 {
                    if let Some(oldest) = recipients
                        .iter()
                        .min_by_key(|(_, (_, touched))| *touched)
                        .map(|(key, _)| key.clone())
                    {
                        recipients.remove(&oldest);
                    }
                }
            }
            let (limiter, touched) = recipients
                .entry((operation, key))
                .or_insert_with(|| (Arc::new(SlidingWindow::new()), now));
            *touched = now;
            limiter.clone()
        };
        limiter
            .acquire(self.config.message_operations_per_second)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::SlidingWindow;

    #[tokio::test(start_paused = true)]
    async fn rolling_window_waits_after_limit() {
        let limiter = SlidingWindow::new();
        limiter.acquire(Some(2)).await;
        limiter.acquire(Some(2)).await;

        let started = tokio::time::Instant::now();
        limiter.acquire(Some(2)).await;

        assert_eq!(started.elapsed(), std::time::Duration::from_secs(1));
    }
}
