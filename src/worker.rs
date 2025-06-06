use crate::{
    auto_traits::{Key, Priority, TaskResult},
    blocks::Blocks,
    intervals::Intervals,
    task::Task,
};

use std::{collections::BinaryHeap, sync::Arc};
use tokio::sync::{Mutex, RwLock};

pub(crate) struct Worker {
    exit_sender: flume::Sender<()>,
}

impl Worker {
    pub fn spawn<K: Key, P: Priority, T: TaskResult>(
        tasks: Arc<Mutex<BinaryHeap<Task<K, P, T>>>>,
        blocks: Arc<RwLock<Blocks<K>>>,
        intervals: Arc<RwLock<Intervals<K>>>,
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
                            let task = tasks.lock().await.pop();

                            if let Some(task) = task {
                                // println!("Worker - popped {task:?}");

                                blocks.read().await.wait(task.key.as_ref()).await;
                                intervals.read().await.wait(task.key.as_ref()).await;

                                // println!("Worker - {task:?} blocks and intervals awaited");

                                // let index = task.index.unwrap_or(999);

                                let _ = task.reply.send(task.job.await);

                                // println!("Worker - Task {index} completed");
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
