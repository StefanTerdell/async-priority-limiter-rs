use std::sync::Arc;

use reqwest::RequestBuilder;
use reqwest_eventsource::{CannotCloneRequestError, EventSource};

use crate::{
    Limiter,
    traits::{Key, Priority},
};

use super::{ReqwestRequestBuilderExt, ReqwestResult};

pub type EventSourceResult = Result<EventSource, CannotCloneRequestError>;

pub trait ReqwestRequestBuilderEventsourceExt<K: Key, P: Priority> {
    fn eventsource_limited(
        self,
        limiter: Arc<Limiter<K, P, ReqwestResult>>,
        priority: P,
    ) -> EventSourceResult;

    fn eventsource_limited_by_key(
        self,
        limiter: Arc<Limiter<K, P, ReqwestResult>>,
        priority: P,
        key: K,
    ) -> EventSourceResult;
}

impl<K: Key, P: Priority> ReqwestRequestBuilderEventsourceExt<K, P> for RequestBuilder {
    fn eventsource_limited(
        self,
        limiter: Arc<Limiter<K, P, ReqwestResult>>,
        priority: P,
    ) -> EventSourceResult {
        EventSource::new_by_send_fn(self, |builder| {
            builder.send_limited(limiter.clone(), priority.clone())
        })
    }
    fn eventsource_limited_by_key(
        self,
        limiter: Arc<Limiter<K, P, ReqwestResult>>,
        priority: P,
        key: K,
    ) -> EventSourceResult {
        EventSource::new_by_send_fn(self, |builder| {
            builder.send_limited_by_key(limiter.clone(), priority.clone(), key.clone())
        })
    }
}
