use crate::{
    auto_traits::{Priority, TaskResult},
    limits::Limits,
    task::Task,
    worker::Worker,
};

use std::{collections::BinaryHeap, sync::Arc};
use tokio::sync::{Mutex, RwLock, oneshot};

struct TaskAck<T: TaskResult, P: Priority> {
    task: Task<T, P>,
    ack: oneshot::Sender<()>,
}

pub(crate) struct Ingress<T: TaskResult, P: Priority> {
    task_sender: flume::Sender<TaskAck<T, P>>,
    notification_receiver: flume::Receiver<()>,
}

impl<T: TaskResult, P: Priority> Ingress<T, P> {
    pub fn spawn(tasks: Arc<Mutex<BinaryHeap<Task<T, P>>>>) -> Self {
        let (task_sender, task_receiver) = flume::unbounded::<TaskAck<T, P>>();
        let (notification_sender, notification_receiver) = flume::unbounded();

        tokio::spawn(async move {
            let mut counter = 0;

            while let Ok(TaskAck { task, ack }) = task_receiver.recv_async().await {
                tasks.lock().await.push(task.with_index(counter));

                counter += 1;

                let _ = ack.send(());
                let _ = notification_sender.send_async(()).await;
            }
        });

        Self {
            task_sender,
            notification_receiver,
        }
    }

    pub async fn send(&self, task: Task<T, P>) {
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
        tasks: Arc<Mutex<BinaryHeap<Task<T, P>>>>,
        limits: Arc<RwLock<Limits>>,
    ) -> Worker {
        Worker::spawn(tasks, limits, self.notification_receiver.clone())
    }
}
