use async_throttle::RateLimiter;
use futures::future::BoxFuture;
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

#[derive(Default)]
pub struct Limits {
    pub(crate) wait_until: Option<Instant>,
    pub(crate) wait_until_key: HashMap<String, Instant>,
    pub(crate) rate_limiter: Option<RateLimiter>,
    pub(crate) rate_limiter_key: HashMap<String, RateLimiter>,
}

impl Limits {
    pub fn get_wait_duration(&self, key: Option<String>) -> Option<Duration> {
        let now = Instant::now();

        if let Some(until) = match (
            self.wait_until,
            key.and_then(|key| self.wait_until_key.get(&key)).copied(),
        ) {
            (Some(until), Some(until_key)) => {
                if until > until_key {
                    Some(until)
                } else {
                    Some(until_key)
                }
            }
            (Some(until), None) => Some(until),
            (None, Some(until_key)) => Some(until_key),
            (None, None) => None,
        } {
            if until > now {
                return Some(until - now);
            }
        }

        None
    }

    pub async fn throttle<T>(&self, job: BoxFuture<'static, T>) -> T {
        if let Some(rl) = &self.rate_limiter {
            rl.throttle(|| job).await
        } else {
            job.await
        }
    }

    pub async fn throttle_by_key<T>(
        &self,
        job: BoxFuture<'static, T>,
        key: impl Into<String>,
    ) -> T {
        let key = key.into();

        match (&self.rate_limiter, self.rate_limiter_key.get(&key)) {
            (Some(rl), Some(rl_key)) => {
                rl.throttle(|| async move { rl_key.throttle(|| job).await })
                    .await
            }
            (Some(rl), None) => rl.throttle(|| job).await,
            (None, Some(rl_key)) => rl_key.throttle(|| job).await,
            (None, None) => job.await,
        }
    }

    pub fn set_wait_until_at_least(&mut self, instant: Instant) {
        if self.wait_until.is_none_or(|curr| curr < instant) {
            self.wait_until = Some(instant);
        }
    }

    pub fn set_wait_until_at_least_by_key(&mut self, instant: Instant, key: impl Into<String>) {
        let key = key.into();

        if self
            .wait_until_key
            .get(&key)
            .is_none_or(|curr| curr < &instant)
        {
            self.wait_until_key.insert(key, instant);
        }
    }

    pub fn set_wait_until(&mut self, instant: Option<Instant>) {
        self.wait_until = instant;
    }

    pub fn set_wait_until_by_key(&mut self, instant: Option<Instant>, key: impl Into<String>) {
        let key = key.into();
        if let Some(instant) = instant {
            self.wait_until_key.insert(key, instant);
        } else {
            self.wait_until_key.remove(&key);
        }
    }

    pub fn set_rate_limit(&mut self, limit: Option<Duration>) {
        self.rate_limiter = limit.map(RateLimiter::new);
    }

    pub fn set_rate_limit_by_key(&mut self, limit: Option<Duration>, key: impl Into<String>) {
        let key = key.into();

        if let Some(limit) = limit {
            self.rate_limiter_key.insert(key, RateLimiter::new(limit));
        } else {
            self.rate_limiter_key.remove(&key);
        }
    }
}
