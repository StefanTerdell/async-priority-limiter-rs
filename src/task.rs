use futures::future::BoxFuture;
use tokio::sync::oneshot;

pub(crate) struct Task<T, P: Ord> {
    pub(crate) key: Option<String>,
    pub(crate) priority: P,
    pub(crate) job: BoxFuture<'static, T>,
    pub(crate) reply: oneshot::Sender<T>,
}

impl<T, P: Ord> PartialEq for Task<T, P> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T, P: Ord> Eq for Task<T, P> {}

impl<T, P: Ord> PartialOrd for Task<T, P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, P: Ord> Ord for Task<T, P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}
