use async_throttle::RateLimiter;
use futures::future::BoxFuture;
use std::{collections::HashMap, fmt::Display, time::Duration};
use tokio::time::Instant;

#[derive(Default)]
pub struct Limits {
    pub(crate) wait_until: Option<Instant>,
    pub(crate) wait_until_key: HashMap<String, Instant>,
    pub(crate) rate_limiter: Option<(Duration, RateLimiter)>,
    pub(crate) rate_limiter_key: HashMap<String, (Duration, RateLimiter)>,
}

impl Limits {
    pub fn get_wait_duration(&self, key: Option<String>) -> Option<Duration> {
        self.wait_until
            .max(key.and_then(|key| self.wait_until_key.get(&key.to_string()).copied()))
            .map(|duration| duration.duration_since(Instant::now()))
    }

    pub async fn throttle<T>(&self, job: BoxFuture<'static, T>) -> T {
        if let Some((_, rl)) = &self.rate_limiter {
            rl.throttle(|| job).await
        } else {
            job.await
        }
    }

    pub async fn throttle_by_key<T>(&self, job: BoxFuture<'static, T>, key: impl Display) -> T {
        let key = key.to_string();

        match (&self.rate_limiter, self.rate_limiter_key.get(&key)) {
            (Some((_, rl)), Some((_, rl_key))) => {
                rl.throttle(|| async move { rl_key.throttle(|| job).await })
                    .await
            }
            (Some((_, rl)), None) => rl.throttle(|| job).await,
            (None, Some((_, rl_key))) => rl_key.throttle(|| job).await,
            (None, None) => job.await,
        }
    }

    pub fn set_wait_until_at_least(&mut self, instant: Instant) {
        if self.wait_until.is_none_or(|curr| curr < instant) {
            self.wait_until = Some(instant);
        }
    }

    pub fn set_wait_until_at_least_by_key(&mut self, instant: Instant, key: impl Display) {
        let key = key.to_string();

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

    pub fn set_wait_until_by_key(&mut self, instant: Option<Instant>, key: impl Display) {
        let key = key.to_string();

        if let Some(instant) = instant {
            self.wait_until_key.insert(key, instant);
        } else {
            self.wait_until_key.remove(&key);
        }
    }

    pub fn set_interval_at_least(&mut self, interval: Duration) {
        if self
            .rate_limiter
            .as_ref()
            .is_none_or(|(prev_limit, _)| prev_limit > &interval)
        {
            self.rate_limiter = Some((interval, RateLimiter::new(interval)))
        }
    }

    pub fn set_interval_at_least_by_key(&mut self, interval: Duration, key: impl Display) {
        let key = key.to_string();

        if self
            .rate_limiter_key
            .get(&key)
            .as_ref()
            .is_none_or(|(prev_limit, _)| prev_limit > &interval)
        {
            self.rate_limiter_key
                .insert(key, (interval, RateLimiter::new(interval)));
        }
    }

    pub fn set_interval(&mut self, interval: Option<Duration>) {
        self.rate_limiter = interval.map(|interval| (interval, RateLimiter::new(interval)));
    }

    pub fn set_interval_by_key(&mut self, interval: Option<Duration>, key: impl Display) {
        let key = key.to_string();

        if let Some(interval) = interval {
            self.rate_limiter_key
                .insert(key, (interval, RateLimiter::new(interval)));
        } else {
            self.rate_limiter_key.remove(&key);
        }
    }
}
