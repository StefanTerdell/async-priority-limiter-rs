#![doc = include_str!("../README.md")]

mod auto_traits;
mod blocks;
mod ingress;
mod intervals;
mod limiter;
mod task;
mod worker;

#[cfg(feature = "reqwest")]
pub mod reqwest;

pub type BoxFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub use limiter::Limiter;
