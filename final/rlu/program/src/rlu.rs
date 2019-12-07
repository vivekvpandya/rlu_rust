use std::sync::atomic::{AtomicU32, AtomicPtr, AtomicUsize,  Ordering};
use std::alloc::{alloc, Layout};
use std::mem::{size_of};
use std::convert::TryInto;
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
    p_obj_copy : AtomicPtr<u32>,
    obj : u32
}

struct rlu_ws_obj_header_t {
    p_obj_actual : * mut u32, 
    obj_size : usize,
    run_counter : usize,
    thread_id : usize
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
    run_counter : usize
}

// NOTE: when ever a new instace of this struct is created it needs to be initialized
struct rlu_thread_data_t {
    //NOTE: we might also want to add padding as it may have impact on the performance
    uniq_id : usize,
    is_check_locks: char,
    is_write_detected : char,
    is_steal : char,
    type_ : u32,
    max_write_set : usize,

    run_counter : AtomicUsize,
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

static mut g_rlu_max_write_sets : usize = 0;
static mut g_rlu_cur_threads : AtomicUsize =  AtomicUsize::new(0);
//static mut g_rlu_cur_threads : usize = 0;
static mut g_rlu_threads : [*mut rlu_thread_data_t; 32] = [0 as *mut rlu_thread_data_t; 32] ;
static mut g_rlu_writer_locks : [u32; 20000] = [0; 20000];
static mut g_rlu_array : [u32; 4096] = [0; 4096];

fn rlu_init( type_ : u32, max_write_sets: usize) {
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


fn rlu_reset(self_: *mut rlu_thread_data_t) {
    unsafe {
    (*self_).is_write_detected = 0 as char;
    (*self_).is_steal = 1 as char;
    (*self_).is_check_locks =1 as char;
    }
}


fn rlu_commit_write_set(self_ : *mut rlu_thread_data_t) {
    unsafe {
    (*self_).n_writers = (*self_).n_writers + 1;
    (*self_).n_writer_writeback = (*self_).n_writer_writeback + 1;
    (*self_).ws_tail_counter = (*self_).ws_tail_counter + 1;

    
    (*self_).ws_cur_id = (*self_).ws_tail_counter % RLU_MAX_WRITE_SETS;
    let cond1 = ((*self_).ws_tail_counter % RLU_MAX_WRITE_SETS) == ((*self_).ws_head_counter % RLU_MAX_WRITE_SETS);
    let cond2 = ((*self_).ws_tail_counter - (*self_).ws_wb_counter) >= (*self_).max_write_set ;
    if cond1 || cond2 {
        rlu_sync_and_writeback(self_);
    } 
    }
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

//Vivek
fn rlu_unlock_objs(self_: *mut rlu_thread_data_t, ws_counter : usize) {
    unimplemented!();
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
        let mut p_obj_h = ptr as *mut rlu_obj_header_t;
        (*p_obj_h).p_obj_copy.store(0 as *mut u32, Ordering::SeqCst); // initialize to 0
        // ptr is *mut u8, should we cast it to *mut u32 before following operation?
        let p_obj = ptr.add(size_of::<usize>()); // size_of pointer == size_of usize, this should skip *u32 and point to actual obj  
        //TODO: check if alloc fails then handle it properly
        p_obj as *mut u32
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

fn rlu_add_ws_obj_header_to_write_set(self_ : *mut rlu_thread_data_t, p_obj : *mut u32, obj_size : usize) -> *mut u32 {
    // !!!! I am highly doubtful if this will work or not
    unsafe {
        let mut p_cur = (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur;
        let mut p_ws_obj_h = p_cur as *mut rlu_ws_obj_header_t;
        (*p_ws_obj_h).p_obj_actual = p_obj;
        (*p_ws_obj_h).obj_size = obj_size;
        (*p_ws_obj_h).run_counter = *(*self_).run_counter.get_mut();
        (*p_ws_obj_h).thread_id = (*self_).uniq_id;
        let offset = size_of::<rlu_ws_obj_header_t>();
        p_cur = p_cur.add(offset);
        let mut p_obj_h = p_cur as *mut rlu_obj_header_t;
        (*p_obj_h).p_obj_copy.store(0x12341234 as *mut u32, Ordering::SeqCst);
        p_cur = p_cur.add(size_of::<usize>());
        (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur = p_cur;
        return p_cur as *mut u32;
    }
}

fn rlu_add_obj_copy_to_write_set(self_ : *mut rlu_thread_data_t, p_obj : *mut u32, obj_size_t: usize) {
    // !!!! I hope this works, I really doubt
    unsafe {
        let mut p_cur = (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur;
        (*p_cur) = (*p_obj); // copy u32 value
        p_cur = p_cur.add(size_of::<u32>());
        (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur = p_cur;
        // increment num_of_objs
        (*self_).obj_write_set[(*self_).ws_cur_id as usize].num_of_objs = (*self_).obj_write_set[(*self_).ws_cur_id as usize].num_of_objs + 1;
    }
}

fn rlu_send_sync_request(th_id : usize) {
    unsafe {
        (*(g_rlu_threads[th_id])).is_sync = (*(g_rlu_threads[th_id])).is_sync + 1;
        //MEMBARSTLD();
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

fn rlu_try_lock(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut u32, obj_size :usize) -> u32 {
    // !!!!! this is incomplete
    unsafe {
    let mut p_obj = *p_p_obj;
        let mut p_obj_copy = p_obj.sub(size_of::<usize>());
        if (p_obj_copy as usize) == 0x12341234 {
            // this is copy
            // TODO:get actaul obj pointer from write_set
            // and get copy pointer on that and set them to p_obj and p_obj_copy
            p_obj = p_obj.sub(size_of::<rlu_ws_obj_header_t>()); // !!! really doubt if this is correct 
            p_obj_copy = p_obj.sub(size_of::<usize>()); // !!! 
        }

        //check if p_obj_copy pointer is locked
        if !p_obj_copy.is_null() {
            let mut p_ws_obj_h = p_obj_copy.sub(size_of::<rlu_ws_obj_header_t>()) as *mut rlu_ws_obj_header_t; // in C implementation they move back by size of ws_header + obj_header I don't understand why? I may be wrong here.
            let th_id : usize = (*p_ws_obj_h).thread_id;
            if th_id == (*self_).uniq_id {
                if *(*self_).run_counter.get_mut() == (*p_ws_obj_h).run_counter {
                    // p_obj us already locked by current execution of this thread.
                    // return copy
                    *p_p_obj = p_obj_copy;
                    return 1;
                }
                // p_obj is locked by another execution of this thread.
                (*self_).is_sync = (*self_).is_sync + 1;
                return 0;
            }
            // p_obj already locked by another thread.
            // sedn sync request to other thread
            // in the mean time sync this thread
            rlu_send_sync_request(th_id);
            (*self_).is_sync = (*self_).is_sync + 1;
            return 0;
        }
        
        // p_obj is free.
        // Indicate that write-set is updated
        if (*self_).is_write_detected == (0 as char) {
            (*self_).is_write_detected = (1 as char);
            (*self_).is_check_locks = (1 as char);
        }

        // add write-set header for the object
        p_obj_copy = rlu_add_ws_obj_header_to_write_set(self_, p_obj, obj_size);
        let p_copy_ = p_obj.sub(size_of::<usize>());
        let p_copy_atomic = AtomicUsize::new(p_copy_ as usize); // I don't know if this is correct way to update raw pointer atomically 
        // how do I know if the address will be 0? in C they set pointer to NULL
        // we must also use 0x0 instead of null_ptr.
        match p_copy_atomic.compare_exchange(0, p_obj_copy as usize, Ordering::Acquire, Ordering::Relaxed) {
            Ok(_) => {},
            Err(_) => {return 0;}
        }

        // locked successfully 
        // Copy object to write-set
        rlu_add_obj_copy_to_write_set(self_, p_obj, obj_size);
        *p_p_obj = p_obj_copy;
         return 1;

    }
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
    //unimplemented!();
    unsafe {
	//MEMBARSTLD();
        // Need to model NULL for thread id
        let NULL = -1;
        let cur_rlu_thread = *g_rlu_cur_threads.get_mut();
	for th_id in 0..cur_rlu_thread {

            (*self_).q_threads[th_id as usize].is_wait = 0;

            if (th_id == (*self_).uniq_id.try_into().unwrap()) {
            // No need to wait for myself
                continue;
            }
            else if (g_rlu_threads[th_id as usize].is_null()) {
                // No need to wait for uninitialized threads
                continue;
            } else {
                (*self_).q_threads[th_id as usize].run_counter = *(*g_rlu_threads[th_id as usize]).run_counter.get_mut();
                if ((*self_).q_threads[th_id as usize].run_counter & 0x1 == 1) {
                    // The other thread is running -> wait for the thread
                     (*self_).q_threads[th_id as usize].is_wait = 1;	
                 }
	    }
        }
    }
}

fn rlu_reset_writer_locks(self_ : *mut rlu_thread_data_t, ws_id : usize) {
    unsafe {
        (*self_).obj_write_set[ws_id].writer_locks.size = 0;
    }
}

fn rlu_reset_write_set(self_ : *mut rlu_thread_data_t, ws_counter : usize) {
    unsafe {
        // TODO: #define WS_INDEX(ws_counter) ((ws_counter) % RLU_MAX_WRITE_SETS)
        let ws_id = 0; //Assuming single write set

        (*self_).obj_write_set[ws_id].num_of_objs = 0;
        // TODO:Assigning har pointer to integer.. need to figure out better way
        //(*self_).obj_write_set[ws_id].p_cur = { (&(*self_).obj_write_set[ws_id].buffer[0]) as *mut u32};

	rlu_reset_writer_locks(self_, ws_id);
    }
}

fn rlu_writeback_write_set(self_ : *mut rlu_thread_data_t, ws_counter: usize) {

        unsafe {
            /*TODO: need to think this routine, it has pointer manipulation
             *      can encapsulated in the functions 
            //ws_id = WS_INDEX(ws_counter);
            // TODO: #define WS_INDEX(ws_counter) ((ws_counter) % RLU_MAX_WRITE_SETS)
            let ws_id = 0; //Assuming single write set
            //p_cur = (intptr_t *)&(self->obj_write_set[ws_id].buffer[0]);

            for i in  0..(*self_).obj_write_set[ws_id].num_of_objs {
                p_ws_obj_h = (rlu_ws_obj_header_t *)p_cur;

                p_obj_actual = (intptr_t *)p_ws_obj_h->p_obj_actual;
                obj_size = (obj_size_t)p_ws_obj_h->obj_size;

                p_cur = MOVE_PTR_FORWARD(p_cur, WS_OBJ_HEADER_SIZE);
                p_obj_h = (rlu_obj_header_t *)p_cur;

                RLU_ASSERT(p_obj_h->p_obj_copy == PTR_ID_OBJ_COPY);

                p_cur = MOVE_PTR_FORWARD(p_cur, OBJ_HEADER_SIZE);

                p_obj_copy = (intptr_t *)p_cur;

                TRACE_2(self, "[%ld] rlu_writeback_and_unlock: copy [%p] <- [%p] [%zu]\n",
                        self->writer_version, p_obj_actual, p_obj_copy, obj_size);

                memcpy((unsigned char *)p_obj_actual, (unsigned char *)p_obj_copy, obj_size);

                p_cur = MOVE_PTR_FORWARD(p_cur, ALIGN_OBJ_SIZE(obj_size));

                /*RLU_ASSERT_MSG(GET_THREAD_ID(p_obj_actual) == self->uniq_id,
                        self, "th_id = %ld my_id = %ld\n p_obj_actual = %p num_of_objs = %u\n",
                        GET_THREAD_ID(p_obj_actual), self->uniq_id, p_obj_actual, self->obj_write_set[ws_id].num_of_objs);*/

                UNLOCK(p_obj_actual);
                }
	//RLU_ASSERT(p_cur == self->obj_write_set[ws_id].p_cur);
        */
    }
}

// Advait next push will have these functions
fn rlu_writeback_write_sets_and_unlock(self_ : *mut rlu_thread_data_t) -> u32 {
    unsafe{
	for  ws_counter  in (*self_).ws_head_counter..(*self_).ws_wb_counter {
	     rlu_reset_write_set(self_, ws_counter.try_into().unwrap());
	}

        (*self_).ws_head_counter = (*self_).ws_wb_counter;

	let mut ws_wb_num = 0;
	for ws_counter in (*self_).ws_wb_counter..(*self_).ws_tail_counter {
	    rlu_writeback_write_set(self_, ws_counter.try_into().unwrap());
	    ws_wb_num = ws_wb_num + 1;
	}

        (*self_).ws_wb_counter = (*self_).ws_tail_counter;

	ws_wb_num
    }
}

fn rlu_wait_for_quiescence(self_ : *mut rlu_thread_data_t, version_limit : u32) -> u32 {
    unsafe {
	let mut iters = 0;
	let cur_threads = *g_rlu_cur_threads.get_mut();
	for th_id in 0..cur_threads {
    	    while ((*self_).q_threads[th_id as usize].is_wait != 1) {
		iters = iters + 1;
		if ((*self_).q_threads[th_id as usize].run_counter != *(*g_rlu_threads[th_id as usize]).run_counter.get_mut()) {
                    (*self_).q_threads[th_id as usize].is_wait = 0;
		    break;
                }
		if (version_limit != 0) {
                    if ((*g_rlu_threads[th_id as usize]).local_version >= version_limit) {
                        (*self_).q_threads[th_id as usize].is_wait = 0;
		        break;
		    }
	        }
                let Q_ITERS_LIMIT = 100000000;
		if (iters > Q_ITERS_LIMIT) {
                    iters = 0;
		    /*printf("[%ld] waiting for [%d] with: local_version = %ld , run_cnt = %ld\n", self->uniq_id, th_id,
                    (*g_rlu_threads[th_id]).local_version, (*g_rlu_threads[th_id]).run_counter);*/
		}
                //TODO: Check for CPU relax
 		//CPU_RELAX();

		}
	}

	iters
    }
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

fn LOCK_ID(th_id : usize) -> usize {
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


fn rlu_lock(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut u32, obj_size :usize) {
    //unimplemented!();
    rlu_try_lock(self_, p_p_obj, obj_size);
}

fn rlu_deref_slow_path(self_ : *mut rlu_thread_data_t, p_obj : *mut u32) -> *mut u32 {
    unimplemented!();
}

// this functions seems never used in original source, so may be okay to skip
// if in any case we want to return -1, 0, 1 we need to change return type
fn rlu_cmp_ptrs(p_obj_1 : *mut u32, p_obj_2 : *mut u32) -> bool {
    unsafe {
    let mut ptr_obj_1 = p_obj_1;
    let mut ptr_obj_2 = p_obj_2;
    if !(p_obj_1.is_null()) {
        if (ptr_obj_1 as usize) == 0x12341234 {
            ptr_obj_1 = ptr_obj_1.add(size_of::<usize>());
        }
    }
    if !(p_obj_1.is_null()) {
        if  (ptr_obj_2 as usize ) == 0x12341234 {
            ptr_obj_2 = ptr_obj_2.add(size_of::<usize>());
        }   
    }

    return (ptr_obj_1 as usize) == (ptr_obj_2 as usize);
    }
}

fn rlu_assign_pointer(p_ptr: *mut *mut u32, p_obj : *mut u32) {
    let mut ptr = p_obj;
    unsafe {
        if !(p_obj.is_null()) {
            if (p_obj as usize) == 0x12341234 {
                // pointer add and get to actaul obj as per rlu_obj_header_t struct
                ptr = p_obj.add(size_of::<usize>());
            }   
        }
        *p_ptr = ptr;
    }
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
