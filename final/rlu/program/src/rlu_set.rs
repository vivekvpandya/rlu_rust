use crate::concurrent_set::ConcurrentSet;
use std::fmt::Debug;
use std::marker::{Unpin, PhantomData};
use std::cell::UnsafeCell;
use std::alloc::{alloc, Layout};
use std::thread;
use std::mem::{size_of};
use crate::rlu::{RLU_ALLOC, RLU_GET_THREAD_DATA, RLU_DEREF, RLU_ASSIGN_POINTER, RLU_READER_LOCK, RLU_READER_UNLOCK, RLU_TRY_LOCK,rlu_new_thread_data, RLU_THREAD_INIT, RLU_INIT, rlu_abort, RLU_FREE};

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
                    RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
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
    //println!("In set insert");
    if !self.contains(value) {
        let mut temp : *mut *mut Node<T> = self.head;
        unsafe {
            if (temp).is_null() {
                //println!("temp is null in insert");
                return  false; //Should be assert
            }

    //println!("In set insert {:?} {:?}", value, thread::current().id());
            let mut restart = true;
            let p_new_node : *mut Node<T> =  self.rlu_new_node();
            (*p_new_node).value = value;
             while (restart) {
                    RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
                        //println!("reader - lock is aquired");
                    if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut temp as *mut *mut *mut Node<T>) == 0) {
                        rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                        //RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));

                        //println!("reader - lock is failed");
                        
                    } else {
                        //println!("object lock is aquired");
                        restart = false;
                        let mut cur_first : *mut Node<T> = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), temp)); 
                        RLU_ASSIGN_POINTER((&mut(*p_new_node).next) as *mut *mut Node<T>, cur_first); 
                        RLU_ASSIGN_POINTER(temp, p_new_node); 
                        RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                        //println!("Element insertion is compete");
                        return true;
                    }
              }

         }
    }
   // RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
    return false;
  }
  
  fn delete(&self, value: T) -> bool {
    // return true;
    if !self.contains(value) {
        return false;
    }
   let mut restart = true;
   let mut do_loop = true;
   unsafe {
      while (restart) {
    //println!("In set delete {:?} {:?}", value, thread::current().id());
	RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
	restart = false;
	let mut p_prev = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), self.head));
        if p_prev.is_null() {
             break;
        }
	let mut p_next = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*p_prev).next);
        if (*p_prev).value == value {
            //let n = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*p_prev).next);
            // need to delete first element
            let mut head = self.head;
	    let mut n = (RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*p_prev).next));
            if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut head as  *mut *mut *mut  Node<T>) == 0) {
                    rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                    //goto restart;
                    restart = true;
                    do_loop = false;
                    
            } else if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut p_prev as *mut *mut  Node<T>) == 0) {
                    rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                    //goto restart;
                    restart = true;
                    do_loop = false;
                     
            } else {
                    RLU_ASSIGN_POINTER(head, n);
                    
                    RLU_FREE(RLU_GET_THREAD_DATA(self.tid), p_prev);
                    
                    RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                    
                    return true;

            }
        }
	while (do_loop) {
		//p_node = (node_t *)RLU_DEREF(self, p_next);
	        if p_next.is_null() {
                    break;
                }
		let v = (*p_next).value;
		
		if (v == value) {
                    let n = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*p_next).next);
                    
                    if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut p_prev) == 0) {
                            rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                            //goto restart;
                            restart = true;
                            break;
                    }
                    
                    if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut p_next) == 0) {
                            rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                            //goto restart;
                            restart = true;
                            break;
                    }
                    
                    RLU_ASSIGN_POINTER( &mut ((*p_prev).next), n);
                    
                    RLU_FREE(RLU_GET_THREAD_DATA(self.tid), p_next);
                    
                    RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                    
                    return true;
		}
		
		p_prev = p_next;
		p_next = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), ((*p_prev).next));
	 }
      }
        RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
        return false	
    }
 }



  /*fn delete(&self, value: T) -> bool {

    println!("In set delete");
    if !self.contains(value) {
        return false;
    }
    let mut head = self.head;
    //TODO: need to handle delete of list contains only one element
    unsafe {
       RLU_READER_LOCK(RLU_GET_THREAD_DATA(self.tid));
       let mut node : *mut Node<T> = *(RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), head)); 
       let mut prev = node; 
       let mut is_first = true;
       while !(node).is_null() {
           if (*node).value == value {
               if (is_first) {
                   if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid),  &mut head as  *mut *mut *mut  Node<T>) == 0) {
                       rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                       RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                       return false;
                        //goto restart;
                   }
                   if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut node as *mut *mut  Node<T>) == 0) {
                       rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                       RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                       return false;
                        //goto restart;
                   }
                   let n = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*node).next);
                   RLU_ASSIGN_POINTER(head, n); 
                   RLU_FREE(RLU_GET_THREAD_DATA(self.tid), node);
                   RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                   return true;

               }
	       if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid),  &mut prev as  *mut *mut  Node<T>) == 0) {
                   rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                   RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                   return false;
	            //goto restart;
	       }
	       if (RLU_TRY_LOCK(RLU_GET_THREAD_DATA(self.tid), &mut node as *mut *mut  Node<T>) == 0) {
                   rlu_abort(RLU_GET_THREAD_DATA(self.tid));
                   RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
                   return false;
	            //goto restart;
	       }
                
               println!("locks for delete are aquired");
               let n = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*node).next);
               RLU_ASSIGN_POINTER( &mut ((*prev).next) as *mut *mut Node<T>, n); 
	       RLU_FREE(RLU_GET_THREAD_DATA(self.tid), node);
               RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));
               println!("Element is deleted");
               return true;
           }
           prev = node; 
           is_first = false;
           node = RLU_DEREF(RLU_GET_THREAD_DATA(self.tid), (*node).next); 
       }
       RLU_READER_UNLOCK(RLU_GET_THREAD_DATA(self.tid));

    }
    false
  }*/

  fn clone_ref(&self) -> Self {
       let tid = RLU_THREAD_INIT(rlu_new_thread_data()); 
       RluSet {
           head: self.head,
           tid: tid
       }
  }
}
