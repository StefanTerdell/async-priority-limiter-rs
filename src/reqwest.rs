#[cfg(feature = "open_ai")]
mod open_ai;
#[cfg(feature = "open_ai")]
pub use self::open_ai::ReqwestResponseOpenAiHeadersExt;

use crate::{
    Limiter,
    traits::{Key, Priority},
};

use httpdate::parse_http_date;
use reqwest::{Error, RequestBuilder, Response, header::RETRY_AFTER};
use std::time::{Duration, SystemTime};
use tokio::time::Instant;

pub type ReqwestResult = Result<Response, Error>;

pub trait ReqwestRequestBuilderExt<K: Key, P: Priority> {
    fn send_limited(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        priority: P,
    ) -> impl Future<Output = ReqwestResult>;

    fn send_limited_by_key(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        priority: P,
        key: K,
    ) -> impl Future<Output = ReqwestResult>;
}

pub trait ReqwestResponseExt<K: Key, P: Priority> {
    fn update_limiter_by_retry_after_header(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
    ) -> impl Future<Output = Self>;

    fn update_limiter_by_key_and_retry_after_header(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        key: K,
    ) -> impl Future<Output = Self>;
}

fn extract_instant_from_retry_after_header_value(response: &Response) -> Option<Instant> {
    if let Some(value) = response
        .headers()
        .get(RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
    {
        if let Ok(seconds) = value.parse::<u64>() {
            return Some(Instant::now() + Duration::from_secs(seconds));
        } else if let Ok(http_date) = parse_http_date(value) {
            if let Ok(duration) = http_date.duration_since(SystemTime::now()) {
                return Some(Instant::now() + duration);
            }
        }
    }

    None
}

impl<K: Key, P: Priority> ReqwestResponseExt<K, P> for Response {
    async fn update_limiter_by_retry_after_header(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
    ) -> Self {
        (&self).update_limiter_by_retry_after_header(limiter).await;
        self
    }

    async fn update_limiter_by_key_and_retry_after_header(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        key: K,
    ) -> Self {
        (&self)
            .update_limiter_by_key_and_retry_after_header(limiter, key)
            .await;
        self
    }
}

impl<K: Key, P: Priority> ReqwestResponseExt<K, P> for &Response {
    async fn update_limiter_by_retry_after_header(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
    ) -> Self {
        if let Some(instant) = extract_instant_from_retry_after_header_value(self) {
            limiter
                .as_ref()
                .set_default_block_until_at_least(instant)
                .await;
        }

        self
    }

    async fn update_limiter_by_key_and_retry_after_header(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        key: K,
    ) -> Self {
        if let Some(instant) = extract_instant_from_retry_after_header_value(self) {
            limiter
                .as_ref()
                .set_block_by_key_until_at_least(instant, key)
                .await;
        }

        self
    }
}

impl<K: Key, P: Priority> ReqwestRequestBuilderExt<K, P> for RequestBuilder {
    async fn send_limited(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        priority: P,
    ) -> ReqwestResult {
        let (client, req) = self.build_split();

        let req = req.expect("Unable to extract request from builder!");
        let res = limiter.as_ref().queue(client.execute(req), priority).await;

        res.await
    }

    async fn send_limited_by_key(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        priority: P,
        key: K,
    ) -> ReqwestResult {
        let (client, req) = self.build_split();

        let req = req.expect("Unable to extract request from builder!");
        let res = limiter
            .as_ref()
            .queue_by_key(client.execute(req), priority, key)
            .await;

        res.await
    }
}

#[cfg(test)]
mod tests {
    use super::ReqwestResponseExt;
    use crate::{Limiter, reqwest::ReqwestRequestBuilderExt};
    use reqwest::{Client, StatusCode};
    use std::time::Duration;
    use tokio::time::Instant;

    #[tokio::test]
    async fn it_should_work() {
        let mut server = mockito::Server::new_async().await;

        server
            .mock("GET", "/")
            .match_header("res", "429")
            .with_status(429)
            .with_header("Retry-After", "1")
            .create();

        server
            .mock("GET", "/")
            .match_header("res", "200")
            .with_status(200)
            .create();

        let limiter = Limiter::new::<String>(1);
        let before = Instant::now();
        let client = Client::new();

        let first = client
            .get(server.url())
            .header("res", "429")
            .send_limited(&limiter, 1)
            .await
            .unwrap()
            .update_limiter_by_retry_after_header(&limiter)
            .await
            .status();

        let second = client
            .get(server.url())
            .header("res", "200")
            .send_limited(&limiter, 1)
            .await
            .unwrap()
            .update_limiter_by_retry_after_header(&limiter)
            .await
            .status();

        let duration = Instant::now() - before;

        assert_eq!(first, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(second, StatusCode::OK);

        assert!(duration > Duration::from_secs(1));
    }
}
