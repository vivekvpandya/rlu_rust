use std::{mem, fmt};
use std::fmt::{Display, Debug};

#[derive(PartialEq, Eq, Clone)]
pub enum BinaryTree<T> {
  Leaf,
  Node(T, Box<BinaryTree<T>>, Box<BinaryTree<T>>)
}

impl<T: Debug + Display + PartialOrd> BinaryTree<T> {
  pub fn len(&self) -> usize {
    match self {
        BinaryTree::Leaf => { 0 },
        BinaryTree::Node(t, l, r) => { 1 + l.len() + r.len()}
    }
  }

  pub fn to_vec(&self) -> Vec<&T> {
    let mut vec = Vec::new();
    match self {
        BinaryTree::Leaf => {vec},
        BinaryTree::Node(t, l, r) => { vec.append(&mut l.to_vec()); vec.push(t); vec.append(&mut r.to_vec()); vec } 
    }
  }

  pub fn sorted(&self) -> bool {
  let vec =  self.to_vec();
  let mut b : bool = true;
  for i in 0..vec.len() {
    if i > 0 && i < (vec.len() - 1) {
      b &= (vec[i] > vec[i-1] && vec[i] <= vec[i+1])  
    };
  };
  b
  }

  pub fn insert(&mut self, t: T) {
    match self {
        BinaryTree::Leaf => {*self = BinaryTree::Node(t, Box::new(BinaryTree::Leaf), Box::new(BinaryTree::Leaf))},
        BinaryTree::Node(tt, l, r) => { if *tt > t {l.insert(t)}
                                        else {r.insert(t)} }
    }
  }

  pub fn insert_tree<'a>(&'a mut self, tree : &'a mut BinaryTree<T>) {
    match self {
        BinaryTree::Leaf => {mem::swap(self, tree)},
        BinaryTree::Node(tt, l, r) => {
            match tree {
                BinaryTree::Leaf => {},
                BinaryTree::Node(t , _, _) => { if *tt > *t { l.insert_tree(tree)} else {r.insert_tree(tree)}} 
            }
        }
    }
  }

  pub fn search(&self, query: &T) -> Option<&T> {
    match self {
        BinaryTree::Leaf => {None},
        BinaryTree::Node(tt, l, r) => { if *tt < *query {r.search(query)}
                                        else {Some(tt)} }
    }
  }


  pub fn largest(&mut self) -> BinaryTree<T> {
    match self {
        BinaryTree::Leaf => {BinaryTree::Leaf},
        BinaryTree::Node(_, _, r) => { if r.len() == 0 {mem::replace(self, BinaryTree::Leaf)}  
                                       else {r.largest()} }
    }
  }


  pub fn smallest(&mut self) -> BinaryTree<T> {
    match self {
        BinaryTree::Leaf => {BinaryTree::Leaf},
        BinaryTree::Node(_, l, _) => { if l.len() == 0 {mem::replace(self, BinaryTree::Leaf)}
                                       else {l.smallest()}}
    }
  }

  pub fn rebalance(&mut self) {
        match self {
            BinaryTree::Leaf => {},
            BinaryTree::Node(t, l, r) => {
                    let ll = l.len();
                    let rl = r.len();
                    if (ll > rl) && (ll - rl) > 1 {
                        let lar_l =&mut l.largest();
                        match lar_l {
                            BinaryTree::Node(tt, lt, rt) => {let mut left1 = mem::replace(lt, Box::new(BinaryTree::Leaf)); 
                                                             let mut right1 = mem::replace(rt, Box::new(BinaryTree::Leaf));
                                                             l.insert_tree(&mut*left1);
                                                             mem::swap(lt, l);
                                                             let old_root : BinaryTree<T> = mem::replace(self, BinaryTree::Leaf);
                                                             mem::swap(rt , &mut Box::new(old_root));
                                                             mem::swap(self, lar_l);

                                                             //println!("Vec {:?}", self.to_vec());
                                                            // println!("Vec {:?}", lar_l.to_vec());
                                                             }
                            BinaryTree::Leaf => {}
                        }
                    }
                    else if (rl > ll) && (rl - ll) > 1 {
                        let sma_r = &mut r.smallest();
                        match sma_r {
                            BinaryTree::Node(tt, lt, rt) => {
                                    let mut left1 = mem::replace(lt, Box::new(BinaryTree::Leaf));
                                    let mut right1 = mem::replace(rt, Box::new(BinaryTree::Leaf));
                                    r.insert_tree(&mut*right1);
                                    mem::swap(rt, r);
                                    let old_root : BinaryTree<T> = mem::replace(self, BinaryTree::Leaf);
                                    mem::swap(lt, &mut Box::new(old_root));
                                    mem::swap(self, sma_r);

                                    //println!("Vec {:?}", self.to_vec());
                                    //println!("Vec {:?}", sma_r.to_vec());
                                }
                            BinaryTree::Leaf => {}
                        }
                    }
                }
        }
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
    //let vect = t.to_vec();
    //println!("{:?}", vect);
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
