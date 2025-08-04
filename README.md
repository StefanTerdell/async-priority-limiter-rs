# Async Priority Limiter

This library exports the Limiter struct. It throttles prioritised tasks by limiting the max concurrent tasks and minimum time between tasks, with up to two levels based on keys.

## Basic use

This example sets up a simple queue where tasks are prioritised by integers. Once a task has been popped from the queue and has started waiting for its permit it will be next in line for execution - if not it may be overtaken by a higher priority task.

```rust
# tokio_test::block_on(async {
use async_priority_limiter::Limiter;
use futures::future::join_all;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// Creates a new limiter with a single concurrent job allowed using String for keys:
let limiter = Limiter::new::<String>(1);
// Set a base-line limit of one request every second:
limiter
    .set_default_interval(Some(Duration::from_secs(1)))
    .await;

let start = Instant::now();

// Task 'A' will execute instantly since the default rate limit has not yet been hit:
let fut_a = limiter
    .queue(
        async move {
            // Mark how many seconds have passed when the task starts
            let task_started = Instant::now().duration_since(start).as_secs();

            // Do whatever async stuff you wish here.
            sleep(Duration::from_millis(100)).await;

            // For now we'll just return the task letter and start time
            ('A', task_started)
        },
        // The priority value can be any type T implementing Ord:
        10,
    )
    // Queueing is async as an ack signal is awaited from the ingress
    // thread to guarantee the order of operations.
    .await;

// Task 'B´ would have started after 1 second following A, except we'll add a
// higher priority task before it has had a chance to start.
let fut_b = limiter
    .queue(
        async move {
            let task_started = Instant::now().duration_since(start).as_secs();

            sleep(Duration::from_millis(100)).await;

            ('B', task_started)
        },
        10,
    )
    .await;

// Task 'C' has a higher priority than 'B' and will thus start after 1 second
// following 'A', while task 'B' will start after 2 seconds instead of 1.
let fut_c = limiter
    .queue(
        async move {
            let task_started = Instant::now().duration_since(start).as_secs();

            sleep(Duration::from_millis(100)).await;

            ('C', task_started)
        },
        20,
    )
    .await;

// The values of the queued async blocks are returned as 'static boxed futures
// which can be awaited - just beware that execution is potentially immediate:
let results = join_all([fut_a, fut_b, fut_c]).await;

assert_eq!(results, [('A', 0), ('B', 2), ('C', 1)]);
# });
```

## Using keys

All relevant methods have a variant accepting a "key". This key referes to a second layer of limits. Say I wanted to keep sending a single request at a time, at most once every two seconds, but also limit a specific endpoint to a call once every _three_ seconds:

> Note: the default limits act as a baseline and is always awaited along the key limits. Adding a key limit that is faster than the baseline therefore does nothing!

```rust
# tokio_test::block_on(async {
use async_priority_limiter::Limiter;
use futures::future::join_all;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// Creates a new limiter with a single concurrent job allowed and using &'static str for keys:
let limiter = Limiter::new(1);
// Set a base-line limit of one request every two seconds:
limiter
    .set_default_interval(Some(Duration::from_secs(2)))
    .await;
// Set limit of one request every five seconds for the key "blargh"
limiter
    .set_interval_by_key(Some(Duration::from_secs(5)), "blargh")
    .await;

let start = Instant::now();

// Task 'A' will execute instantly since the default rate limit has not yet been hit:
let fut_a = limiter
    .queue(
        async move {
            let task_started = Instant::now().duration_since(start).as_secs();
            sleep(Duration::from_millis(100)).await;

            ('A', task_started)
        },
        10,
    )
    .await;

// Task 'B' will execute after 4 seconds:
// Task 'A' and 'D' will get the first two default permits
let fut_b = limiter
    .queue(
        async move {
            let task_started = Instant::now().duration_since(start).as_secs();
            sleep(Duration::from_millis(100)).await;

            ('B', task_started)
        },
        10,
    )
    .await;

// Task 'C' will execute after 6 seconds:
// It gets the next key permit following task 'D' at 5 seconds,
// then the next default permit at 6 after tasks 'A' and 'D'
let fut_c = limiter
    .queue_by_key(
        async move {
            let task_started = Instant::now().duration_since(start).as_secs();
            sleep(Duration::from_millis(100)).await;

            ('C', task_started)
        },
        10, // <-- Notice that if the priority is the same the order of insertion matters: first in, first out.
        "blargh", // <-- Second layer key
    )
    .await;

// Task 'D' will execute after 2 seconds,
// as it gets the next default permit after task 'A'
let fut_d = limiter
    .queue_by_key(
        async move {
            let task_started = Instant::now().duration_since(start).as_secs();
            sleep(Duration::from_millis(100)).await;

            ('D', task_started)
        },
        20,
        "blargh",
    )
    .await;

// The values of the queued async blocks are returned as 'static boxed futures
// which can be awaited - just beware that execution is potentially immediate:
let results = join_all([fut_a, fut_b, fut_c, fut_d]).await;

assert_eq!(results, [('A', 0), ('B', 4), ('C', 6), ('D', 2)]);
# });
```

## Features

### `reqwest`

Enables the `send_limiter` and `send_limited_by_key` methods for the `reqwest::RequestBuilder` struct, and the `update_limiter_by_retry_after_header` and `update_limiter_by_key_and_retry_after_header` for the `reqwest::Response` struct:

```not-rust
let client = reqwest::Client::new();
let limits = async_priority_limiter::Limiter::new(1);

let response = client
  .get("herpderp")
  .send_limited(&limits)
  .await?
  // If the status is 429 and the Retry-After-header contained a valid value, the limiter will be updated accordingly, setting a hard timeout on requests:
  .update_limiter_by_retry_after_header(&limits)
  .await;
```

### `open_ai`

Enables the `update_limiter_by_open_ai_ratelimit_headers` and `update_limiter_by_open_ai_ratelimit_headers_by_key` methods of the `reqwest::Response` struct.

This prevents new requests from being made if remaining tokens or requests reach zero or less until the counter resets.

Requires the `reqwest` feature.
