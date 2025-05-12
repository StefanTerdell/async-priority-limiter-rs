use crate::{
    auto_traits::{Priority, TaskResult},
    ingress::Ingress,
    limits::Limits,
    task::Task,
    worker::Worker,
};

use futures::future::BoxFuture;
use std::{collections::BinaryHeap, fmt::Display, sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, RwLock, oneshot},
    time::Instant,
};

pub struct Limiter<T: TaskResult, P: Priority> {
    tasks: Arc<Mutex<BinaryHeap<Task<T, P>>>>,
    limits: Arc<RwLock<Limits>>,
    ingress: Ingress<T, P>,
    workers: Mutex<Vec<Worker>>,
}

impl<T: TaskResult, P: Priority> Limiter<T, P> {
    pub fn new(concurrent: usize) -> Self {
        let tasks: Arc<Mutex<BinaryHeap<Task<T, P>>>> = Default::default();
        let limits: Arc<RwLock<Limits>> = Default::default();
        let ingress = Ingress::spawn(tasks.clone());
        let workers = Mutex::new(
            (0..concurrent)
                .map(|_| ingress.spawn_worker(tasks.clone(), limits.clone()))
                .collect(),
        );

        Self {
            tasks,
            limits,
            ingress,
            workers,
        }
    }

    pub async fn get_wait_duration(&self) -> Option<Duration> {
        self.limits.read().await.get_wait_duration(None)
    }

    pub async fn get_wait_duration_by_key(&self, key: impl Display) -> Option<Duration> {
        self.limits
            .read()
            .await
            .get_wait_duration(Some(key.to_string()))
    }

    pub async fn set_wait_until_at_least(&self, instant: Instant) {
        self.limits.write().await.set_wait_until_at_least(instant);
    }

    pub async fn set_interval_at_least(&self, interval: Duration) {
        self.limits.write().await.set_interval_at_least(interval);
    }

    pub async fn set_interval_at_least_by_key(&self, interval: Duration, key: impl Display) {
        self.limits
            .write()
            .await
            .set_interval_at_least_by_key(interval, key);
    }

    pub async fn set_wait_until_at_least_by_key(&self, instant: Instant, key: impl Display) {
        self.limits
            .write()
            .await
            .set_wait_until_at_least_by_key(instant, key);
    }

    pub async fn set_wait_until(&self, instant: Option<Instant>) {
        self.limits.write().await.set_wait_until(instant);
    }

    pub async fn set_wait_until_by_key(&self, instant: Option<Instant>, key: impl Display) {
        self.limits
            .write()
            .await
            .set_wait_until_by_key(instant, key);
    }

    pub async fn set_interval(&self, interval: Option<Duration>) {
        self.limits.write().await.set_interval(interval);
    }

    pub async fn set_interval_by_key(&self, interval: Option<Duration>, key: impl Display) {
        self.limits.write().await.set_interval_by_key(interval, key);
    }

    pub async fn set_concurrent(&self, concurrent: usize) {
        let mut guard = self.workers.lock().await;
        let len = guard.len();

        match len.cmp(&concurrent) {
            std::cmp::Ordering::Less => {
                for _ in len..concurrent {
                    guard.push(
                        self.ingress
                            .spawn_worker(self.tasks.clone(), self.limits.clone()),
                    );
                }
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                guard.drain(concurrent..);
            }
        }
    }

    pub async fn queue<J: Future<Output = T> + Send + 'static>(
        &self,
        job: J,
        priority: P,
    ) -> BoxFuture<'static, T> {
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
        key: impl Display,
    ) -> BoxFuture<'static, T> {
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
        let limiter = Limiter::new(0);

        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
        enum Prio {
            Low,
            Mid,
            High,
        }

        use Prio::*;

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

        limiter.set_interval(Some(Duration::from_millis(100))).await;
        limiter.set_concurrent(2).await;

        let order = join_all(futures).await;

        assert_eq!(order, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let acc = acc.lock().await.clone();

        assert_eq!(acc, [High, High, High, Mid, Mid, Mid, Low, Low, Low]);
    }
}
