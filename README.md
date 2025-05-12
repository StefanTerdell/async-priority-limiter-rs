# Async Priority Limiter

This library exports the Limiter struct. It throttles prioritised tasks by limiting the max concurrent tasks and minimum time between tasks, with up to two levels based on keys.

## Basic use

Let's say I want to call an API at most once a second, one at a time, with the same priority of 10:

```rust
# tokio_test::block_on(async {
use async_priority_limiter::Limiter;
use futures::future::join_all;
use std::time::{Duration, Instant};

// Creates a new limiter with a single concurrent job allowed:
let limiter = Limiter::new(1);
// Set a base-line limit of one request every two seconds:
limiter.set_interval(Some(Duration::from_secs(1))).await;

let start = Instant::now();

// Will complete instantly since the default rate limit has not been hit yet:
let fut_a = limiter
    .queue(
        async move {
            // Do whatever async stuff you wish here.
            // For now we'll just return the task letter and the number of seconds passed since the start of the test.
            ('A', Instant::now().duration_since(start).as_secs())
        },
        10, // <-- The priority value can be anything implementing Ord
    )
    .await; // <-- Queueing is async as an acc is awaited from the ingress thread to guarantee order of operations.

// Would have completed after 1 second, following A, except I'll add a higher priority task before task 'B' has had a chance to aquire a permit.
let fut_b = limiter
    .queue(
        async move { ('B', Instant::now().duration_since(start).as_secs()) },
        10,
    )
    .await;

// Task 'C' will complete after 1 second following 'A', and task 'B' will now complete after 2 seconds instead of 1.
let fut_c = limiter
    .queue(
        async move { ('C', Instant::now().duration_since(start).as_secs()) },
        20,
    )
    .await;

// The values of the queued async blocks are returned as BoxFuture<'static, T> which can be awaited (though the execution itself is potentially immediate):
let results = join_all([fut_a, fut_b, fut_c]).await;

assert_eq!(results, [('A', 0), ('B', 2), ('C', 1)]);
# });
```

## Using keys

All public methods have a variant accepting a "key". This is simply a String that referes to a second layer of limits. Say I wanted to keep sending a single request at a time, at most once every two seconds, but also limit a specific endpoint to a call once every _three_ seconds:

```rust
# tokio_test::block_on(async {
use async_priority_limiter::Limiter;
use futures::future::join_all;
use std::time::{Duration, Instant};

// Creates a new limiter with a single concurrent job allowed:
let limiter = Limiter::new(1);
// Set a base-line limit of one request every two seconds:
limiter.set_interval(Some(Duration::from_secs(2))).await;
// Set limit of one request every five seconds for the key "blargh"
limiter
    .set_interval_by_key(Some(Duration::from_secs(5)), "blargh")
    .await;

let start = Instant::now();

// Will complete instantly since the default rate limit has not been hit yet:
let fut_a = limiter
    .queue(
        async move {
            // Do whatever async stuff you wish here.
            // For now we'll just return the task letter and the number of seconds passed since the start of the test.
            ('A', Instant::now().duration_since(start).as_secs())
        },
        10, // <-- The priority value can be anything implementing Ord
    )
    .await; // <-- Queueing is async as an acc is awaited from the ingress thread to guarantee order of operations.

// Will complete after 4 seconds since the last task has a higher priority
let fut_b = limiter
    .queue(
        async move { ('B', Instant::now().duration_since(start).as_secs()) },
        10,
    )
    .await;

// Will complete after 6 seconds (gets the next key permit at 5 seconds, then the base permit at 6 seconds)
let fut_c = limiter
    .queue_by_key(
        async move { ('C', Instant::now().duration_since(start).as_secs()) },
        10, // <-- Notice that if the priority is the same the order of insertion matters: first in, first out.
        "blargh", // <-- Second layer key
    )
    .await;

// Will complete after 2 seconds (gets key permit immediately, then the next base permit at 2 seconds)
let fut_d = limiter
    .queue_by_key(
        async move { ('D', Instant::now().duration_since(start).as_secs()) },
        20,
        "blargh",
    )
    .await;

// The values of the queued async blocks are returned as BoxFuture<'static, T> which can be awaited (though the execution itself is potentially immediate):
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

let response_a = client
  .get("herpderp")
  .send_limited(&limits)
  .await?
  // If the status is 429 and the Retry-After-header contained a valid value, the limiter will be updated accordingly, setting a hard timeout on requests:
  .update_limiter_by_retry_after_header()
  .await
  .text()
  .await;

// Potentially limited:
let response_b = client
  .get("herpderp")
  .send_limited(&limits)
  .await?
  // If the status is 429 and the header contained a valid value, the limiter will be updated accordingly, setting a hard request stop:
  .update_limiter_by_retry_after_header()
  .await
  .text()
  .await;
```

### `open_ai`

Enables the `update_limiter_by_open_ai_ratelimit_headers` and `update_limiter_by_open_ai_ratelimit_headers_by_key` methods of the `reqwest::Response` struct.

This prevents new requests from being made if remaining tokens or requests reach zero or less until the counter resets.

Requires the `reqwest` feature.
