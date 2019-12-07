use crate::concurrent_set::ConcurrentSet;
use std::fmt::Debug;
use std::marker::{Unpin, PhantomData};
use std::cell::UnsafeCell;
use std::alloc::{alloc, Layout};
struct Node<T> {
    pub value : T,
    pub next : UnsafeCell<*mut Node<T>>
}

pub struct RluSet<T> {
  // data goes here
  head: UnsafeCell<*mut Node<T>>
  //_marker : PhantomData<T>
}

// In case you need raw pointers in your RluSet, you can assert that RluSet is definitely
// Send and Sync
unsafe impl<T> Send for RluSet<T> {}
unsafe impl<T> Sync for RluSet<T> {}

impl<T> RluSet<T> where T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin {
  pub fn new() -> RluSet<T> {
    RluSet {
        head: UnsafeCell::new(std::ptr::null_mut())
    }
  }


  pub fn to_string(&self) -> String {
    let mut res = String::from("head -> ");
    let mut temp = self.head.get();
    unsafe {
        while !(*temp).is_null() {
        
            let node = *temp;
            res.push_str(format!("{:?} -> ", (*node).value).as_str());
            temp = (*node).next.get(); 
        }
        res.push_str(format!("null").as_str());
    }
    res
  }
}

impl<T> ConcurrentSet<T> for RluSet<T> where T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin {
  fn contains(&self, value: T) -> bool {
        let mut temp = self.head.get();
        unsafe {
            while !(*temp).is_null() {
                let node = *temp;
                if (*node).value == value {
                    return true;
                }
                temp = (*node).next.get();
            }
        }
        false
  }

  fn len(&self) -> usize {
    let mut len : usize = 0;
    let mut temp = self.head.get();
        unsafe {
    while !(*temp).is_null() {
        len = len + 1;
            let node = *temp;
            temp = (*node).next.get();
        }
    }
    len
  }

  fn insert(&self, value: T) -> bool {
      unsafe {
   
    /*let new_node = Box::new(Node {
            value : value,
            next : UnsafeCell::new(*(self.head.get()))
        });
    let new_el = Box::into_raw(new_node);*/
    //*(self.head.get()) = new_el;
    
    //if want to allocate memory using alloc() API then
    // code is as following
    let layout = Layout::new::<Node<T>>();
    let ptr =  alloc(layout);
    let n_ptr : *mut Node<T> =  ptr as *mut Node<T>;
    (*n_ptr).value =  value;
    (*n_ptr).next = UnsafeCell::new(*(self.head.get())); 
    *(self.head.get()) = n_ptr;
    }
    true
  }

  fn delete(&self, value: T) -> bool {
    let mut temp = self.head.get();
    let mut prev = self.head.get();
    unsafe {
    let mut node = *temp;
        while !(*temp).is_null() {
            node = *temp;
            if (*node).value == value {
                break;
            }
            prev = temp;
            temp = (*node).next.get();
        }
        if node.is_null() {
            return false;
        }
        if node == *(self.head.get()) {
            // removing first node
            *(self.head.get()) = *((*node).next.get())  ;
        } else {
            let next = (*node).next.get();
            (*(*prev)).next = UnsafeCell::new(*next);
        }

    }
    true
  }

  fn clone_ref(&self) -> Self {
    unimplemented!()
  }
}
