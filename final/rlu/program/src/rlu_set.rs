// A sorted singly-linked-list set synchronised with Read-Log-Update (see rlu.rs).
//
// The list is a chain of RLU-managed nodes behind a head sentinel (value None).
// Nodes are kept in ascending order. Readers (`contains`, `len`, `to_string`)
// run lock-free, snapshotting the global clock and using `dereference` at every
// hop so they observe a consistent version of the list. Writers (`insert`,
// `delete`) take one global writer lock (the single-writer RLU configuration),
// lock the node(s) they mutate to obtain private copies, splice in/out, and then
// `commit` (bump the clock, wait for older readers to drain, write the copies
// back). Deleted nodes are unlinked but not freed until the whole set is
// dropped, which keeps reclamation trivially safe.

use crate::concurrent_set::ConcurrentSet;
use crate::rlu::{RluBounds, RluGlobal, RluObj, RluThread};
use std::fmt::Debug;
use std::marker::Unpin;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering::SeqCst};
use std::sync::Arc;

struct Node<T> {
  value: Option<T>, // None only for the head sentinel
  next: AtomicPtr<RluObj<Node<T>>>,
}

impl<T: Clone> Clone for Node<T> {
  fn clone(&self) -> Self {
    Node {
      value: self.value.clone(),
      next: AtomicPtr::new(self.next.load(SeqCst)),
    }
  }
}

impl<T: Clone> RluBounds for Node<T> {
  // The only mutable field is `next`; publish it atomically so in-flight readers
  // never see a torn pointer.
  fn rlu_writeback(&self, src: &Self) {
    self.next.store(src.next.load(SeqCst), SeqCst);
  }
}

struct Shared<T> {
  global: *mut RluGlobal<Node<T>>,
  head: *mut RluObj<Node<T>>,
}

unsafe impl<T: Send> Send for Shared<T> {}
unsafe impl<T: Send> Sync for Shared<T> {}

impl<T> Drop for Shared<T> {
  fn drop(&mut self) {
    // Last handle gone and all threads joined: RluGlobal::drop frees every node,
    // copy, and thread context in one shot.
    unsafe {
      drop(Box::from_raw(self.global));
    }
  }
}

pub struct RluSet<T> {
  shared: Arc<Shared<T>>,
  thread: *mut RluThread<Node<T>>,
}

// See rlu.rs: cross-thread access only ever touches atomics or is serialised.
unsafe impl<T> Send for RluSet<T> {}
unsafe impl<T> Sync for RluSet<T> {}

impl<T> RluSet<T>
where
  T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin,
{
  pub fn new() -> RluSet<T> {
    unsafe {
      let global = RluGlobal::new();
      let head = RluGlobal::alloc(
        global,
        Node {
          value: None,
          next: AtomicPtr::new(ptr::null_mut()),
        },
      );
      let thread = RluGlobal::register_thread(global);
      RluSet {
        shared: Arc::new(Shared { global, head }),
        thread,
      }
    }
  }

  pub fn to_string(&self) -> String {
    unsafe {
      let t = &*self.thread;
      t.reader_lock();
      let mut s = String::from("[");
      let mut prev = self.shared.head;
      let mut first = true;
      loop {
        let prev_d = t.dereference(prev);
        let next = RluObj::data(prev_d).next.load(SeqCst);
        if next.is_null() {
          break;
        }
        let next_d = t.dereference(next);
        let v = RluObj::data(next_d).value.as_ref().unwrap();
        if !first {
          s.push_str(", ");
        }
        s.push_str(&format!("{:?}", v));
        first = false;
        prev = next;
      }
      s.push(']');
      t.reader_unlock();
      s
    }
  }
}

impl<T> ConcurrentSet<T> for RluSet<T>
where
  T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin,
{
  fn len(&self) -> usize {
    unsafe {
      let t = &*self.thread;
      t.reader_lock();
      let mut count = 0;
      let mut prev = self.shared.head;
      loop {
        let prev_d = t.dereference(prev);
        let next = RluObj::data(prev_d).next.load(SeqCst);
        if next.is_null() {
          break;
        }
        count += 1;
        prev = next;
      }
      t.reader_unlock();
      count
    }
  }

  fn contains(&self, value: T) -> bool {
    unsafe {
      let t = &*self.thread;
      t.reader_lock();
      let mut prev = self.shared.head;
      let mut found = false;
      loop {
        let prev_d = t.dereference(prev);
        let next = RluObj::data(prev_d).next.load(SeqCst);
        if next.is_null() {
          break;
        }
        let next_d = t.dereference(next);
        let nv = *RluObj::data(next_d).value.as_ref().unwrap();
        if nv == value {
          found = true;
          break;
        }
        if nv > value {
          break; // sorted list: gone past where it would be
        }
        prev = next;
      }
      t.reader_unlock();
      found
    }
  }

  fn insert(&self, value: T) -> bool {
    unsafe {
      let t = &*self.thread;
      let _writer = RluGlobal::lock_writers(self.shared.global);
      t.reader_lock();
      let mut prev = self.shared.head;
      loop {
        let prev_d = t.dereference(prev);
        let next = RluObj::data(prev_d).next.load(SeqCst);
        if next.is_null() {
          break; // insert at the tail
        }
        let next_d = t.dereference(next);
        let nv = *RluObj::data(next_d).value.as_ref().unwrap();
        if nv == value {
          t.reader_unlock();
          return false; // already present
        }
        if nv > value {
          break; // insert between prev and next
        }
        prev = next;
      }
      let prev_copy = t.try_lock(prev);
      let old_next = t.get_mut(prev_copy).next.load(SeqCst);
      let new_node = RluGlobal::alloc(
        self.shared.global,
        Node {
          value: Some(value),
          next: AtomicPtr::new(old_next),
        },
      );
      t.get_mut(prev_copy).next.store(new_node, SeqCst);
      t.reader_unlock(); // commits the write log
      true
    }
  }

  fn delete(&self, value: T) -> bool {
    unsafe {
      let t = &*self.thread;
      let _writer = RluGlobal::lock_writers(self.shared.global);
      t.reader_lock();
      let mut prev = self.shared.head;
      loop {
        let prev_d = t.dereference(prev);
        let cur = RluObj::data(prev_d).next.load(SeqCst);
        if cur.is_null() {
          t.reader_unlock();
          return false; // not found
        }
        let cur_d = t.dereference(cur);
        let cv = *RluObj::data(cur_d).value.as_ref().unwrap();
        if cv == value {
          let cur_next = RluObj::data(cur_d).next.load(SeqCst);
          let prev_copy = t.try_lock(prev);
          t.get_mut(prev_copy).next.store(cur_next, SeqCst);
          t.reader_unlock(); // commits; `cur` is unlinked (freed on drop)
          return true;
        }
        if cv > value {
          t.reader_unlock();
          return false; // gone past it
        }
        prev = cur;
      }
    }
  }

  fn clone_ref(&self) -> Self {
    unsafe {
      let thread = RluGlobal::register_thread(self.shared.global);
      RluSet {
        shared: self.shared.clone(),
        thread,
      }
    }
  }
}
