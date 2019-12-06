use std::sync::atomic::{AtomicU32, Ordering};
use std::alloc::{alloc, Layout};
// Some handy constants if you want fixed-size arrays of the relevant constructs.
const RLU_MAX_LOG_SIZE: usize = 128;
const RLU_MAX_THREADS: usize = 32;
const RLU_MAX_FREE_NODES: usize = 100;
const RLU_MAX_WRITE_SETS : usize = 200;
struct rlu_data {
    n_starts :  AtomicU32,
    n_finish :  AtomicU32,
    n_writers : AtomicU32,

    n_writer_writeback : AtomicU32,
    n_writeback_q_iters : AtomicU32,
    n_pure_readers :    AtomicU32,

    n_steals : AtomicU32,
    n_aborts : AtomicU32,

    n_sync_requests :   AtomicU32,
    n_sync_and_writeback : AtomicU32
}

// Note: to access p_obj_copy we can use
// https://doc.rust-lang.org/std/primitive.pointer.html#method.offset-1
// with count = std::mem::size_of::<*mut u32>()
struct rlu_obj_header_t {
    p_obj: *mut u32,
    p_obj_copy : * mut u32
}

struct rlu_ws_obj_header {
    p_obj_actual : * mut u32, 
    obj_size : u32,
    run_counter : u32,
    thread_id : u32
}

struct writer_locks_t {
    size : usize,
    ids : [usize; 20] //array , size must be known at compile time, according to rust spec this lives on stack
}

struct obj_list_t {
    writer_locks : writer_locks_t,
    num_of_objs : u32,
    p_cur : * mut u32,
    buffer : [char; 100000]
}

struct wait_entry_t {
    is_wait : char,
    run_counter : u32
}

// NOTE: when ever a new instace of this struct is created it needs to be initialized
struct rlu_thread_data_t {
    //NOTE: we might also want to add padding as it may have impact on the performance
    uniq_id : u32,
    is_check_locks: char,
    is_write_detected : char,
    is_steal : char,
    type_ : u32,
    max_write_set : u32,

    run_counter : AtomicU32,
    local_version: u32,
    local_commit_version: u32,
    is_no_quiescence : u32,
    is_sync: u32,

    writer_version : u32,
    q_threads : [wait_entry_t; RLU_MAX_THREADS],

    ws_head_counter : usize,
    ws_wb_counter : usize,
    ws_tail_counter : usize,
    ws_cur_id : usize,
    obj_write_set : [obj_list_t; 200],
    free_nodes_size : usize,
    free_nodes : [*mut u32; 200],

    n_starts : u32,
    n_finish : u32,
    n_writers: u32,
    n_writer_writeback: u32,
    n_pure_readers : u32,
    n_aborts: u32,
    n_steals : u32,
    n_writer_sync_waits: u32,
    n_writeback_q_iters: u32,
    n_sync_requests: u32,
    n_sync_and_writeback: u32,
}
    
static mut g_rlu_data : rlu_data = rlu_data{
    n_starts : AtomicU32::new(0),
    n_finish : AtomicU32::new(0),
    n_writers : AtomicU32::new(0),

    n_writer_writeback : AtomicU32::new(0),
    n_writeback_q_iters: AtomicU32::new(0),
    n_pure_readers : AtomicU32::new(0),

    n_steals : AtomicU32::new(0),
    n_aborts : AtomicU32::new(0),

    n_sync_requests : AtomicU32::new(0),
    n_sync_and_writeback : AtomicU32::new(0)
};

static mut g_rlu_max_write_sets : u32 = 0;
static mut g_rlu_cur_threads : AtomicU32 =  AtomicU32::new(0);
static mut g_rlu_threads : [*mut rlu_thread_data_t; 32] = [0 as *mut rlu_thread_data_t; 32] ;
static mut g_rlu_writer_locks : [u32; 20000] = [0; 20000];
static mut g_rlu_array : [u32; 4096] = [0; 4096];

fn rlu_init( type_ : u32, max_write_sets: u32) {
    unsafe {
        g_rlu_array[64 * 2] = 0; // this is same as g_rlu_writer_version RLU_CACHE_LINE_SIZE = 64
        g_rlu_array[64 * 4] = 9; // this is same as g_rlu_commit_version  
    
        // I think we need to implement FINE GRAINED for now.
        g_rlu_max_write_sets = max_write_sets;
    }
}

fn rlu_finish() {
}

fn rlu_print_stats() {
    unsafe {
        println!("=================================================");
        println!("RLU statistics:");
        println!("-------------------------------------------------");
        println!("  t_starts = {:?}", g_rlu_data.n_starts);
        println!("  t_finish = {:?}", g_rlu_data.n_finish);
        println!("  t_writers = {:?}", g_rlu_data.n_writers);
        println!("-------------------------------------------------");
        println!("  t_writer_writebacks = {:?}", g_rlu_data.n_writer_writeback);
        println!("  t_writeback_q_iters = {:?}", g_rlu_data.n_writeback_q_iters);
        if (*(g_rlu_data.n_writer_writeback.get_mut()) > 0) {
            println!("  a_writeback_q_iters = {:?}", *(g_rlu_data.n_writeback_q_iters.get_mut()) / *(g_rlu_data.n_writer_writeback.get_mut()));
        } else {
            println!("  a_writeback_q_iters = 0");
        }
        println!("  t_pure_readers = {:?}", g_rlu_data.n_pure_readers);
        println!("  t_steals = {:?}", g_rlu_data.n_steals);
        println!("  t_aborts = {:?}", g_rlu_data.n_aborts);
        println!("  t_sync_requests = {:?}", g_rlu_data.n_sync_requests);
        println!("  t_sync_and_writeback = {:?}", g_rlu_data.n_sync_and_writeback);

        println!("=================================================");
    }
}

fn rlu_reset_write_set(self_: *mut rlu_thread_data_t, ws_counter : usize) {
    unimplemented!();
}

fn rlu_reset(self_: *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_commit_write_set(self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_release_writer_lock(self_ : *mut rlu_thread_data_t, writer_lock_id : usize) {
    unsafe {
        g_rlu_writer_locks[writer_lock_id] = 0;
    }
}

fn rlu_release_writer_locks(self_ : *mut rlu_thread_data_t, ws_id : usize) {
    let mut i : usize = 0;
    unsafe {
    for i in 0..(*self_).obj_write_set[ws_id].writer_locks.size {
        rlu_release_writer_lock(self_, (*self_).obj_write_set[ws_id].writer_locks.ids[i as usize]);
    }
    }
}

fn rlu_unlock_objs(self_: *mut rlu_thread_data_t, ws_counter : usize) {
    unimplemented!();
}

fn rlu_reset_writer_locks(self_: *mut rlu_thread_data_t, ws_id : usize) {
    unsafe {
        (*self_).obj_write_set[ws_id].writer_locks.size = 0;
    }
}

fn rlu_thread_init(self_ : *mut rlu_thread_data_t) {
    unsafe {
        (*self_).max_write_set = g_rlu_max_write_sets;
        (*self_).uniq_id = g_rlu_cur_threads.fetch_add(1, Ordering::SeqCst); // use fetch_and_add here
        (*self_).local_version = 0;
        (*self_).writer_version = u32::max_value();
        let mut ws_counter : usize = 0;
        for ws_couter in 0..RLU_MAX_WRITE_SETS {
            rlu_reset_write_set(self_, ws_counter);
        }
        g_rlu_threads[(*self_).uniq_id as usize] = self_;
        //NOTE: should we require something equivalent to __sync_synchromize()
    }
}

fn rlu_thread_finish(self_ : *mut rlu_thread_data_t) {
    unsafe {
        rlu_sync_and_writeback(self_);
        rlu_sync_and_writeback(self_); // I don't know why there are two calls to rlu_sync_and_writeback()
        g_rlu_data.n_starts.fetch_add((*self_).n_starts, Ordering::SeqCst);
        g_rlu_data.n_finish.fetch_add((*self_).n_finish, Ordering::SeqCst);
        g_rlu_data.n_writers.fetch_add((*self_).n_writers, Ordering::SeqCst);
        g_rlu_data.n_writer_writeback.fetch_add((*self_).n_writer_writeback, Ordering::SeqCst);
        g_rlu_data.n_writeback_q_iters.fetch_add((*self_).n_writeback_q_iters, Ordering::SeqCst);
        g_rlu_data.n_pure_readers.fetch_add((*self_).n_pure_readers, Ordering::SeqCst);
        g_rlu_data.n_steals.fetch_add((*self_).n_steals, Ordering::SeqCst);
        g_rlu_data.n_aborts.fetch_add((*self_).n_aborts, Ordering::SeqCst);
        g_rlu_data.n_sync_requests.fetch_add((*self_).n_sync_requests, Ordering::SeqCst);
        g_rlu_data.n_sync_and_writeback.fetch_add((*self_).n_sync_and_writeback, Ordering::SeqCst);
    }
}

fn is_ptr_copy(ptr :*mut u32) -> bool {
    unsafe {    
       if  (ptr as usize) == 0x12341234 {
            return true;
        }
    }
    false
}
fn rlu_alloc() -> *mut u32 {
    unsafe {
        let layout = Layout::new::<rlu_obj_header_t>();
        let ptr = alloc(layout);
        //TODO: check if alloc fails then handle it properly
        ptr as *mut u32
    }
}

fn rlu_free(self_ : *mut rlu_thread_data_t, p_obj : *mut u32) {
    unimplemented!();
}

fn rlu_register_thread(self_ : *mut rlu_thread_data_t) {
    unsafe {
        (*self_).run_counter.fetch_add(1, Ordering::SeqCst); // does this really need to be atomic in thread_data_t
        (*self_).local_version = g_rlu_array[64 * 2];
        (*self_).local_commit_version = g_rlu_array[64 * 4];
    }
}

fn rlu_unregister_thread(self_ : *mut rlu_thread_data_t) {
    unsafe {
        (*self_).run_counter.fetch_add(1, Ordering::SeqCst); // does this really need to be atomic in thread_data_t
    }
}

fn rlu_reader_lock(self_ : *mut rlu_thread_data_t) {
    unsafe {
    (*self_).n_starts = (*self_).n_starts + 1;
    rlu_sync_checkpoint(self_);
    rlu_reset(self_);
    rlu_register_thread(self_);
    (*self_).is_steal = 1 as char;
    if ((*self_).local_version - (*self_).local_commit_version) == 0 {
        (*self_).is_steal = 0 as char;
    }
    (*self_).is_check_locks = 1 as char;
    if (((*self_).local_version - (*self_).local_commit_version) == 0) && (((*self_).ws_tail_counter - (*self_).ws_wb_counter)) == 0{
    (*self_).is_check_locks = 0 as char;
    (*self_).n_pure_readers = (*self_).n_pure_readers + 1;
}
}
}

fn rlu_reader_unlock(self_ : *mut rlu_thread_data_t) {
    unsafe {
    (*self_).n_finish = (*self_).n_finish + 1;
    rlu_unregister_thread(self_);
    if (*self_).is_write_detected != 0 as char {
        (*self_).is_write_detected = 0 as char;
        rlu_commit_write_set(self_);
        rlu_release_writer_locks(self_, (((*self_).ws_tail_counter -1 ) % RLU_MAX_WRITE_SETS) as usize);
    } else {
        rlu_release_writer_locks(self_, (*self_).ws_cur_id);
        rlu_release_writer_locks(self_, (*self_).ws_cur_id);
    }
    rlu_sync_checkpoint(self_);
    }
}

fn rlu_try_lock(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut u32, obj_size :u32) -> u32 {
    unimplemented!();
}

fn rlu_abort(self_ : *mut rlu_thread_data_t) {
    unsafe {
        (*self_).n_aborts = (*self_).n_aborts + 1;
        rlu_unregister_thread(self_);
        if (*self_).is_write_detected != 0 as char{
            (*self_).is_write_detected = 0 as char;
            rlu_unlock_objs(self_, (*self_).ws_tail_counter);
            rlu_release_writer_locks(self_, (*self_).ws_cur_id);
            rlu_reset_write_set(self_, (*self_).ws_tail_counter);
        } else {
            rlu_release_writer_locks(self_, (*self_).ws_cur_id);
            rlu_reset_writer_locks(self_, (*self_).ws_cur_id);
        }
        rlu_sync_checkpoint(self_);
    }
}

fn rlu_init_quiescence(self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_writeback_write_sets_and_unlock(self_ : *mut rlu_thread_data_t) -> u32 {
    unimplemented!();
}

fn rlu_wait_for_quiescence(self_ : *mut rlu_thread_data_t, writer_version : u32) -> u32 {
    unimplemented!();
}

fn rlu_synchronize(self_ : *mut rlu_thread_data_t) {
    unsafe {
	if ((*self_).is_no_quiescence == 1) {
		return;
	}

	rlu_init_quiescence(self_);

	let q_iters = rlu_wait_for_quiescence(self_, (*self_).writer_version);

        (*self_).n_writeback_q_iters = (*self_).n_writeback_q_iters + q_iters;
    }
}

fn rlu_process_free(self_ : *mut rlu_thread_data_t) {
    unsafe{

	//TRACE_3(self, "start free process free_nodes_size = %ld.\n", self->free_nodes_size);

	for i  in 0..(*self_).free_nodes_size  {
		let p_obj = (*self_).free_nodes[i];

	/*	RLU_ASSERT_MSG(IS_UNLOCKED(p_obj),
			self, "object is locked. p_obj = %p th_id = %ld\n",
			p_obj, GET_THREAD_ID(p_obj));

		TRACE_3(self, "freeing: p_obj = %p, p_actual = %p\n",
			p_obj, (intptr_t *)OBJ_TO_H(p_obj));*/
            // TODO: Free in rust ?
	    //	free((intptr_t *)OBJ_TO_H(p_obj));
	}

        (*self_).free_nodes_size = 0;
    }
}

fn  rlu_sync_and_writeback(self_ : *mut rlu_thread_data_t) {
    //unimplemented!();
    unsafe {

        //RLU_ASSERT((self->run_counter & 0x1) == 0);

        if ((*self_).ws_tail_counter == (*self_).ws_head_counter) {
                return;
        }

        (*self_).n_sync_and_writeback = (*self_).n_sync_and_writeback + 1;

        let ws_num = (*self_).ws_tail_counter - (*self_).ws_wb_counter;

        (*self_).writer_version = g_rlu_array[128 * 2]  + 1;
       //TODO: implement fetch and add for g_rlu_array[128 * 2]
       //FETCH_AND_ADD(&g_rlu_writer_version, 1);


        rlu_synchronize(self_);

        let ws_wb_num = rlu_writeback_write_sets_and_unlock(self_);

        //RLU_ASSERT_MSG(ws_num == ws_wb_num, self, "failed: %ld != %ld\n", ws_num, ws_wb_num);
        // TODO:define max value
        //(*self_).writer_version = MAX_VERSION;

       //TODO: implement fetch and add for g_rlu_array[128 * 2]
       // FETCH_AND_ADD(&g_rlu_commit_version, 1);

        if ((*self_).is_sync == 1) {
            (*self_).is_sync = 0;
        }

        rlu_process_free(self_);
    }
}

fn rlu_add_writer_lock(self_ : *mut rlu_thread_data_t, writer_lock_id : usize) {
    //unimplemented!();
	unsafe {
	  let n_locks : usize = (*self_).obj_write_set[(*self_).ws_cur_id].writer_locks.size;
	   //for (i = 0; i < n_locks; i++) {
		//RLU_ASSERT(self->obj_write_set[self->ws_cur_id].writer_locks.ids[i] != writer_lock_id);
	  // }
	
          (*self_).obj_write_set[(*self_).ws_cur_id].writer_locks.ids[n_locks] = writer_lock_id;
          (*self_).obj_write_set[(*self_).ws_cur_id].writer_locks.size = 1 + (*self_).obj_write_set[(*self_).ws_cur_id].writer_locks.size; 
        }
}

fn LOCK_ID(th_id : u32) -> u32 {
    th_id + 1
}

fn rlu_try_acquire_writer_lock(self_ : *mut rlu_thread_data_t, writer_lock_id : usize) -> Option<u32> {
    unsafe {
        //TODO:  need to borrow mutbly
        let  cur_lock = g_rlu_writer_locks[writer_lock_id];
        let other_ptr   = &mut LOCK_ID((*self_).uniq_id);
        match cur_lock {
        0 => {
              // check for Ordering
                let val = {
                    unsafe {
                             //TODO need to implement compare_and_swap
                            //let pa: *mut u32 = &mut g_rlu_writer_locks[writer_lock_id];
                            //pa.compare_and_swap(0, other_ptr, Ordering::Relaxed);
                            0
                            }
                };
                if (val == 0) {
                   Some(1)
                } else {
                  None
                }

             },
        _ => None
    }
  }	
}

// Convert Bool into option type
// NOTE: used usize for writer_lock_id
fn rlu_try_write_lock(self_ : *mut rlu_thread_data_t, writer_lock_id : usize) -> u32 {
    //unimplemented!();
    // Valid only for coarse grain
    match (rlu_try_acquire_writer_lock(self_, writer_lock_id))
    {
        Some(x) => {
	            // rlu_add_writer_lock(self_, writer_lock_id);
                     1
                   }
        None    => {
                      0
                   }
    }
}


fn rlu_lock(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut u32, obj_size :u32) {
    //unimplemented!();
    rlu_try_lock(self_, p_p_obj, obj_size);
}

fn rlu_deref_slow_path(self_ : *mut rlu_thread_data_t, p_obj : *mut u32) -> *mut u32 {
    unimplemented!();
}

fn rlu_cmp_ptrs(p_obj_1 : *mut u32, p_obj_2 : *mut u32) -> u32 {
    unimplemented!();
}

fn rlu_assign_pointer(p_ptr: *mut *mut u32, p_obj : *mut u32) {
    /*unsafe {
        if !(p_obj.is_null()) {
            
        }
        *p_ptr = p_obj;
    }*/
    unimplemented!();
}

fn rlu_sync_checkpoint(self_ : *mut rlu_thread_data_t) {
    unsafe {
        if (*self_).is_sync == 0 {
            return;
        }
        (*self_).n_sync_requests = (*self_).n_sync_requests + 1;
        rlu_sync_and_writeback(self_);
    }
}
