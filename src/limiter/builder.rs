use super::Limiter;
use crate::{
    blocks::Blocks,
    intervals::Intervals,
    traits::{Key, Priority, TaskResult},
};

use tokio::time::Instant;

#[derive(Debug)]
pub struct LimiterBuilder<K: Key> {
    pub(crate) concurrent_tasks: usize,
    pub(crate) blocks: Blocks<K>,
    pub(crate) intervals: Intervals<K>,
}

impl LimiterBuilder<String> {
    pub fn new<K: Key>(concurrent_tasks: usize) -> LimiterBuilder<K> {
        LimiterBuilder {
            concurrent_tasks,
            blocks: Default::default(),
            intervals: Default::default(),
        }
    }
}

impl<K: Key> LimiterBuilder<K> {
    pub fn with_concurrent_tasks(mut self, concurrent_tasks: usize) -> Self {
        self.concurrent_tasks = concurrent_tasks;
        self
    }
    pub fn with_block_until(mut self, instant: Option<Instant>) -> Self {
        self.blocks.set_default(instant);
        self
    }
    pub fn with_block_until_at_least(mut self, instant: Instant) -> Self {
        self.blocks.set_default_at_least(instant);
        self
    }
    pub fn with_block_by_key_until(mut self, instant: Option<Instant>, key: K) -> Self {
        self.blocks.set_by_key(instant, key);
        self
    }
    pub fn with_block_by_key_until_at_least(mut self, instant: Instant, key: K) -> Self {
        self.blocks.set_at_least_by_key(instant, key);
        self
    }

    pub fn build<P: Priority, T: TaskResult>(self) -> Limiter<K, P, T> {
        Limiter::new_with::<K>(self.concurrent_tasks, self.blocks, self.intervals)
    }
}
