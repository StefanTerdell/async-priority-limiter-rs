pub trait TaskResult: Send + 'static {}
impl<T: Send + 'static> TaskResult for T {}

pub trait Priority: Ord + Send + 'static {}
impl<P: Ord + Send + 'static> Priority for P {}
