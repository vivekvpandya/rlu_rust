use std::convert::TryInto;
// Some handy constants if you want fixed-size arrays of the relevant constructs.
const RLU_MAX_LOG_SIZE: usize = 128;
const RLU_MAX_THREADS: usize = 32;
const RLU_MAX_FREE_NODES: usize = 100;

struct rlu_data {
    n_starts :  u32,
    n_finish :  u32,
    n_writers : u32,

    n_writer_writeback :  u32,
    n_writeback_q_iters : u32,
    n_pure_reader :       u32,

    n_steals : u32,
    n_aborts : u32,

    n_sync_requests :      u32,
    n_sync_and_writeback : u32
}

struct rlu_obj_header {
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
    is_wait : u8,
    run_counter : u32
}

struct rlu_thread_data_t {
    //NOTE: we might also want to add padding as it may have impact on the performance
    uniq_id : u32,
    is_check_locks: char,
    is_write_detected : char,
    is_steal : char,
    type_ : u32,
    max_write_set : u32,

    run_counter : u32,
    local_version: u32,
    local_commit_version: u32,
    is_no_quiescence : u32,
    is_sync: u32,

    writer_version : u32,
    q_threads : [wait_entry_t; RLU_MAX_THREADS],

    ws_head_counter : u32,
    ws_wb_counter : u32,
    ws_tail_counter : u32,
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
    n_starts : 0,
    n_finish : 0,
    n_writers : 0,

    n_writer_writeback :  0,
    n_writeback_q_iters:  0,
    n_pure_reader :       0,

    n_steals : 0,
    n_aborts : 0,

    n_sync_requests :      0,
    n_sync_and_writeback : 0
};

static mut g_rlu_max_write_sets : u32 = 0;
static mut g_rlu_cur_threads : usize = 0;
static mut g_rlu_threads : [*mut rlu_thread_data_t; 32] = [0 as *mut rlu_thread_data_t; 32] ;
static mut g_rlu_writer_locks : [u32; 20000] = [0; 20000];
static mut g_rlu_array : [u32; 4096] = [0; 4096];

fn rlu_init( type_ : u32, max_write_sets: u32) {
    unimplemented!();
}

fn rlu_finish() {
    unimplemented!();
}

fn rlu_print_stats() {
    unimplemented!();
}

fn rlu_thread_init( self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_thread_finish(self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_alloc(obj_size : u32) -> *mut u32 {
    unimplemented!();
}

fn rlu_free(self_ : *mut rlu_thread_data_t, p_obj : *mut u32) {
    unimplemented!();
}

fn rlu_reader_lock(self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_reader_unlock(self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_try_lock(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut u32, obj_size :u32) -> u32 {
    unimplemented!();
}

fn rlu_abort(self_ : *mut rlu_thread_data_t) {
    unimplemented!();
}

fn rlu_init_quiescence(self_ : *mut rlu_thread_data_t) {
    //unimplemented!();
    unsafe {
	//MEMBARSTLD();
        // Need to model NULL for thread id
        let NULL = -1;
	for th_id in 0..g_rlu_cur_threads {

            (*self_).q_threads[th_id].is_wait = 0;

            if (th_id == (*self_).uniq_id.try_into().unwrap()) {
            // No need to wait for myself
                continue;
            }
            else if (g_rlu_threads[th_id].is_null()) {
                // No need to wait for uninitialized threads
                continue;
            } else {
                (*self_).q_threads[th_id].run_counter = (*g_rlu_threads[th_id]).run_counter;
                if ((*self_).q_threads[th_id].run_counter & 0x1 == 1) {
                    // The other thread is running -> wait for the thread
                     (*self_).q_threads[th_id].is_wait = 1;	
                 }
	    }
        }
    }
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
    unimplemented!();
}

fn rlu_sync_checkpoint(self_ : *mut rlu_thread_data_t) {
   // unimplemented!();
   unsafe {
        if ((*self_).is_sync == 0) {
                    return;
        }

        (*self_).n_sync_requests = (*self_).n_sync_requests + 1;
        rlu_sync_and_writeback(self_);
   }
}
