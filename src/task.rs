use futures::future::BoxFuture;
use std::{cmp::Ordering, fmt::Display};
use tokio::sync::oneshot;

pub(crate) struct Task<T, P: Ord> {
    index: Option<u64>,
    priority: P,
    pub(crate) key: Option<String>,
    pub(crate) job: BoxFuture<'static, T>,
    pub(crate) reply: oneshot::Sender<T>,
}

impl<T, P: Ord> Task<T, P> {
    pub fn new<J: Future<Output = T> + Send + 'static>(
        job: J,
        priority: P,
        reply: oneshot::Sender<T>,
    ) -> Self {
        Self {
            job: Box::pin(job),
            priority,
            reply,
            key: None,
            index: None,
        }
    }

    pub fn new_with_key<J: Future<Output = T> + Send + 'static>(
        job: J,
        priority: P,
        reply: oneshot::Sender<T>,
        key: impl Display,
    ) -> Self {
        Self {
            job: Box::pin(job),
            priority,
            reply,
            key: Some(key.to_string()),
            index: None,
        }
    }

    pub fn with_index(mut self, index: u64) -> Self {
        self.index = Some(index);
        self
    }
}

impl<T, P: Ord> PartialEq for Task<T, P> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T, P: Ord> Eq for Task<T, P> {}

impl<T, P: Ord> PartialOrd for Task<T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, P: Ord> Ord for Task<T, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then(other.index.cmp(&self.index))
    }
}
