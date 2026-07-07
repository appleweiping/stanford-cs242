use std::cell::Cell;
use std::ops::{Deref, DerefMut};

/*
 * Part 4 - UseCounter
 *
 * A smart pointer that transparently wraps a value of type `T` and counts how
 * many times the inner value is accessed. Every deref (shared or mutable)
 * bumps the counter. Because `Deref` hands out a `&T` from a `&self`, we need
 * interior mutability to update the count on a shared borrow; a `Cell<usize>`
 * gives us that cheaply and keeps `UseCounter<T>: Send` when `T: Send`.
 */
pub struct UseCounter<T> {
  value: T,
  count: Cell<usize>,
}

impl<T> UseCounter<T> {
  pub fn new(value: T) -> UseCounter<T> {
    UseCounter {
      value,
      count: Cell::new(0),
    }
  }

  /// Number of times the wrapped value has been dereferenced.
  pub fn count(&self) -> usize {
    self.count.get()
  }
}

impl<T> Deref for UseCounter<T> {
  type Target = T;

  fn deref(&self) -> &T {
    self.count.set(self.count.get() + 1);
    &self.value
  }
}

impl<T> DerefMut for UseCounter<T> {
  fn deref_mut(&mut self) -> &mut T {
    self.count.set(self.count.get() + 1);
    &mut self.value
  }
}
