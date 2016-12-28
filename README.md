# tokio-pool

```toml
[dependencies]
tokio-pool = "0.1"
```

[Documentation](https://docs.rs/tokio-pool)

TokioPool is a library designed to help you multithread your
tokio applications by providing a pool of threads which you
can distribute your load across.

# Example

```rust
extern crate futures;
extern crate tokio_core;
extern crate tokio_pool;

use futures::Future;

use futures::Stream;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_core::net::TcpListener;
use tokio_pool::TokioPool;

fn main() {
    // Create a pool with 4 workers
    let (pool, join) = TokioPool::new(4).expect("Failed to create event loop");
    // Wrap it in an Arc to share it with the listener worker
    let pool = Arc::new(pool);
    // We can listen on 8080
    let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();

    // Clone the pool reference for the listener worker
    let pool_ref = pool.clone();
    // Use the first pool worker to listen for connections
    pool.next_worker().spawn(move |handle| {
        // Bind a TCP listener to our address
        let listener = TcpListener::bind(&addr, handle).unwrap();
        // Listen for incoming clients
        listener.incoming().for_each(move |(socket, addr)| {
            pool_ref.next_worker().spawn(move |handle| {
                // Do work with a client socket
            });

            Ok(())
        }).map_err(|_| ())
    });

    join.join();
}
```
