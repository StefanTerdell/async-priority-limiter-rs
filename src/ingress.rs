use crate::{
    auto_traits::{Priority, TaskResult},
    task::Task,
    tasks_and_limits::TasksAndLimits,
    worker::Worker,
};

use std::sync::Arc;

pub(crate) struct Ingress<T: TaskResult, P: Priority> {
    task_sender: flume::Sender<Task<T, P>>,
    notification_receiver: flume::Receiver<()>,
}

impl<T: TaskResult, P: Priority> Ingress<T, P> {
    pub fn spawn(tasks_and_limits: Arc<TasksAndLimits<T, P>>) -> Self {
        let (task_sender, task_receiver) = flume::unbounded();
        let (notification_sender, notification_receiver) = flume::unbounded();

        tokio::spawn({
            let task_receiver = task_receiver.clone();

            async move {
                while let Ok(task) = task_receiver.recv_async().await {
                    let mut guard = tasks_and_limits.tasks().lock().await;
                    guard.push(task);
                    drop(guard);

                    let _ = notification_sender.send_async(()).await;
                }
            }
        });

        Self {
            task_sender,
            notification_receiver,
        }
    }

    pub fn send(&self, task: Task<T, P>) {
        let _ = self.task_sender.send(task);
    }

    pub fn spawn_worker(&self, tasks_and_limits: Arc<TasksAndLimits<T, P>>) -> Worker {
        Worker::spawn(tasks_and_limits, self.notification_receiver.clone())
    }
}
