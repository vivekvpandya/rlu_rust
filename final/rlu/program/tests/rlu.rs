
use std::alloc::{alloc, Layout};
use rlu::rlu_thread_data_t;
use rlu::rlu_alloc;
use rlu::rlu_free;
use std::mem::size_of;
use std::thread;

struct Node {
    value : u32,
    next : *mut Node
}

#[test]
fn rlu_basic() {
   let mut handles = Vec::new(); 
   for i in 0..5 {
        handles.push(thread::spawn(|| {
            
        unsafe {
            let layout = Layout::new::<rlu_thread_data_t>();
            let data = alloc(layout);
            let d_ptr : *mut rlu_thread_data_t = data as *mut rlu_thread_data_t;
            (*d_ptr).uniq_id = 100;
    
            let ptr = rlu_alloc(size_of::<Node>());
            let n_ptr : *mut Node = ptr as *mut Node;
            (*n_ptr).value = 100;
            (*n_ptr).next = std::ptr::null_mut();
            if (*n_ptr).value == 100 {
                println!("yes this works! from thread : {:?}", thread::current().id());
            }

            // rlu_free(std::ptr::null_mut(), n_ptr as *mut u32); this test works on Linux
        }
        }));
    }
    
    for handle in handles {
        handle.join();
    }
}
