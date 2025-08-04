use std::{collections::HashMap, time::Duration};
use tokio::{
    join,
    sync::Mutex,
    time::{Instant, sleep},
};

use crate::traits::Key;

pub struct Intervals<K: Key> {
    default: Option<(Duration, Mutex<Instant>)>,
    by_key: HashMap<K, (Duration, Mutex<Instant>)>,
}

impl<K: Key> Default for Intervals<K> {
    fn default() -> Self {
        Self {
            default: None,
            by_key: HashMap::new(),
        }
    }
}

async fn tick((duration, instant): &(Duration, Mutex<Instant>)) {
    // let (curr_duration, next_duration) = {
    let curr_duration = {
        let mut curr_instant = instant.lock().await;

        let now = Instant::now();

        let curr_duration = curr_instant.duration_since(now);
        let next_duration = curr_duration + *duration;
        let next_instant = now + next_duration;

        *curr_instant = next_instant;

        // (curr_duration, next_duration)
        curr_duration
    };

    if !curr_duration.is_zero() {
        //     println!("Intervals - Wait - Duration {duration:?} : Delta {curr_duration:?} ");
        sleep(curr_duration).await;
        //     println!("Intervals - Done - Duration {duration:?} : Delta {curr_duration:?} ");
        // } else {
        //     println!("Intervals - Skip - Duration {duration:?} : Delta {curr_duration:?} ");
    }

    // if next_duration.is_zero() {
    // println!("Intervals - Next - Duration {duration:?} : Nothing added")
    // } else {
    // println!("Intervals - Next - Duration {duration:?} : Added {next_duration:?}")
    // }
}

impl<K: Key> Intervals<K> {
    pub async fn wait(&self, key: Option<&K>) {
        match (
            self.default.as_ref(),
            key.as_ref().and_then(|key| self.by_key.get(key)),
        ) {
            (None, None) => {
                // println!("Intervals - None");
            }
            (None, Some(by_key)) => {
                // println!("Intervals - Key {:?}", by_key.0);
                tick(by_key).await;
            }
            (Some(default), None) => {
                // println!("Intervals - Default {:?}", default.0);
                tick(default).await;
            }
            (Some(default), Some(by_key)) => {
                // println!("Intervals - Default {:?} & Key {:?}", default.0, by_key.0);
                join!(tick(default), tick(by_key));
            }
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

    pub fn set_default_at_least(&mut self, period: Duration) {
        if self
            .default
            .as_ref()
            .is_none_or(|(previous, _)| previous < &period)
        {
            self.default = Some((period, (Instant::now()).into()));
        }
    }

    pub fn set_at_least_by_key(&mut self, period: Duration, key: K) {
        if self
            .by_key
            .get(&key)
            .is_none_or(|(previous, _)| previous < &period)
        {
            self.by_key.insert(key, (period, (Instant::now()).into()));
        }
    }

    pub fn set_default(&mut self, period: Option<Duration>) {
        self.default = period.map(|period| (period, (Instant::now()).into()));
    }

    pub fn set_by_key(&mut self, period: Option<Duration>, key: K) {
        if let Some(period) = period {
            self.by_key.insert(key, (period, (Instant::now()).into()));
        } else {
            self.by_key.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[tokio::test]
    async fn default() {
        let mut intervals = Intervals::<String>::default();

        intervals.set_default(Some(Duration::from_secs(1)));
        // intervals.set_by_key(Some(Duration::from_secs(2)), "derp".into());

        let start = Instant::now();

        let key = "derp".to_string();

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 0;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 1;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 2;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 3;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 5;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 6;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);
    }

    #[tokio::test]
    async fn by_key() {
        let mut intervals = Intervals::<String>::default();

        let key = "derp".to_string();

        // intervals.set(Some(Duration::from_secs(1)));
        intervals.set_by_key(Some(Duration::from_secs(2)), key.clone());

        let start = Instant::now();

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 0;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 2;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 6;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 8;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);
    }

    #[tokio::test]
    async fn default_and_by_key() {
        let mut intervals = Intervals::<String>::default();

        let key = "derp".to_string();

        intervals.set_default(Some(Duration::from_secs(1)));
        intervals.set_by_key(Some(Duration::from_secs(2)), key.clone());

        let start = Instant::now();

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 0;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 2;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 4;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_default().await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 5;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 6;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);

        intervals.wait_by_key(&key).await;
        let passed = Instant::now().duration_since(start).as_secs();
        let expected = 8;
        println!("Passed: {passed}s Expected: {expected}s");
        assert_eq!(passed, expected);
    }
}
