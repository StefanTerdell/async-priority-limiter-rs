#[cfg(feature = "open_ai")]
mod open_ai;

pub use self::open_ai::ReqwestResponseOpenAiHeadersExt;

use crate::{Limiter, auto_traits::Priority};

use httpdate::parse_http_date;
use reqwest::{Error, RequestBuilder, Response, header::RETRY_AFTER};
use std::{
    fmt::Display,
    time::{Duration, SystemTime},
};
use tokio::time::Instant;

type ReqwestResult = Result<Response, Error>;

pub trait ReqwestRequestBuilderExt<P: Priority> {
    fn send_limited(
        self,
        limiter: &Limiter<ReqwestResult, P>,
        priority: P,
    ) -> impl Future<Output = ReqwestResult>;

    fn send_limited_by_key(
        self,
        limiter: &Limiter<ReqwestResult, P>,
        priority: P,
        key: impl Display,
    ) -> impl Future<Output = ReqwestResult>;
}

pub trait ReqwestResponseExt<P: Priority> {
    fn update_limiter_by_retry_after_header(
        self,
        limiter: &Limiter<ReqwestResult, P>,
    ) -> impl Future<Output = Self>;

    fn update_limiter_by_key_and_retry_after_header(
        self,
        limiter: &Limiter<ReqwestResult, P>,
        key: impl Display,
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

impl<P: Priority> ReqwestResponseExt<P> for Response {
    async fn update_limiter_by_retry_after_header(
        self,
        limiter: &Limiter<ReqwestResult, P>,
    ) -> Self {
        if let Some(instant) = extract_instant_from_retry_after_header_value(&self) {
            limiter.wait_until_at_least(instant).await;
        }

        self
    }

    async fn update_limiter_by_key_and_retry_after_header(
        self,
        limiter: &Limiter<ReqwestResult, P>,
        key: impl Display,
    ) -> Self {
        if let Some(instant) = extract_instant_from_retry_after_header_value(&self) {
            limiter.wait_until_at_least_by_key(instant, key).await;
        }

        self
    }
}

impl<P: Priority> ReqwestRequestBuilderExt<P> for RequestBuilder {
    async fn send_limited(self, limiter: &Limiter<ReqwestResult, P>, priority: P) -> ReqwestResult {
        let (client, req) = self.build_split();

        let req = req.expect("Unable to extract request from builder!");

        limiter
            .queue(client.execute(req), priority)
            .await
            .expect("Sender dropped!")
    }

    async fn send_limited_by_key(
        self,
        limiter: &Limiter<ReqwestResult, P>,
        priority: P,
        key: impl Display,
    ) -> ReqwestResult {
        let (client, req) = self.build_split();

        let req = req.expect("Unable to extract request from builder!");

        limiter
            .queue_by_key(client.execute(req), priority, key)
            .await
            .expect("Sender dropped!")
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

        let limiter = Limiter::new(1);
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
