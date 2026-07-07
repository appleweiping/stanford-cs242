use future::*;
use std::path::PathBuf;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::fs;
use std::io;

pub struct FileReader {
  path: PathBuf,
  thread: Option<thread::JoinHandle<io::Result<String>>>,
  done_flag: Arc<AtomicBool>,
}

impl FileReader {
  pub fn new(path: PathBuf) -> FileReader {
    // Kick off the blocking file read on a background thread immediately.
    // The thread flips `done_flag` to true once the read finishes, so the
    // executor can poll cheaply (just an atomic load) instead of blocking.
    let done_flag = Arc::new(AtomicBool::new(false));
    let thread_flag = Arc::clone(&done_flag);
    let thread_path = path.clone();
    let thread = thread::spawn(move || {
      let result = fs::read_to_string(&thread_path);
      thread_flag.store(true, Ordering::SeqCst);
      result
    });

    FileReader {
      path,
      thread: Some(thread),
      done_flag,
    }
  }
}

impl Future for FileReader {
  type Item = io::Result<String>;

  fn poll(&mut self) -> Poll<Self::Item> {
    if self.done_flag.load(Ordering::SeqCst) {
      // The reader thread has finished; join it to retrieve the result.
      // `take` ensures we only join once (a second poll would be a bug).
      let handle = self.thread.take().expect("FileReader polled after completion");
      match handle.join() {
        Ok(result) => Poll::Ready(result),
        Err(_) => Poll::Ready(Err(io::Error::new(
          io::ErrorKind::Other,
          format!("file reader thread panicked for {:?}", self.path),
        ))),
      }
    } else {
      Poll::NotReady
    }
  }
}
