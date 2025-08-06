use crate::{BoxFuture, traits::Key};

use std::{cmp::Ordering, fmt::Debug};
use tokio::sync::oneshot;

pub(crate) enum Job<T> {
    Some {
        job: BoxFuture<T>,
        reply: oneshot::Sender<T>,
    },
    None {
        reply: oneshot::Sender<()>,
    },
}

pub(crate) struct Task<K: Key, P: Ord, T> {
    pub(crate) index: Option<u64>,
    pub(crate) priority: P,
    pub(crate) key: Option<K>,
    pub(crate) job: Job<T>,
}

impl<T, P: Ord + Debug, K: Key + Debug> Debug for Task<K, P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task<T, P>")
            .field("index", &self.index)
            .field("priority", &self.priority)
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl<T, P: Ord, K: Key> Task<K, P, T> {
    pub fn new<J: Future<Output = T> + Send + 'static>(
        job: J,
        priority: P,
        reply: oneshot::Sender<T>,
    ) -> Self {
        Self {
            job: Job::Some {
                job: Box::pin(job),
                reply,
            },
            priority,
            key: None,
            index: None,
        }
    }

    pub fn new_empty(priority: P, reply: oneshot::Sender<()>) -> Self {
        Self {
            job: Job::None { reply },
            priority,
            key: None,
            index: None,
        }
    }

    pub fn new_with_key<J: Future<Output = T> + Send + 'static>(
        job: J,
        priority: P,
        reply: oneshot::Sender<T>,
        key: K,
    ) -> Self {
        Self {
            job: Job::Some {
                job: Box::pin(job),
                reply,
            },
            priority,
            key: Some(key),
            index: None,
        }
    }

    pub fn new_empty_with_key(priority: P, reply: oneshot::Sender<()>, key: K) -> Self {
        Self {
            job: Job::None { reply },
            priority,
            key: Some(key),
            index: None,
        }
    }

    pub fn with_index(mut self, index: u64) -> Self {
        self.index = Some(index);
        self
    }
}

impl<T, P: Ord, K: Key> PartialEq for Task<K, P, T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T, P: Ord, K: Key> Eq for Task<K, P, T> {}

impl<T, P: Ord, K: Key> PartialOrd for Task<K, P, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, P: Ord, K: Key> Ord for Task<K, P, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then(other.index.cmp(&self.index))
    }
}
