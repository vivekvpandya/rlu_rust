use crate::concurrent_set::ConcurrentSet;
use std::fmt::Debug;
use std::marker::{Unpin, PhantomData};
use std::cell::UnsafeCell;
use std::alloc::{alloc, Layout};
use std::mem::{size_of};
use crate::rlu::{RLU_ALLOC, RLU_GET_THREAD_DATA, RLU_DEREF, RLU_ASSIGN_POINTER, RLU_READER_LOCK, RLU_READER_UNLOCK, RLU_TRY_LOCK,rlu_new_thread_data, RLU_THREAD_INIT, RLU_INIT, rlu_abort};

struct Node<T> {
    pub value : T,
    pub next : *mut Node<T>
    //pub next : UnsafeCell<*mut Node<T>>
}

pub struct RluSet<T> {
  // data goes here
  head: *mut *mut Node<T>,
  tid  : usize
  //_marker : PhantomData<T>
}

// In case you need raw pointers in your RluSet, you can assert that RluSet is definitely
// Send and Sync
unsafe impl<T> Send for RluSet<T> {}
unsafe impl<T> Sync for RluSet<T> {}

impl<T> RluSet<T> where T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin {

  unsafe fn rlu_new_node(&self) -> *mut Node<T> {
    unsafe {
        RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
        let p = RLU_ALLOC(size_of::<Node<T>>()) as *mut Node<T>;
        RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
        p
    }
  }
  pub fn new() -> RluSet<T> {
    // Need to init/global data here
       RLU_INIT(0, 1);
       let tid = RLU_THREAD_INIT(rlu_new_thread_data()); 
       RLU_READER_LOCK(RLU_GET_THREAD_DATA(tid));
       let p = RLU_ALLOC(size_of::<*mut Node<T>>()) as *mut *mut Node<T>;
       RLU_ASSIGN_POINTER(p, std::ptr::null_mut());
       RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(tid));

       RluSet {
           head: p,
           tid: tid
       }
  }


  pub fn to_string(&self) -> String {
    let mut res = String::from(" ");
    let mut temp = self.head;
    unsafe {
        if (*temp).is_null() {
            res.push_str("Empty list");
            return res
        }
         res.push_str("head -> ");

        RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
        let mut node : *mut Node<T> = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), temp)); 
        while !(node).is_null() {
            res.push_str(format!("{:?} -> ", (*node).value).as_str());
            node = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*node).next); 
        }
        RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
        res.push_str(format!("null").as_str());
    }
    res
  }
}

impl<T> ConcurrentSet<T> for RluSet<T> where T: PartialEq + PartialOrd + Copy + Clone + Debug + Unpin {
  fn contains(&self, value: T) -> bool {
        let mut temp = self.head;
        unsafe {
            if (temp).is_null() {
                return  false // should be assert
            }
            RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
            let mut node : *mut Node<T> = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), temp)); 
            while !(node).is_null() {
                if (*node).value == value {
                    return true;
                }
                node = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*node).next); 
            }
            RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
        }
        false
  }

  fn len(&self) -> usize {
    let mut len : usize = 0;
    let mut temp : *mut *mut Node<T> = self.head;
    unsafe {
        if (*temp).is_null() {
            return  0; 
        }
        RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
        let mut node : *mut Node<T> = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), temp)); 
        while !(node).is_null() {
             len = len + 1;
            node = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*node).next); 
        }
        RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
    }
    len
  }

  fn insert(&self, value: T) -> bool {
    println!("In set insert");
    if !self.contains(value) {
/*        unsafe {
       

        //if want to allocate memory using alloc() API then
        // code is as following
        let layout = Layout::new::<Node<T>>();
        let ptr =  alloc(layout);
        let n_ptr : *mut Node<T> =  ptr as *mut Node<T>;
        
        // With RLU:
        //let n_ptr : *mut Node<T> =  self.rlu_new_node();

        (*n_ptr).value =  value;
        // ?? Use RLU assignment : rlu_assign_pointer,rlu_deref_slow_path
        // tdata = get_or_init thread_data
        // RLU_READER_LOCK(tdata);
        // let p_new_node : *mut Node<T> =  self.rlu_new_node();
	// p_new_node->val = val;
	// p_head = (node_t *)RLU_DEREF(self, (p_list->p_head));
	//  while  (!RLU_TRY_LOCK(tdata, p_head)) {
	// RLU_ASSIGN_PTR(tdata, &(p_new_node->p_next), p_head);
	// TODO: Need to figure out how to handle head pointer
        // RLU_ASSIGN_PTR(tdata, &(p_prev->p_next), p_new_node);
        // *(self.head.get()) = n_ptr;
        //
	// RLU_READER_UNLOCK(self);
        (*n_ptr).next = UnsafeCell::new(*(self.head.get())); 
        *(self.head.get()) = n_ptr;
        }
        true
    } else {
       false
    }
*/
        let mut temp : *mut *mut Node<T> = self.head;
        unsafe {
            if (temp).is_null() {
                println!("temp is null in insert");
                return  false; //Should be assert
            }
            let p_new_node : *mut Node<T> =  self.rlu_new_node();
	    (*p_new_node).value = value;
            RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
                println!("reader - lock is aquired");
            if RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut temp as *mut *mut *mut Node<T>) != 0 {
                println!("object lock is aquired");
                let mut cur_first : *mut Node<T> = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), temp)); 
                RLU_ASSIGN_POINTER((&mut(*p_new_node).next) as *mut *mut Node<T>, cur_first); 
                RLU_ASSIGN_POINTER(temp, p_new_node); 
                RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));

                return true;
            } else {
                println!("Try lock unsuccesful in insert");
                rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                return false; // TODO: need to hnadle abort
            }
            
            RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
            //RLU_READER_LOCK(tdata);
            return false;

        }
    }
    return false;
  }

  fn delete(&self, value: T) -> bool {
    //let mut temp = self.head.get();
    //let mut prev = self.head.get();
    /*unsafe {
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

    }*/
    true
  }

  fn clone_ref(&self) -> Self {
    unimplemented!()
  }
}
