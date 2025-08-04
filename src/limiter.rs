use crate::{
    BoxFuture,
    blocks::Blocks,
    ingress::Ingress,
    intervals::Intervals,
    task::Task,
    traits::{Key, Priority, TaskResult},
    worker::Worker,
};

use std::{collections::BinaryHeap, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, RwLock, oneshot},
    time::Instant,
};

pub struct Limiter<K: Key, P: Priority, T: TaskResult> {
    tasks: Arc<Mutex<BinaryHeap<Task<K, P, T>>>>,
    ingress: Ingress<K, P, T>,
    workers: Mutex<Vec<Worker>>,
    blocks: Arc<RwLock<Blocks<K>>>,
    intervals: Arc<RwLock<Intervals<K>>>,
}

impl<P: Priority, T: TaskResult> Default for Limiter<String, P, T> {
    fn default() -> Self {
        Self::new(1)
    }
}

impl<P: Priority, T: TaskResult> Limiter<String, P, T> {
    pub fn new<K: Key>(concurrent: usize) -> Limiter<K, P, T> {
        let tasks: Arc<Mutex<BinaryHeap<Task<K, P, T>>>> = Default::default();
        let blocks: Arc<RwLock<Blocks<K>>> = Default::default();
        let intervals: Arc<RwLock<Intervals<K>>> = Default::default();
        let ingress = Ingress::spawn(tasks.clone());
        let workers = Mutex::new(
            (0..concurrent)
                .map(|_| ingress.spawn_worker(tasks.clone(), blocks.clone(), intervals.clone()))
                .collect(),
        );

        Limiter {
            tasks,
            blocks,
            intervals,
            ingress,
            workers,
        }
    }
}

impl<K: Key, P: Priority, T: TaskResult> Limiter<K, P, T> {
    pub async fn get_default_block_duration(&self) -> Option<Duration> {
        self.blocks.write().await.get_default()
    }

    pub async fn get_block_duration_by_key(&self, key: &K) -> Option<Duration> {
        self.blocks.write().await.get_by_key(key)
    }

    pub async fn set_default_block_until_at_least(&self, instant: Instant) {
        self.blocks.write().await.set_default_at_least(instant);
    }

    pub async fn set_block_by_key_until_at_least(&self, instant: Instant, key: K) {
        self.blocks.write().await.set_at_least_by_key(instant, key);
    }

    pub async fn set_default_block_until(&self, instant: Option<Instant>) {
        self.blocks.write().await.set_default(instant);
    }

    pub async fn set_block_by_key_until(&self, instant: Option<Instant>, key: K) {
        self.blocks.write().await.set_by_key(instant, key);
    }

    pub async fn set_default_interval_at_least(&self, interval: Duration) {
        self.intervals.write().await.set_default_at_least(interval);
    }

    pub async fn set_interval_by_key_at_least(&self, interval: Duration, key: K) {
        self.intervals
            .write()
            .await
            .set_at_least_by_key(interval, key);
    }

    pub async fn set_default_interval(&self, interval: Option<Duration>) {
        self.intervals.write().await.set_default(interval);
    }

    pub async fn set_interval_by_key(&self, interval: Option<Duration>, key: K) {
        self.intervals.write().await.set_by_key(interval, key);
    }

    pub async fn set_concurrent_tasks(&self, concurrent_tasks: usize) {
        let mut guard = self.workers.lock().await;
        let len = guard.len();

        match len.cmp(&concurrent_tasks) {
            std::cmp::Ordering::Less => {
                for _ in len..concurrent_tasks {
                    guard.push(self.ingress.spawn_worker(
                        self.tasks.clone(),
                        self.blocks.clone(),
                        self.intervals.clone(),
                    ));
                }
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                guard.drain(concurrent_tasks..);
            }
        }
    }

    pub async fn queue<J: Future<Output = T> + Send + 'static>(
        &self,
        job: J,
        priority: P,
    ) -> BoxFuture<T> {
        let (reply_sender, reply_receiver) = oneshot::channel();

        self.ingress
            .send(Task::new(job, priority, reply_sender))
            .await;

        Box::pin(async move { reply_receiver.await.expect("reply_sender should not drop") })
    }

    pub async fn queue_by_key<J: Future<Output = T> + Send + 'static>(
        &self,
        job: J,
        priority: P,
        key: K,
    ) -> BoxFuture<T> {
        let (reply_sender, reply_receiver) = oneshot::channel();

        self.ingress
            .send(Task::new_with_key(job, priority, reply_sender, key))
            .await;

        Box::pin(async move { reply_receiver.await.expect("reply_sender should not drop") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::join_all;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn it_should_work() {
        use Prio::*;

        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
        enum Prio {
            Low,
            Mid,
            High,
        }

        let limiter = Limiter::new::<String>(0);

        let acc: Arc<Mutex<Vec<Prio>>> = Default::default();

        let futures = [
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(High);
                            1
                        }
                    },
                    High,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(Mid);
                            2
                        }
                    },
                    Mid,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(Low);
                            3
                        }
                    },
                    Low,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(Low);
                            4
                        }
                    },
                    Low,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(Mid);
                            5
                        }
                    },
                    Mid,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(High);
                            6
                        }
                    },
                    High,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(Mid);
                            7
                        }
                    },
                    Mid,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(Low);
                            8
                        }
                    },
                    Low,
                )
                .await,
            limiter
                .queue(
                    {
                        let results = acc.clone();
                        async move {
                            results.lock().await.push(High);
                            9
                        }
                    },
                    High,
                )
                .await,
        ];

        limiter
            .set_default_interval(Some(Duration::from_millis(100)))
            .await;

        limiter.set_concurrent_tasks(2).await;

        let order = join_all(futures).await;
        let acc = acc.lock().await.clone();

        assert_eq!(order, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        assert_eq!(acc, [High, High, High, Mid, Mid, Mid, Low, Low, Low]);
    }
}
