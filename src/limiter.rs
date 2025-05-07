use crate::{
    auto_traits::{Priority, TaskResult},
    ingress::Ingress,
    task::Task,
    tasks_and_limits::TasksAndLimits,
    worker::Worker,
};

use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, oneshot},
    time::Instant,
};

pub struct Limiter<T: TaskResult, P: Priority> {
    tasks_and_limits: Arc<TasksAndLimits<T, P>>,
    ingress: Ingress<T, P>,
    workers: Mutex<Vec<Worker>>,
}

impl<T: TaskResult, P: Priority> Limiter<T, P> {
    pub fn new(concurrent: usize) -> Self {
        let tasks_and_limits = Arc::new(TasksAndLimits::default());
        let ingress = Ingress::spawn(tasks_and_limits.clone());
        let workers = Mutex::new(
            (0..concurrent)
                .map(|_| ingress.spawn_worker(tasks_and_limits.clone()))
                .collect(),
        );

        Self {
            tasks_and_limits,
            ingress,
            workers,
        }
    }

    pub async fn wait_until_at_least(&self, instant: Instant) {
        self.tasks_and_limits
            .limits()
            .write()
            .await
            .set_wait_until_at_least(instant);
    }

    pub async fn wait_until_at_least_by_key(&self, instant: Instant, key: impl Into<String>) {
        self.tasks_and_limits
            .limits()
            .write()
            .await
            .set_wait_until_at_least_by_key(instant, key);
    }

    pub async fn wait_until(&self, instant: Option<Instant>) {
        self.tasks_and_limits
            .limits()
            .write()
            .await
            .set_wait_until(instant);
    }

    pub async fn wait_until_by_key(&self, instant: Option<Instant>, key: impl Into<String>) {
        self.tasks_and_limits
            .limits()
            .write()
            .await
            .set_wait_until_by_key(instant, key);
    }

    pub async fn set_rate_limit(&self, limit: Option<Duration>) {
        self.tasks_and_limits
            .limits()
            .write()
            .await
            .set_rate_limit(limit);
    }

    pub async fn set_rate_limit_by_key(&self, limit: Option<Duration>, key: impl Into<String>) {
        self.tasks_and_limits
            .limits()
            .write()
            .await
            .set_rate_limit_by_key(limit, key);
    }

    pub async fn set_concurrent(&self, concurrent: usize) {
        let mut guard = self.workers.lock().await;
        let len = guard.len();

        match len.cmp(&concurrent) {
            std::cmp::Ordering::Less => {
                for _ in len..concurrent {
                    guard.push(self.ingress.spawn_worker(self.tasks_and_limits.clone()));
                }
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                guard.drain(concurrent..);
            }
        }
    }

    pub fn queue<J: Future<Output = T> + Send + 'static>(
        &self,
        job: J,
        priority: P,
    ) -> oneshot::Receiver<T> {
        let (send, recv) = oneshot::channel();

        self.ingress.send(Task {
            key: None,
            priority,
            job: Box::pin(job),
            reply: send,
        });

        recv
    }

    pub fn queue_by_key<J: Future<Output = T> + Send + 'static>(
        &self,
        job: J,
        priority: P,
        key: impl Into<String>,
    ) -> oneshot::Receiver<T> {
        let (send, recv) = oneshot::channel();

        self.ingress.send(Task {
            key: Some(key.into()),
            priority,
            job: Box::pin(job),
            reply: send,
        });

        recv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::join_all;
    use itertools::Itertools;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn it_should_work() {
        let limiter = Limiter::new(0);
        let acc: Arc<Mutex<Vec<&'static str>>> = Default::default();

        let futures = [
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("High");
                        1
                    }
                },
                3,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("Mid");
                        2
                    }
                },
                2,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("Low");
                        3
                    }
                },
                1,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("Low");
                        4
                    }
                },
                1,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("Mid");
                        5
                    }
                },
                2,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("High");
                        6
                    }
                },
                3,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("Mid");
                        7
                    }
                },
                2,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("Low");
                        8
                    }
                },
                1,
            ),
            limiter.queue(
                {
                    let results = acc.clone();
                    async move {
                        results.lock().await.push("High");
                        9
                    }
                },
                3,
            ),
        ];

        limiter
            .set_rate_limit(Some(Duration::from_millis(100)))
            .await;
        limiter.set_concurrent(2).await;

        let order = join_all(futures)
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect_vec();

        assert_eq!(order, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);

        let acc = acc.lock().await.join(" ");

        assert_eq!(acc, "High High High Mid Mid Mid Low Low Low");
    }
}
