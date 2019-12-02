use crate::concurrent_set::ConcurrentSet;
use std::fmt::Debug;
use std::marker::{Unpin, PhantomData};
use std::cell::RefCell;
/*struct Node<T> {
    pub value : T,
    pub next : *mut Node<T>
}*/

pub struct RluSet<T> {
  // data goes here
  //head: *mut Node<T>
  _marker : PhantomData<T>
}

// In case you need raw pointers in your RluSet, you can assert that RluSet is definitely
// Send and Sync
unsafe impl<T> Send for RluSet<T> {}
unsafe impl<T> Sync for RluSet<T> {}

impl<T> RluSet<T> where T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin {
  pub fn new() -> RluSet<T> {
    /*RluSet {
        head: std::ptr::null_mut()
    }*/
    unimplemented!()
  }


  pub fn to_string(&self) -> String {
    /*let mut res = String::from("head -> ");
    let mut temp = &self.head;
    while !temp.is_null() {
        unsafe {
            let node = *temp;
            res.push_str(format!("{:?} -> ", (*node).value).as_str());
            temp = &(*node).next; 
        }
    }
    res*/
    unimplemented!() 
  }
}

impl<T> ConcurrentSet<T> for RluSet<T> where T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin {
  fn contains(&self, value: T) -> bool {
        /*let mut temp = &self.head;
        while !temp.is_null() {
            unsafe {
                let node = *temp;
                if (*node).value == value {
                    return true;
                }
                temp = &(*node).next;
            }
        }
        false*/
        unimplemented!()
  }

  fn len(&self) -> usize {
    /*let mut len : usize = 0;
    let mut temp = &self.head;
    while !temp.is_null() {
        len = len + 1;
        unsafe {
            let node = *temp;
            temp = &(*node).next;
        }
    }
    len*/
    unimplemented!()
  }

  fn insert(&self, value: T) -> bool {
   /* let new_node = Box::new(Node {
            value : value,
            next : self.head
        });
    unsafe {
    let new_el = Box::into_raw(new_node);
    self.head = new_el;
    }
    true*/
    unimplemented!()

  }

  fn delete(&self, value: T) -> bool {
    unimplemented!()
  }

  fn clone_ref(&self) -> Self {
    unimplemented!()
  }
}
