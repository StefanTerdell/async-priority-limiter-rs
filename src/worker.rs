use tokio::sync::{Mutex, RwLock};

use crate::{
    auto_traits::{Priority, TaskResult},
    limits::Limits,
    task::Task,
};

use std::{collections::BinaryHeap, sync::Arc};

pub(crate) struct Worker {
    exit_sender: flume::Sender<()>,
}

impl Worker {
    pub fn spawn<T: TaskResult, P: Priority>(
        tasks: Arc<Mutex<BinaryHeap<Task<T, P>>>>,
        limits: Arc<RwLock<Limits>>,
        notification_receiver: flume::Receiver<()>,
    ) -> Self {
        let (exit_sender, exit_receiver) = flume::bounded(1);

        tokio::spawn({
            async move {
                loop {
                    tokio::select! {
                        biased;

                        _ = exit_receiver.recv_async() => {
                            break
                        },
                        _ = notification_receiver.recv_async() => {
                            let mut tasks_guard = tasks.lock().await;
                            let task = tasks_guard.pop();
                            drop(tasks_guard);

                            if let Some(task) = task {
                                let mut limits_lock = limits.read().await;

                                if let Some(duration) = limits_lock.get_wait_duration(task.key.clone()) {
                                    drop(limits_lock);
                                    tokio::time::sleep(duration).await;
                                    limits_lock = limits.read().await;
                                }

                                let result = if let Some(key) = task.key {
                                    limits_lock.throttle_by_key(task.job, key).await
                                } else {
                                    limits_lock.throttle(task.job).await
                                };

                                let _ = task.reply.send(result);
                            }
                        }
                    }
                }
            }
        });

        Self { exit_sender }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.exit_sender.send(());
    }
}
