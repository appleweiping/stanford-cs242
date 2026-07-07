use std::mem;
use std::sync::{mpsc, Mutex, Arc};
use std::thread;
use future::{Future, Poll};

/*
 * Core executor interface.
 */

pub trait Executor {
  fn spawn<F>(&mut self, f: F)
  where
    F: Future<Item = ()> + 'static;
  fn wait(&mut self);
}


/*
 * Example implementation of a naive executor that executes futures
 * in sequence.
 */

pub struct BlockingExecutor;

impl BlockingExecutor {
  pub fn new() -> BlockingExecutor {
    BlockingExecutor
  }
}

impl Executor for BlockingExecutor {
  fn spawn<F>(&mut self, mut f: F)
  where
    F: Future<Item = ()>,
  {
    loop {
      if let Poll::Ready(()) = f.poll() {
        break;
      }
    }
  }

  fn wait(&mut self) {}
}

/*
 * Part 2a - Single threaded executor
 */

pub struct SingleThreadExecutor {
  futures: Vec<Box<dyn Future<Item = ()>>>,
}

impl SingleThreadExecutor {
  pub fn new() -> SingleThreadExecutor {
    SingleThreadExecutor { futures: vec![] }
  }
}

impl Executor for SingleThreadExecutor {
  fn spawn<F>(&mut self, f: F)
  where
    F: Future<Item = ()> + 'static,
  {
    // Box the future so heterogeneous future types share one Vec, then
    // queue it. Boxes implement `Future` (see future_util), so they poll
    // uniformly.
    self.futures.push(Box::new(f));
  }

  fn wait(&mut self) {
    // Cooperative round-robin: repeatedly poll every queued future, keeping
    // only the ones still NotReady, until all have completed.
    let mut futures = mem::replace(&mut self.futures, vec![]);
    while !futures.is_empty() {
      let mut still_running = vec![];
      for mut f in futures.into_iter() {
        match f.poll() {
          Poll::Ready(()) => {}
          Poll::NotReady => still_running.push(f),
        }
      }
      futures = still_running;
    }
  }
}

pub struct MultiThreadExecutor {
  sender: mpsc::Sender<Option<Box<dyn Future<Item = ()>>>>,
  threads: Vec<thread::JoinHandle<()>>,
}

impl MultiThreadExecutor {
  pub fn new(num_threads: i32) -> MultiThreadExecutor {
    // Work is sent over an mpsc channel. Since mpsc has a single consumer,
    // we wrap the receiver in Arc<Mutex<..>> so all worker threads can pull
    // from the same queue. A `None` message tells a worker to shut down.
    let (sender, receiver) = mpsc::channel::<Option<Box<dyn Future<Item = ()>>>>();
    let receiver = Arc::new(Mutex::new(receiver));

    let mut threads = vec![];
    for _ in 0..num_threads {
      let receiver = Arc::clone(&receiver);
      let handle = thread::spawn(move || loop {
        // Lock only long enough to dequeue one item, then release so other
        // workers can grab work while we block-poll this future.
        let msg = {
          let lock = receiver.lock().unwrap();
          lock.recv()
        };
        match msg {
          Ok(Some(mut fut)) => loop {
            if let Poll::Ready(()) = fut.poll() {
              break;
            }
          },
          // `None` = shutdown signal; `Err` = all senders dropped.
          Ok(None) | Err(_) => break,
        }
      });
      threads.push(handle);
    }

    MultiThreadExecutor { sender, threads }
  }
}

impl Executor for MultiThreadExecutor {
  fn spawn<F>(&mut self, f: F)
  where
    F: Future<Item = ()> + 'static,
  {
    // Hand the boxed future off to the worker pool. `Future` requires Send,
    // so it's safe to move across the thread boundary.
    self.sender.send(Some(Box::new(f))).unwrap();
  }

  fn wait(&mut self) {
    // Send one shutdown signal per worker, then join them. Each worker
    // finishes its current future (if any) before seeing `None` and exiting,
    // so joining guarantees all spawned work has completed.
    for _ in 0..self.threads.len() {
      self.sender.send(None).unwrap();
    }
    for handle in self.threads.drain(..) {
      handle.join().unwrap();
    }
  }
}
