//! TokioPool is a library designed to help you multithread your
//! tokio applications by providing a pool of threads which you
//! can distribute your load across.
//!
//! ## Example
//!
//! ```rust
//! # extern crate futures;
//! # extern crate tokio_core;
//! # extern crate tokio_pool;
//! #
//! # use tokio_pool::TokioPool;
//! #
//! # use futures::Stream;
//! # use futures::Future;
//! # use std::net::SocketAddr;
//! # use std::sync::Arc;
//! # use tokio_core::net::TcpListener;
//! #
//! # fn main() {
//! // Create a pool with 4 workers
//! let (pool, join) = TokioPool::new(4)
//!     .expect("Failed to create event loop");
//! // Wrap it in an Arc to share it with the listener worker
//! let pool = Arc::new(pool);
//! // We can listen on 8080
//! let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
//!
//! // Clone the pool reference for the listener worker
//! let pool_ref = pool.clone();
//! // Use the first pool worker to listen for connections
//! pool.next_worker().spawn(move |handle| {
//!     // Bind a TCP listener to our address
//!     let listener = TcpListener::bind(&addr, handle).unwrap();
//!     // Listen for incoming clients
//!     listener.incoming().for_each(move |(socket, addr)| {
//!         pool_ref.next_worker().spawn(move |handle| {
//!             // Do work with a client socket
//! #           Ok(())
//!         });
//!
//!         Ok(())
//!     }).map_err(|_| ()) // You might want to log these errors or something
//! });
//!
//! // You might call `join.join()` here, I don't in this example so that
//! // `cargo test` doesn't wait forever.
//! # }
//! ```

extern crate tokio_core;

use std::io;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use tokio_core::reactor::{Core, Remote};

pub struct TokioPool {
    remotes: Vec<Remote>,
    running: Arc<AtomicBool>,
    next_worker: AtomicUsize,
}

pub struct PoolJoin {
    joiners: Vec<JoinHandle<()>>,
}

impl TokioPool {
    /// Create a TokioPool with the given number of workers
    pub fn new(worker_count: usize) -> io::Result<(TokioPool, PoolJoin)> {
        assert!(worker_count != 0);
        let (tx, rx) = mpsc::channel();
        let running = Arc::new(AtomicBool::new(true));

        let mut joiners = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let tx = tx.clone();
            let running = running.clone();

            let join = thread::spawn(move || {
                let mut core = match Core::new() {
                    Ok(core) => core,
                    Err(err) => {
                        tx.send(Err(err)).expect("Channel was closed early");
                        return;
                    }
                };

                tx.send(Ok(core.remote())).expect("Channel was closed early");

                while running.load(Ordering::Relaxed) {
                    core.turn(None);
                }
            });
            joiners.push(join);
        }

        let remotes: io::Result<_> = rx.into_iter().take(worker_count).collect();
        let remotes = remotes?;

        let pool = TokioPool {
            remotes: remotes,
            running: running,
            next_worker: AtomicUsize::new(0),
        };
        let join = PoolJoin { joiners: joiners };
        Ok((pool, join))
    }

    pub fn next_worker(&self) -> &Remote {
        let next = self.next_worker.fetch_add(1, Ordering::SeqCst);
        let idx = next % self.remotes.len();
        &self.remotes[idx]
    }

    /// Stops all of the worker threads
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl PoolJoin {
    /// Joins on the threads. Can only be called once.
    pub fn join(self) {
        for joiner in self.joiners {
            joiner.join().expect("Worker thread panic");
        }
    }
}
