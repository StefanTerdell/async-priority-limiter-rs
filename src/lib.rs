#![doc = include_str!("../README.md")]

mod auto_traits;
mod ingress;
mod limiter;
mod limits;
mod task;
mod worker;

#[cfg(feature = "reqwest")]
pub mod reqwest;

pub use limiter::Limiter;
