use crate::{
    blocks::Blocks,
    intervals::Intervals,
    task::Task,
    traits::{Key, Priority, TaskResult},
    worker::Worker,
};

use std::{collections::BinaryHeap, sync::Arc};
use tokio::sync::{Mutex, RwLock, oneshot};

struct TaskAck<K: Key, P: Priority, T: TaskResult> {
    task: Task<K, P, T>,
    ack: oneshot::Sender<()>,
}

pub(crate) struct Ingress<K: Key, P: Priority, T: TaskResult> {
    task_sender: flume::Sender<TaskAck<K, P, T>>,
    notification_receiver: flume::Receiver<()>,
}

impl<K: Key, P: Priority, T: TaskResult> Ingress<K, P, T> {
    pub fn spawn(tasks: Arc<Mutex<BinaryHeap<Task<K, P, T>>>>) -> Self {
        let (task_sender, task_receiver) = flume::unbounded::<TaskAck<K, P, T>>();
        let (notification_sender, notification_receiver) = flume::unbounded::<()>();

        tokio::spawn(async move {
            let mut counter = 0;

            while let Ok(TaskAck { task, ack }) = task_receiver.recv_async().await {
                let task = task.with_index(counter);

                // println!("Ingress - pushing task {task:?}");

                let mut lock = tasks.lock().await;
                lock.push(task);
                drop(lock);
                // tasks.lock().await.push(task);

                let _ = ack.send(());
                let _ = notification_sender.send_async(()).await;

                // println!("Ingress - Ack and notification sent for task {counter}");

                counter += 1;
            }
        });

        Self {
            task_sender,
            notification_receiver,
        }
    }

    pub async fn send(&self, task: Task<K, P, T>) {
        let (ack_sender, ack_receiver) = oneshot::channel();

        let _ = self
            .task_sender
            .send_async(TaskAck {
                task,
                ack: ack_sender,
            })
            .await;

        let _ = ack_receiver.await;
    }

    pub fn spawn_worker(
        &self,
        tasks: Arc<Mutex<BinaryHeap<Task<K, P, T>>>>,
        blocks: Arc<RwLock<Blocks<K>>>,
        intervals: Arc<RwLock<Intervals<K>>>,
    ) -> Worker {
        Worker::spawn(tasks, blocks, intervals, self.notification_receiver.clone())
    }
}
