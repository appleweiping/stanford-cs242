use take_mut;

/*
 * Core futures interface.
 */

#[derive(Debug)]
pub enum Poll<T> {
  Ready(T),
  NotReady,
}

pub trait Future: Send {
  type Item: Send;
  fn poll(&mut self) -> Poll<Self::Item>;
}

/*
 * Example implementation of a future for an item that returns immediately.
 */

// Container for the state of the future.
pub struct Immediate<T> {
  t: Option<T>,
}

// Constructor to build the future. Note that the return type just says
// "this produces a future", not specifying concretely the type Immediate.
pub fn immediate<T>(t: T) -> impl Future<Item = T>
where
  T: Send,
{
  Immediate { t: Some(t) }
}

// To treat Immediate as a future, we have to implement poll. Here it's
// relatively simple, since we return immediately with a Poll::Ready.
impl<T> Future for Immediate<T>
where
  T: Send,
{
  type Item = T;

  fn poll(&mut self) -> Poll<Self::Item> {
    Poll::Ready(self.t.take().unwrap())
  }
}

/*
 * Example implementation of a future combinator that applies a function to
 * the output of a future.
 */

struct Map<Fut, Fun> {
  fut: Fut,
  fun: Option<Fun>,
}

pub fn map<T, Fut, Fun>(fut: Fut, fun: Fun) -> impl Future<Item = T>
where
  T: Send,
  Fut: Future,
  Fun: FnOnce(Fut::Item) -> T + Send,
{
  Map {
    fut,
    fun: Some(fun),
  }
}

impl<T, Fut, Fun> Future for Map<Fut, Fun>
where
  T: Send,
  Fut: Future,
  Fun: FnOnce(Fut::Item) -> T + Send,
{
  type Item = T;

  fn poll(&mut self) -> Poll<Self::Item> {
    match self.fut.poll() {
      Poll::NotReady => Poll::NotReady,
      Poll::Ready(s) => {
        let f = self.fun.take();
        Poll::Ready(f.unwrap()(s))
      }
    }
  }
}


/*
 * Part 1a - Join
 */

// A join of two futures is a state machine depending on which future is
// completed, represented as an enum.
pub enum Join<F, G>
where
  F: Future,
  G: Future,
{
  BothRunning(F, G),
  FirstDone(F::Item, G),
  SecondDone(F, G::Item),
  Done,
}

// When a join is created, we start by assuming neither child future
// has completed.
pub fn join<F, G>(f: F, g: G) -> impl Future<Item = (F::Item, G::Item)>
where
  F: Future,
  G: Future,
{
  Join::BothRunning(f, g)
}

impl<F, G> Future for Join<F, G>
where
  F: Future,
  G: Future,
{
  type Item = (F::Item, G::Item);

  fn poll(&mut self) -> Poll<Self::Item> {
    // We may need to move the child futures / their results out of `self`
    // in order to transition between states. `take_mut::take` lets us
    // temporarily take ownership of `*self`, compute the next state, and
    // write it back. We stash the final result in `ret` so we can return it
    // after the borrow of `self` has ended.
    let mut ret = Poll::NotReady;
    take_mut::take(self, |join| match join {
      Join::BothRunning(mut f, mut g) => match (f.poll(), g.poll()) {
        (Poll::Ready(a), Poll::Ready(b)) => {
          ret = Poll::Ready((a, b));
          Join::Done
        }
        (Poll::Ready(a), Poll::NotReady) => Join::FirstDone(a, g),
        (Poll::NotReady, Poll::Ready(b)) => Join::SecondDone(f, b),
        (Poll::NotReady, Poll::NotReady) => Join::BothRunning(f, g),
      },
      Join::FirstDone(a, mut g) => match g.poll() {
        Poll::Ready(b) => {
          ret = Poll::Ready((a, b));
          Join::Done
        }
        Poll::NotReady => Join::FirstDone(a, g),
      },
      Join::SecondDone(mut f, b) => match f.poll() {
        Poll::Ready(a) => {
          ret = Poll::Ready((a, b));
          Join::Done
        }
        Poll::NotReady => Join::SecondDone(f, b),
      },
      Join::Done => panic!("Join::poll called after completion"),
    });
    ret
  }
}

/*
 * Part 1b - AndThen
 */

// The AndThen state machine depends on which future is currently running.
pub enum AndThen<Fut1, Fut2, Fun> {
  First(Fut1, Fun),
  Second(Fut2),
  Done,
}

pub fn and_then<Fut1, Fut2, Fun>(fut: Fut1, fun: Fun)
                                 -> impl Future<Item = Fut2::Item>
where
  Fut1: Future,
  Fut2: Future,
  Fun: FnOnce(Fut1::Item) -> Fut2 + Send,
{
  AndThen::First(fut, fun)
}

impl<Fut1, Fut2, Fun> Future for AndThen<Fut1, Fut2, Fun>
where
  Fut1: Future,
  Fut2: Future,
  Fun: FnOnce(Fut1::Item) -> Fut2 + Send,
{
  type Item = Fut2::Item;

  fn poll(&mut self) -> Poll<Self::Item> {
    // First we poll `fut1`. When it completes, we feed its output to `fun`
    // to build the second future, then start polling that. `take_mut::take`
    // lets us move `fut1`/`fun` out of `self` to construct the `Second`
    // state (`fun` is `FnOnce`, so it must be owned to be called).
    let mut ret = Poll::NotReady;
    take_mut::take(self, |and_then| match and_then {
      AndThen::First(mut fut1, fun) => match fut1.poll() {
        Poll::NotReady => AndThen::First(fut1, fun),
        Poll::Ready(v) => {
          // Build and immediately poll the second future so that a chain
          // of ready futures can resolve within a single poll.
          let mut fut2 = fun(v);
          match fut2.poll() {
            Poll::Ready(r) => {
              ret = Poll::Ready(r);
              AndThen::Done
            }
            Poll::NotReady => AndThen::Second(fut2),
          }
        }
      },
      AndThen::Second(mut fut2) => match fut2.poll() {
        Poll::Ready(r) => {
          ret = Poll::Ready(r);
          AndThen::Done
        }
        Poll::NotReady => AndThen::Second(fut2),
      },
      AndThen::Done => panic!("AndThen::poll called after completion"),
    });
    ret
  }
}
