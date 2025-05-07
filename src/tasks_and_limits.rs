use crate::{
    auto_traits::{Priority, TaskResult},
    limits::Limits,
    task::Task,
};

use std::collections::BinaryHeap;
use tokio::sync::{Mutex, RwLock};

pub(crate) struct TasksAndLimits<T: TaskResult, P: Priority> {
    tasks: Mutex<BinaryHeap<Task<T, P>>>,
    limits: RwLock<Limits>,
}

impl<T: TaskResult, P: Priority> TasksAndLimits<T, P> {
    pub fn tasks(&self) -> &Mutex<BinaryHeap<Task<T, P>>> {
        &self.tasks
    }

    pub fn limits(&self) -> &RwLock<Limits> {
        &self.limits
    }
}

impl<T: TaskResult, P: Priority> Default for TasksAndLimits<T, P> {
    fn default() -> Self {
        Self {
            tasks: Default::default(),
            limits: Default::default(),
        }
    }
}
