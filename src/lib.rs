#![doc = include_str!("../README.md")]

mod blocks;
mod ingress;
mod intervals;
mod limiter;
mod task;
mod worker;

pub mod traits;

#[cfg(feature = "reqwest")]
pub mod reqwest;

pub type BoxFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub use limiter::{Limiter, builder::LimiterBuilder};
