use std::{mem, fmt};
use std::fmt::{Display, Debug};

#[derive(PartialEq, Eq, Clone)]
pub enum BinaryTree<T> {
  Leaf,
  Node(T, Box<BinaryTree<T>>, Box<BinaryTree<T>>)
}

use BinaryTree::{Leaf, Node};

// Remove the right-most (in-order maximum) node from a non-empty tree and
// return its value. The removed node has no right child, so it is spliced out
// by replacing it with its own left subtree. No `T: Clone` needed -- values are
// moved with `mem::replace`.
fn remove_rightmost<T>(t: &mut BinaryTree<T>) -> T {
  if let Node(_, _, right) = t {
    if matches!(**right, Node(..)) {
      return remove_rightmost(right);
    }
  }
  match mem::replace(t, Leaf) {
    Node(v, left, _leaf_right) => {
      *t = *left; // splice the (right-most) node out, promoting its left subtree
      v
    }
    Leaf => unreachable!("remove_rightmost called on an empty tree"),
  }
}

// Mirror of `remove_rightmost`: remove the left-most (in-order minimum) node.
fn remove_leftmost<T>(t: &mut BinaryTree<T>) -> T {
  if let Node(_, left, _) = t {
    if matches!(**left, Node(..)) {
      return remove_leftmost(left);
    }
  }
  match mem::replace(t, Leaf) {
    Node(v, _leaf_left, right) => {
      *t = *right;
      v
    }
    Leaf => unreachable!("remove_leftmost called on an empty tree"),
  }
}

impl<T: Debug + Display + PartialOrd> BinaryTree<T> {
  pub fn len(&self) -> usize {
    match self {
      Leaf => 0,
      Node(_, l, r) => 1 + l.len() + r.len(),
    }
  }

  pub fn to_vec(&self) -> Vec<&T> {
    let mut acc = Vec::new();
    self.inorder(&mut acc);
    acc
  }

  fn inorder<'a>(&'a self, acc: &mut Vec<&'a T>) {
    if let Node(x, l, r) = self {
      l.inorder(acc);
      acc.push(x);
      r.inorder(acc);
    }
  }

  pub fn sorted(&self) -> bool {
    // A BST is sorted iff its in-order traversal is strictly increasing.
    self.to_vec().windows(2).all(|w| w[0] < w[1])
  }

  pub fn insert(&mut self, t: T) {
    match self {
      Leaf => *self = Node(t, Box::new(Leaf), Box::new(Leaf)),
      Node(v, l, r) => {
        if t < *v {
          l.insert(t)
        } else if t > *v {
          r.insert(t)
        }
        // t == *v: already present, set semantics -> ignore.
      }
    }
  }

  // Return the least element that is >= `query` (ceiling / lower-bound search),
  // or None if every element is strictly less than `query`.
  pub fn search(&self, query: &T) -> Option<&T> {
    match self {
      Leaf => None,
      Node(v, l, r) => {
        if v < query {
          r.search(query) // v is too small; the answer (if any) is to the right
        } else {
          // v >= query is a candidate; a smaller valid candidate can only be
          // in the left subtree (all of whose values are < v).
          l.search(query).or(Some(v))
        }
      }
    }
  }

  // Move one element from deep in the tree up to the root, per the assignment:
  // walk down the right spine of the left subtree, pull that (maximum) element
  // up to become the new root, and demote the old root into the right subtree.
  // When the left subtree is empty, do the mirror image on the right subtree.
  // All surgery is done by moving sub-trees with `mem::replace` -- no cloning,
  // because `T: Clone` is deliberately not a bound.
  pub fn rebalance(&mut self) {
    let tree = mem::replace(self, Leaf);
    *self = match tree {
      Leaf => Leaf,
      Node(v, mut l, r) => {
        if matches!(*l, Node(..)) {
          let m = remove_rightmost(&mut l);
          Node(m, l, Box::new(Node(v, Box::new(Leaf), r)))
        } else if matches!(*r, Node(..)) {
          let mut r = r;
          let m = remove_leftmost(&mut r);
          Node(m, Box::new(Node(v, l, Box::new(Leaf))), r)
        } else {
          Node(v, l, r) // single node: nothing to do
        }
      }
    };
  }


  // Adapted from https://github.com/bpressure/ascii_tree
  fn fmt_levels(&self, f: &mut fmt::Formatter<'_>, level: Vec<usize>) -> fmt::Result {
    use BinaryTree::*;
    const EMPTY: &str = "   ";
    const EDGE: &str = " └─";
    const PIPE: &str = " │ ";
    const BRANCH: &str = " ├─";

    let maxpos = level.len();
    let mut second_line = String::new();
    for (pos, l) in level.iter().enumerate() {
      let last_row = pos == maxpos - 1;
      if *l == 1 {
        if !last_row { write!(f, "{}", EMPTY)? } else { write!(f, "{}", EDGE)? }
        second_line.push_str(EMPTY);
      } else {
        if !last_row { write!(f, "{}", PIPE)? } else { write!(f, "{}", BRANCH)? }
        second_line.push_str(PIPE);
      }
    }

    match self {
      Node(s, l, r) => {
        let mut d = 2;
        write!(f, " {}\n", s)?;
        for t in &[l, r] {
          let mut lnext = level.clone();
          lnext.push(d);
          d -= 1;
          t.fmt_levels(f, lnext)?;
        }
      }
      Leaf => {write!(f, "\n")?}
    }
    Ok(())
  }
}

impl<T: Debug + Display + PartialOrd> fmt::Debug for BinaryTree<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.fmt_levels(f, vec![])
  }
}

#[cfg(test)]
mod test {
  use lazy_static::lazy_static;
  use super::BinaryTree::*;
  use crate::BinaryTree;

  lazy_static! {
    static ref TEST_TREE: BinaryTree<&'static str> = {
      Node(
        "B",
        Box::new(Node("A", Box::new(Leaf), Box::new(Leaf))),
        Box::new(Node("C", Box::new(Leaf), Box::new(Leaf))))
    };
  }

  #[test]
  fn len_test() {
    assert_eq!(TEST_TREE.len(), 3);
  }

  #[test]
  fn to_vec_test() {
    assert_eq!(TEST_TREE.to_vec(), vec![&"A", &"B", &"C"]);
  }

  #[test]
  fn sorted_test() {
    let mut t = TEST_TREE.clone();
    assert!(t.sorted());

    t = Node("D", Box::new(Leaf), Box::new(t));
    assert!(!t.sorted());
  }

  #[test]
  fn insertion_test() {
    let mut t = TEST_TREE.clone();
    t.insert("E");
    assert!(t.sorted());
  }

  #[test]
  fn search_test() {
    let mut t= TEST_TREE.clone();
    t.insert("E");
    assert!(t.search(&"D") == Some(&"E"));
    assert!(t.search(&"C") == Some(&"C"));
    assert!(t.search(&"F") == None);
  }

  #[test]
  fn rebalance1_test() {
    let mut t = Node(
      "D",
      Box::new(Node(
        "B",
        Box::new(Node(
          "A", Box::new(Leaf), Box::new(Leaf))),
        Box::new(Node(
          "C", Box::new(Leaf), Box::new(Leaf))))),
      Box::new(Node(
        "E", Box::new(Leaf), Box::new(Leaf))));

    let t2 = Node(
      "C",
      Box::new(Node(
        "B",
        Box::new(Node(
          "A", Box::new(Leaf), Box::new(Leaf))),
        Box::new(Leaf))),
      Box::new(Node(
        "D",
        Box::new(Leaf),
        Box::new(Node(
          "E", Box::new(Leaf), Box::new(Leaf)))
      )));

    t.rebalance();
    assert_eq!(t, t2);
  }

  #[test]
  fn rebalance2_test() {
    let mut t = Node(
      "A",
      Box::new(Leaf),
      Box::new(Node(
        "B",
        Box::new(Leaf),
        Box::new(Node(
          "C",
          Box::new(Leaf),
          Box::new(Node(
            "D",
            Box::new(Leaf),
            Box::new(Leaf))))))));

    let t2 = Node(
      "B",
      Box::new(Node("A", Box::new(Leaf), Box::new(Leaf))),
        Box::new(Node(
          "C",
          Box::new(Leaf),
          Box::new(Node(
            "D",
            Box::new(Leaf),
            Box::new(Leaf))))));

    t.rebalance();
    assert_eq!(t, t2);
  }

  #[test]
  fn rebalance3_test() {
    let mut t = Node(
      "E",
      Box::new(Node(
        "B",
        Box::new(Leaf),
        Box::new(Node(
          "D",
          Box::new(Node(
            "C", Box::new(Leaf), Box::new(Leaf))),
          Box::new(Leaf))))),
      Box::new(Node(
        "F", Box::new(Leaf), Box::new(Leaf))));

    let t2 = Node(
      "D",
      Box::new(Node(
        "B",
        Box::new(Leaf),
        Box::new(Node("C", Box::new(Leaf), Box::new(Leaf))))),
      Box::new(Node(
        "E",
        Box::new(Leaf),
        Box::new(Node("F", Box::new(Leaf), Box::new(Leaf))))));

    t.rebalance();
    assert_eq!(t, t2);
  }
}
