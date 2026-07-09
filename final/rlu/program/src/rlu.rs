// A from-scratch implementation of the Read-Log-Update (RLU) synchronization
// mechanism of Matveev et al. (SOSP '15), specialised to the single-writer-lock
// configuration that `rlu_set` uses.
//
// Design (see also the module docs in rlu_set.rs):
//   * There is one monotonically increasing `global_clock`.
//   * Every thread that touches the structure owns an `RluThread` context,
//     registered in the global `RluGlobal`. A context records the thread's
//     `local_clock` (the clock it snapshotted at the start of its current
//     critical section), a `write_clock` (the clock it is committing at, else
//     `UNLOCKED`), and a `run_count` (even = outside a section, odd = inside).
//   * Every RLU-managed object (`RluObj<P>`) carries a header: an atomic `copy`
//     pointer. On an *original*, `copy` points to a writer's locked copy (or is
//     null). A *copy* additionally records its `original` and the id of the
//     writer thread that owns it.
//   * Readers are lock-free: `dereference` returns the copy of a locked object
//     iff the copy's owner has already committed at a clock <= the reader's
//     snapshot, and otherwise returns the (old) original. So each reader sees a
//     consistent snapshot as of its `local_clock`.
//   * A writer logs the copies it creates, and on `commit` it bumps the global
//     clock, waits (quiescence, `synchronize`) for every reader that started
//     before the bump to leave its section, then writes the copies back into the
//     originals and unlocks them.
//
// Memory reclamation: writers only ever *add* nodes/copies (deletions unlink a
// node but never free it eagerly). Every allocation is tracked in the global
// arena and freed exactly once, when the last handle to the set is dropped and
// all worker threads have joined. This trades space for a simple, provably
// use-after-free-free reclamation policy (documented in README / final.tex).

use std::cell::UnsafeCell;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering::SeqCst};
use std::sync::Mutex;

pub const RLU_MAX_THREADS: usize = 32;
const UNLOCKED: usize = usize::MAX;

// Payload stored in an RLU object. `rlu_writeback` must *atomically* publish the
// mutable state of `src` (a committed copy) into `self` (the original), so that
// concurrent readers never observe a torn value.
pub trait RluBounds: Clone {
  fn rlu_writeback(&self, src: &Self);
}

pub struct RluObj<P> {
  copy: AtomicPtr<RluObj<P>>, // original: -> active copy or null; copy: null
  original: *mut RluObj<P>,   // null on an original; -> original on a copy
  thread_id: usize,           // owner id (meaningful on a copy)
  data: UnsafeCell<P>,
}

impl<P> RluObj<P> {
  #[inline]
  pub unsafe fn data<'a>(obj: *const RluObj<P>) -> &'a P {
    &*(*obj).data.get()
  }
  #[inline]
  unsafe fn data_mut<'a>(obj: *mut RluObj<P>) -> &'a mut P {
    &mut *(*obj).data.get()
  }
}

pub struct RluGlobal<P> {
  global_clock: AtomicUsize,
  threads: Vec<AtomicPtr<RluThread<P>>>,
  n_threads: AtomicUsize,
  writer_lock: Mutex<()>,
  arena: Mutex<Vec<*mut RluObj<P>>>,           // every object ever allocated
  thread_arena: Mutex<Vec<*mut RluThread<P>>>, // every thread context
}

// Safety: cross-thread access to RluGlobal/RluThread/RluObj only ever touches
// atomic fields or is serialised by `writer_lock`; the arenas are Mutex-guarded.
unsafe impl<P: Send> Send for RluGlobal<P> {}
unsafe impl<P: Send> Sync for RluGlobal<P> {}

impl<P: RluBounds> RluGlobal<P> {
  pub fn new() -> *mut RluGlobal<P> {
    let mut threads = Vec::with_capacity(RLU_MAX_THREADS);
    for _ in 0..RLU_MAX_THREADS {
      threads.push(AtomicPtr::new(ptr::null_mut()));
    }
    Box::into_raw(Box::new(RluGlobal {
      global_clock: AtomicUsize::new(0),
      threads,
      n_threads: AtomicUsize::new(0),
      writer_lock: Mutex::new(()),
      arena: Mutex::new(Vec::new()),
      thread_arena: Mutex::new(Vec::new()),
    }))
  }

  // Register a fresh thread context and return a raw pointer to it. Called once
  // per set handle (i.e. once per participating thread).
  pub unsafe fn register_thread(global: *mut RluGlobal<P>) -> *mut RluThread<P> {
    let id = (*global).n_threads.fetch_add(1, SeqCst);
    assert!(id < RLU_MAX_THREADS, "exceeded RLU_MAX_THREADS");
    let t = Box::into_raw(Box::new(RluThread {
      id,
      global,
      local_clock: AtomicUsize::new(0),
      write_clock: AtomicUsize::new(UNLOCKED),
      run_count: AtomicUsize::new(0),
      is_writer: UnsafeCell::new(false),
      write_log: UnsafeCell::new(Vec::new()),
    }));
    (&(*global).threads)[id].store(t, SeqCst);
    (*global).thread_arena.lock().unwrap().push(t);
    t
  }

  // Allocate a new original object, tracked in the arena for later reclamation.
  pub unsafe fn alloc(global: *mut RluGlobal<P>, data: P) -> *mut RluObj<P> {
    let obj = Box::into_raw(Box::new(RluObj {
      copy: AtomicPtr::new(ptr::null_mut()),
      original: ptr::null_mut(),
      thread_id: 0,
      data: UnsafeCell::new(data),
    }));
    (*global).arena.lock().unwrap().push(obj);
    obj
  }

  pub unsafe fn lock_writers(global: *const RluGlobal<P>) -> std::sync::MutexGuard<'static, ()> {
    (*global).writer_lock.lock().unwrap()
  }
}

impl<P> Drop for RluGlobal<P> {
  fn drop(&mut self) {
    // All threads have joined and all handles dropped: free everything once.
    unsafe {
      for &obj in self.arena.lock().unwrap().iter() {
        drop(Box::from_raw(obj));
      }
      for &t in self.thread_arena.lock().unwrap().iter() {
        drop(Box::from_raw(t));
      }
    }
  }
}

pub struct RluThread<P> {
  id: usize,
  global: *mut RluGlobal<P>,
  local_clock: AtomicUsize,
  write_clock: AtomicUsize,
  run_count: AtomicUsize,
  is_writer: UnsafeCell<bool>,
  write_log: UnsafeCell<Vec<*mut RluObj<P>>>,
}

impl<P: RluBounds> RluThread<P> {
  #[inline]
  fn global(&self) -> *mut RluGlobal<P> {
    self.global
  }

  // ---- critical section brackets ----

  pub fn reader_lock(&self) {
    unsafe {
      *self.is_writer.get() = false;
      (*self.write_log.get()).clear();
    }
    self.run_count.fetch_add(1, SeqCst); // -> odd = active
    let g = unsafe { (*self.global()).global_clock.load(SeqCst) };
    self.local_clock.store(g, SeqCst);
  }

  pub fn reader_unlock(&self) {
    if unsafe { *self.is_writer.get() } {
      self.commit();
    }
    self.run_count.fetch_add(1, SeqCst); // -> even = inactive
  }

  // ---- reads ----

  // Return the version of `obj` that this thread should read.
  pub fn dereference(&self, obj: *mut RluObj<P>) -> *mut RluObj<P> {
    if obj.is_null() {
      return obj;
    }
    let copy = unsafe { (*obj).copy.load(SeqCst) };
    if copy.is_null() {
      return obj; // unlocked
    }
    let owner = unsafe { (*copy).thread_id };
    if owner == self.id {
      return copy; // my own uncommitted copy
    }
    let wc = unsafe { (&(*self.global()).threads)[owner].load(SeqCst) };
    // owner's write_clock; if it has committed at or before our snapshot, the
    // copy is the visible version, else we still read the old original.
    let owner_wc = unsafe { (*wc).write_clock.load(SeqCst) };
    if owner_wc <= self.local_clock.load(SeqCst) {
      copy
    } else {
      obj
    }
  }

  // ---- writes ----

  // Lock `obj` for writing and return the copy to mutate. Assumes at most one
  // active writer (the set holds a global writer lock), so locking never fails.
  pub fn try_lock(&self, obj: *mut RluObj<P>) -> *mut RluObj<P> {
    unsafe {
      *self.is_writer.get() = true;
      let existing = (*obj).copy.load(SeqCst);
      if !existing.is_null() {
        // Already locked. With a single writer this must be our own copy.
        debug_assert_eq!((*existing).thread_id, self.id);
        return existing;
      }
      let data_clone = RluObj::data(obj).clone();
      let copy = Box::into_raw(Box::new(RluObj {
        copy: AtomicPtr::new(ptr::null_mut()),
        original: obj,
        thread_id: self.id,
        data: UnsafeCell::new(data_clone),
      }));
      (*self.global()).arena.lock().unwrap().push(copy);
      (*obj).copy.store(copy, SeqCst); // publish the lock
      (*self.write_log.get()).push(copy);
      copy
    }
  }

  // Mutable access to a locked copy's payload.
  pub unsafe fn get_mut<'a>(&self, copy: *mut RluObj<P>) -> &'a mut P {
    RluObj::data_mut(copy)
  }

  fn commit(&self) {
    unsafe {
      let log = &*self.write_log.get();
      if log.is_empty() {
        *self.is_writer.get() = false;
        return;
      }
      // 1. take a new clock and announce we are committing at it.
      let new_clock = (*self.global()).global_clock.fetch_add(1, SeqCst) + 1;
      self.write_clock.store(new_clock, SeqCst);
      // 2. quiescence: wait for readers that started before the bump to finish.
      self.synchronize(new_clock);
      // 3. write copies back into their originals and unlock.
      for &copy in log.iter() {
        let orig = (*copy).original;
        RluObj::data(orig).rlu_writeback(RluObj::data(copy));
        (*orig).copy.store(ptr::null_mut(), SeqCst);
      }
      // 4. reset.
      self.write_clock.store(UNLOCKED, SeqCst);
      (*self.write_log.get()).clear();
      *self.is_writer.get() = false;
    }
  }

  fn synchronize(&self, write_clock: usize) {
    unsafe {
      let g = self.global();
      let n = (*g).n_threads.load(SeqCst);
      for i in 0..n {
        if i == self.id {
          continue;
        }
        let t = (&(*g).threads)[i].load(SeqCst);
        if t.is_null() {
          continue;
        }
        let start = (*t).run_count.load(SeqCst);
        if start % 2 == 0 {
          continue; // thread i is outside a critical section
        }
        loop {
          // wait until it leaves/re-enters its section, or snapshotted a clock
          // at least as new as ours (so it already sees our writes).
          if (*t).run_count.load(SeqCst) != start {
            break;
          }
          if (*t).local_clock.load(SeqCst) >= write_clock {
            break;
          }
          std::thread::yield_now();
        }
      }
    }
  }
}
