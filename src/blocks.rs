use std::{collections::HashMap, time::Duration};
use tokio::time::{Instant, sleep};

use crate::auto_traits::Key;

pub struct Blocks<K: Key> {
    default: Option<Instant>,
    by_key: HashMap<K, Instant>,
}

impl<K: Key> Default for Blocks<K> {
    fn default() -> Self {
        Self {
            default: None,
            by_key: HashMap::new(),
        }
    }
}

impl<K: Key> Blocks<K> {
    pub fn get(&self, key: Option<&K>) -> Option<Duration> {
        self.default
            .as_ref()
            .max(key.and_then(|key| self.by_key.get(key)))
            .map(|instant| instant.duration_since(Instant::now()))
            .filter(|duration| !duration.is_zero())
    }

    pub fn get_default(&self) -> Option<Duration> {
        self.get(None)
    }

    pub fn get_by_key(&self, key: &K) -> Option<Duration> {
        self.get(Some(key))
    }

    pub async fn wait(&self, key: Option<&K>) {
        if let Some(duration) = self.get(key) {
            sleep(duration).await;
        }
    }

    #[allow(unused)]
    pub async fn wait_default(&self) {
        self.wait(None).await;
    }

    #[allow(unused)]
    pub async fn wait_by_key(&self, key: &K) {
        self.wait(Some(key)).await;
    }

    pub fn set_default_at_least(&mut self, instant: Instant) {
        if self
            .default
            .as_ref()
            .is_none_or(|previous| previous < &instant)
        {
            self.default = Some(instant);
        }
    }

    pub fn set_at_least_by_key(&mut self, instant: Instant, key: K) {
        if self
            .by_key
            .get(&key)
            .is_none_or(|previous| previous < &instant)
        {
            self.by_key.insert(key, instant);
        }
    }

    pub fn set_default(&mut self, instant: Option<Instant>) {
        self.default = instant;
    }

    pub fn set_by_key(&mut self, instant: Option<Instant>, key: K) {
        if let Some(instant) = instant {
            self.by_key.insert(key, instant);
        } else {
            self.by_key.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default() {
        let mut blocks = Blocks::<String>::default();

        let start = Instant::now();

        blocks.set_default(Some(start + Duration::from_secs(1)));

        blocks.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 1;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        blocks.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 1;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);
    }

    #[tokio::test]
    async fn by_key() {
        let mut blocks = Blocks::default();

        let start = Instant::now();

        let key = "derp";

        blocks.set_by_key(Some(start + Duration::from_secs(1)), key);

        blocks.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 1;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        blocks.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 1;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);
    }

    #[tokio::test]
    async fn default_and_by_key() {
        let mut blocks = Blocks::default();

        let start = Instant::now();

        let key_a = "derp";
        let key_b = "herp";
        let key_c = "flerp";

        blocks.set_default(Some(start + Duration::from_secs(1)));
        blocks.set_by_key(Some(start + Duration::from_secs(2)), key_a);
        blocks.set_by_key(Some(start + Duration::from_secs(4)), key_b);
        blocks.set_by_key(Some(start + Duration::from_secs(3)), key_c);

        // blocks.wait(&None).await;
        // let passed = Instant::now().duration_since(start).as_secs();
        // let expected = 1;
        // println!("Passed: {passed}s Expected: {expected}s");
        // assert_eq!(passed, expected);

        blocks.wait_by_key(&key_a).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 2;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        blocks.wait_by_key(&key_b).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        blocks.wait_by_key(&key_c).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);
    }
}
