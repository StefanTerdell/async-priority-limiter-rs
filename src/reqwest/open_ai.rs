use reqwest::Response;
use std::time::Duration;
use tokio::time::Instant;

use super::{ReqwestResponseExt, ReqwestResult};
use crate::{
    Limiter,
    traits::{Key, Priority},
};

pub trait ReqwestResponseOpenAiHeadersExt<K: Key, P: Priority> {
    fn update_limiter_by_open_ai_ratelimit_headers(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
    ) -> impl Future<Output = Self>;

    fn update_limiter_by_key_and_open_ai_ratelimit_headers(
        self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        key: K,
    ) -> impl Future<Output = Self>;
}

struct OpenAiRateLimit {
    #[allow(unused)]
    limit_type: OpenAiRateLimitType,
    #[allow(unused)]
    max: usize,
    remaining: isize,
    reset: Duration,
}

enum OpenAiRateLimitType {
    Tokens,
    Requests,
}

const X_RATELIMIT_LIMIT_REQUESTS: &str = "x-ratelimit-limit-requests";
const X_RATELIMIT_REMAINING_REQUESTS: &str = "x-ratelimit-remaining-requests";
const X_RATELIMIT_RESET_REQUESTS: &str = "x-ratelimit-reset-requests";
const X_RATELIMIT_LIMIT_TOKENS: &str = "x-ratelimit-limit-tokens";
const X_RATELIMIT_REMAINING_TOKENS: &str = "x-ratelimit-remaining-tokens";
const X_RATELIMIT_RESET_TOKENS: &str = "x-ratelimit-reset-tokens";

impl OpenAiRateLimit {
    fn from_response_headers_by_type(
        response: &Response,
        limit_type: OpenAiRateLimitType,
    ) -> Option<Self> {
        let (max_key, remaining_key, reset_key) = match &limit_type {
            OpenAiRateLimitType::Requests => (
                X_RATELIMIT_LIMIT_REQUESTS,
                X_RATELIMIT_REMAINING_REQUESTS,
                X_RATELIMIT_RESET_REQUESTS,
            ),
            OpenAiRateLimitType::Tokens => (
                X_RATELIMIT_LIMIT_TOKENS,
                X_RATELIMIT_REMAINING_TOKENS,
                X_RATELIMIT_RESET_TOKENS,
            ),
        };

        Some(Self {
            limit_type,
            remaining: response
                .headers()
                .get(remaining_key)
                .and_then(|value| value.to_str().ok())
                .and_then(|remaining| remaining.parse().ok())?,
            max: response
                .headers()
                .get(max_key)
                .and_then(|value| value.to_str().ok())
                .and_then(|max| max.parse().ok())?,
            reset: response
                .headers()
                .get(reset_key)
                .and_then(|value| value.to_str().ok())
                .and_then(|max| humantime::parse_duration(max).ok())?,
        })
    }

    fn wait_until_instant(self) -> Option<Instant> {
        if self.remaining <= 0 {
            Some(Instant::now() + self.reset)
        } else {
            None
        }
    }
}

fn extract_max_wait_until_instant_from_headers(response: &Response) -> Option<Instant> {
    let no_requests_until =
        OpenAiRateLimit::from_response_headers_by_type(response, OpenAiRateLimitType::Requests)
            .and_then(OpenAiRateLimit::wait_until_instant);

    let no_tokens_until =
        OpenAiRateLimit::from_response_headers_by_type(response, OpenAiRateLimitType::Tokens)
            .and_then(OpenAiRateLimit::wait_until_instant);

    no_requests_until.max(no_tokens_until)
}

impl<K: Key, P: Priority> ReqwestResponseOpenAiHeadersExt<K, P> for Response {
    async fn update_limiter_by_open_ai_ratelimit_headers(
        mut self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
    ) -> Self {
        let limiter = limiter.as_ref();

        self = self.update_limiter_by_retry_after_header(limiter).await;

        if let Some(instant) = extract_max_wait_until_instant_from_headers(&self) {
            limiter.set_default_block_until_at_least(instant).await;
        }

        self
    }

    async fn update_limiter_by_key_and_open_ai_ratelimit_headers(
        mut self,
        limiter: impl AsRef<Limiter<K, P, ReqwestResult>>,
        key: K,
    ) -> Self {
        let limiter = limiter.as_ref();

        self = self
            .update_limiter_by_key_and_retry_after_header(limiter, key.clone())
            .await;

        if let Some(instant) = extract_max_wait_until_instant_from_headers(&self) {
            limiter.set_block_by_key_until_at_least(instant, key).await;
        }

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .with_header(X_RATELIMIT_LIMIT_REQUESTS, "200")
            .with_header(X_RATELIMIT_REMAINING_REQUESTS, "0")
            .with_header(X_RATELIMIT_RESET_REQUESTS, "1h2m3s")
            .with_header(X_RATELIMIT_LIMIT_TOKENS, "200")
            .with_header(X_RATELIMIT_REMAINING_TOKENS, "0")
            .with_header(X_RATELIMIT_RESET_TOKENS, "3h2m1s")
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
            .update_limiter_by_open_ai_ratelimit_headers(&limiter)
            .await
            .status();

        let second = client
            .get(server.url())
            .header("res", "200")
            .send_limited(&limiter, 1)
            .await
            .unwrap()
            .update_limiter_by_open_ai_ratelimit_headers(&limiter)
            .await
            .status();

        let test_duration = Instant::now() - before;
        let wait_duration = limiter.get_default_block_duration().await;

        assert_eq!(first, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(second, StatusCode::OK);

        assert!(test_duration > Duration::from_secs(1));

        assert!(wait_duration.is_some());
        assert!(wait_duration.is_some_and(|duration| duration
            > Duration::from_secs((60 * 60 * 3) + (60 * 2))
            && duration < Duration::from_secs((60 * 60 * 3) + (60 * 2) + 2)))
    }
}
