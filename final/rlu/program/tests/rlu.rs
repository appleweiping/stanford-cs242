extern crate rand;

use rlu::{ConcurrentSet, RluSet};
use std::collections::HashSet;
use std::thread;

use rand::{thread_rng, Rng};

#[test]
fn rlu_basic() {
  // Single-threaded sanity: the RLU set behaves like an ordinary sorted set.
  let set = RluSet::new();
  assert!(set.insert(3));
  assert!(set.insert(1));
  assert!(set.insert(2));
  assert!(!set.insert(2)); // duplicate rejected
  assert_eq!(set.len(), 3);
  assert_eq!(set.to_string(), "[1, 2, 3]"); // stays sorted
  assert!(set.contains(1) && set.contains(2) && set.contains(3));
  assert!(!set.contains(4));
  assert!(set.delete(2));
  assert!(!set.delete(2));
  assert_eq!(set.to_string(), "[1, 3]");
}

// A heavier concurrent workload: writers own disjoint key ranges (so the final
// contents are deterministic) while extra readers hammer contains() throughout.
// Verifies RLU keeps the list consistent and lets each committed insert become
// visible to a fresh reader.
#[test]
fn rlu_concurrent_partitioned() {
  let set: RluSet<usize> = RluSet::new();
  let n_writers: usize = 4;
  let per_writer: usize = 500;

  let writers: Vec<_> = (0..n_writers)
    .map(|w| {
      let set = set.clone_ref();
      thread::spawn(move || {
        for i in 0..per_writer {
          let key = w * per_writer + i; // disjoint ranges
          assert!(set.insert(key));
        }
      })
    })
    .collect();

  let readers: Vec<_> = (0..8)
    .map(|_| {
      let set = set.clone_ref();
      thread::spawn(move || {
        let mut rng = thread_rng();
        for _ in 0..5000 {
          // never asserts a specific answer (writes are racing) -- just must not
          // crash / corrupt the list.
          let _ = set.contains(rng.gen_range(0, n_writers * per_writer));
        }
      })
    })
    .collect();

  for t in writers {
    t.join().unwrap();
  }
  for t in readers {
    t.join().unwrap();
  }

  // Every inserted key must now be present, and nothing else.
  assert_eq!(set.len(), n_writers * per_writer);
  let expected: HashSet<usize> = (0..n_writers * per_writer).collect();
  for k in &expected {
    assert!(set.contains(*k), "missing key {}", k);
  }
  assert!(!set.contains(n_writers * per_writer));
}
