use backend::{order, login, UserId};
use std::marker::PhantomData;

// Typestate markers. The `Cart` carries one of these as a phantom type
// parameter so the compiler statically enforces which operations are legal in
// each state:
//   Empty     - just logged in, no items. Can add items. Cannot check out.
//   NonEmpty  - has >= 1 item. Can add more items, or check out.
//   Checkout  - frozen for payment. Cannot add items. Can cancel or order.
pub struct Empty;
pub struct NonEmpty;
pub struct Checkout;

// A shopping cart parameterized by its state `S`. The item prices and the
// authenticated user are common to every state; `PhantomData<S>` records the
// state at the type level only (zero runtime cost).
pub struct Cart<S> {
  user: UserId,
  items: Vec<f64>,
  _state: PhantomData<S>,
}

impl<S> Cart<S> {
  // Internal constructor: move the payload into a cart of a chosen state.
  fn into_state<T>(self) -> Cart<T> {
    Cart {
      user: self.user,
      items: self.items,
      _state: PhantomData,
    }
  }

  /// Total price of everything currently in the cart.
  pub fn total(&self) -> f64 {
    self.items.iter().sum()
  }
}

impl Cart<Empty> {
  /// Authenticate and obtain a fresh, empty cart. Fails if login fails.
  pub fn login(username: String, password: String) -> Result<Cart<Empty>, String> {
    let user = login(username, password)?;
    Ok(Cart {
      user,
      items: Vec::new(),
      _state: PhantomData,
    })
  }

  /// Adding the first item transitions Empty -> NonEmpty.
  pub fn additem(mut self, price: f64) -> Cart<NonEmpty> {
    self.items.push(price);
    self.into_state()
  }
}

impl Cart<NonEmpty> {
  /// Add another item; the cart stays NonEmpty.
  pub fn additem(mut self, price: f64) -> Cart<NonEmpty> {
    self.items.push(price);
    self.into_state()
  }

  /// Freeze the cart for payment: NonEmpty -> Checkout.
  pub fn checkout(self) -> Cart<Checkout> {
    self.into_state()
  }
}

impl Cart<Checkout> {
  /// Abandon the checkout and return to editing the cart.
  pub fn cancel(self) -> Cart<NonEmpty> {
    self.into_state()
  }

  /// Attempt to place the order. On success we get a fresh Empty cart (ready
  /// for the next order); on failure the Checkout cart is returned unchanged
  /// alongside the backend error so the caller can retry.
  pub fn order(self) -> Result<Cart<Empty>, (Cart<Checkout>, String)> {
    let amount = self.total();
    match order(&self.user, amount) {
      Ok(()) => {
        let mut empty: Cart<Empty> = self.into_state();
        empty.items.clear();
        Ok(empty)
      }
      Err(e) => Err((self, e)),
    }
  }
}
