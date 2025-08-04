use std::hash::Hash;

pub trait TaskResult: Send + 'static {}
impl<T: Send + 'static> TaskResult for T {}

pub trait Priority: Ord + Send + Clone + 'static {}
impl<P: Ord + Send + Clone + 'static> Priority for P {}

pub trait Key: Clone + Hash + PartialEq + Eq + Send + Sync + 'static {}
impl<K: Clone + Hash + PartialEq + Eq + Send + Sync + 'static> Key for K {}
