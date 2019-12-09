use std::sync::atomic::{AtomicU32, AtomicPtr, AtomicUsize,  Ordering};
use std::alloc::{alloc, Layout};
use std::mem::{size_of};
use std::convert::TryInto;
extern crate libc;
use std::ptr;
use std::mem;
use arr_macro::arr;
// Some handy constants if you want fixed-size arrays of the relevant constructs.
const RLU_MAX_LOG_SIZE: usize = 128;
const RLU_MAX_THREADS: usize = 32;
const RLU_MAX_FREE_NODES: usize = 100;
const RLU_MAX_WRITE_SETS : usize = 200;
const RLU_MAX_WRITE_SET_BUFFER_SIZE : usize = 100000;
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
    buffer : [char; RLU_MAX_WRITE_SET_BUFFER_SIZE]
}

struct wait_entry_t {
    is_wait : u8,
    run_counter : usize
}

// NOTE: when ever a new instace of this struct is created it needs to be initialized
pub struct rlu_thread_data_t {
    //NOTE: we might also want to add padding as it may have impact on the performance
    pub uniq_id : usize,
    is_check_locks: u8,
    is_write_detected : u8,
    is_steal : u8,
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
static mut g_rlu_writer_locks : [AtomicUsize; 20000] = arr![AtomicUsize::new(0); 20000];
//static mut g_rlu_array : [AtomicUsize; 4096] =  arr![AtomicUsize::new(0); 4096];
//static mut g_rlu_array : [AtomicUsize; 4096] = [0; 4096];
static mut g_rlu_writer_version : AtomicUsize = AtomicUsize::new(0);
static mut g_rlu_commit_version : AtomicUsize = AtomicUsize::new(9);

macro_rules! RLU_ASSERT {
    ($input:expr) => {
        unsafe {
            assert!($input);
        }
    }
}

pub fn rlu_get_thread_data(uniq_id : usize) -> *mut rlu_thread_data_t {
    if uniq_id > 31 {
        panic!("maximum 32 thread allowed, i.e uniq_id must be in range 0..31");
    }
    unsafe {
        return g_rlu_threads[uniq_id];
    }
}
pub fn rlu_new_thread_data() -> *mut rlu_thread_data_t {
    unsafe {
        let layout = Layout::new::<rlu_thread_data_t>();
        let ptr = alloc(layout);
        let thp = ptr as *mut rlu_thread_data_t;
        (*thp).uniq_id = 0;
        (*thp).is_check_locks = 0;
        (*thp).is_write_detected = 0;
        (*thp).is_steal = 0;
        (*thp).type_ = 0;
       
        
        (*thp).max_write_set = 0;
        (*thp).run_counter = AtomicUsize::new(0);
        (*thp).local_version = 0;
        (*thp).local_commit_version = 0;
        (*thp).is_no_quiescence = 0;
        (*thp).is_sync =0;
        // NOTE: do we need to initalize q_threads, obj_write_set, free_nodes array? I expecte they
        // should have be allocated just have some garbage value, we can just use it without
        // initializing them to some default value.         
        (*thp).writer_version = 0;
        //(*thp).q_threads need loop
        (*thp).ws_head_counter= 0;
        (*thp).ws_wb_counter = 0;
        (*thp).ws_tail_counter = 0;
        //(*thp).obj_write_set may be need loop
        (*thp).free_nodes_size = 0;
        //(*thp).free_nodes need loop
        
        (*thp).n_starts = 0;
        (*thp).n_finish = 0;
        (*thp).n_writers = 0;
        (*thp).n_writer_writeback = 0;
        (*thp).n_pure_readers = 0;
        (*thp).n_aborts = 0;
        (*thp).n_steals = 0;
        (*thp).n_writer_sync_waits = 0;
        (*thp).n_writeback_q_iters = 0;
        (*thp).n_sync_requests = 0;
        (*thp).n_sync_and_writeback = 0;
        return thp;
    }

}
fn rlu_init( type_ : u32, max_write_sets: usize) {
    unsafe {
        //g_rlu_array[64 * 2] = AtomicUsize::new(0); // this is same as g_rlu_writer_version RLU_CACHE_LINE_SIZE = 64
        //g_rlu_array[64 * 4] = AtomicUsize::new(9); // this is same as g_rlu_commit_version  
          g_rlu_writer_version  = AtomicUsize::new(0);
          g_rlu_commit_version  = AtomicUsize::new(0);
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
    (*self_).is_write_detected = 0;
    (*self_).is_steal = 1;
    (*self_).is_check_locks = 1;
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
        g_rlu_writer_locks[writer_lock_id].store(0, Ordering::Relaxed);
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
    unsafe{
    let mut i : usize = 0;
    let mut ws_id : usize = WS_INDEX(ws_counter);
    let mut obj_size : usize = 0;
    
    let mut p_cur = &mut((*self_).obj_write_set[ws_id].buffer[0]) as *mut char;

    let mut p_obj_actual : *mut u32 = std::ptr::null_mut();
    let mut p_ws_obj_h : *mut rlu_ws_obj_header_t = std::ptr::null_mut();
    let mut p_obj_h : *mut rlu_obj_header_t = std::ptr::null_mut();

    for i in 0..(*self_).obj_write_set[ws_id].num_of_objs {
        p_ws_obj_h = p_cur as *mut rlu_ws_obj_header_t;
        p_obj_actual = (*p_ws_obj_h).p_obj_actual;
        obj_size = (*p_ws_obj_h).obj_size;
        p_cur = MOVE_PTR_FORWARD(p_cur, size_of::<rlu_ws_obj_header_t>());
        p_obj_h = p_cur as *mut rlu_obj_header_t;

        RLU_ASSERT!(((*p_obj_h).p_obj_copy.load(Ordering::Relaxed) as usize) == 0x12341234 );

        p_cur = MOVE_PTR_FORWARD(p_cur, size_of::<rlu_obj_header_t>());

        RLU_ASSERT!(GET_COPY(p_obj_actual) == p_cur as *mut u32);

        p_cur = MOVE_PTR_FORWARD(p_cur, ALIGN_OBJ_SIZE(obj_size));

        UNLOCK(p_obj_actual); 
    }
    }

}

pub fn rlu_thread_init(self_ : *mut rlu_thread_data_t) ->  usize {
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
        //NOTE: should we require something equivalent to __sync_synchronize()
        // If C code has full memory barrier (i.e __sync_synchronize()) for above atomic fetch and
        // add then we might want to skip this because we are already using atomic operatioin with
        // Ordering:SeqCst.
        // But in any case we neeed something similar this link
        // https://www.reddit.com/r/rust/comments/5yszdc/an_equivalent_for_sync_synchronize_in_rust/
        // has good information also https://doc.rust-lang.org/nomicon/atomics.html this has nice
        // information on what Ordering we should use with our atomic operations, for now we have
        // used Ordering::SeqCst that is strict ordering but if see any perf problem and we are
        // sure that certain fetch_and_add operations can be done in any order and still produce
        // correct result we may want to change to Ordering::Relaxed. 
        (*self_).uniq_id
    }
}

fn rlu_thread_finish(self_ : *mut rlu_thread_data_t) {
    // In case of perf problem above note applies here too.
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

fn is_ptr_copy<T>(ptr :*mut T) -> bool {
    unsafe {    
       if  (ptr as usize) == 0x12341234 {
            return true;
        }
    }
    false
}

pub fn rlu_alloc(obj_size : usize) -> *mut u8 {
    unsafe {
        let ptr : *mut u8 = libc::malloc(size_of::<rlu_obj_header_t>() + obj_size) as *mut u8;
        if ptr.is_null() {
            panic!("failed to allocate memory");
        }
        let mut p_obj_h : *mut rlu_obj_header_t = ptr as *mut rlu_obj_header_t;
        (*p_obj_h).p_obj_copy = AtomicPtr::new(std::ptr::null_mut()); // 

        return H_TO_OBJ(p_obj_h) as *mut u8;
    }
}

pub fn rlu_free<T>(self_ : *mut rlu_thread_data_t, p_obj : *mut T) {
    unsafe {
    if p_obj.is_null() {
        return;
    }

    if self_.is_null() {
        libc::free(OBJ_TO_H(p_obj) as *mut libc::c_void);
        return;
    }

    //rlu_assert_in_section(self);

    let mut p_obj = FORCE_ACTUAL(p_obj);
    
    (*self_).free_nodes[(*self_).free_nodes_size] = p_obj as *mut u32;
    (*self_).free_nodes_size = (*self_).free_nodes_size +  1;

    RLU_ASSERT!((*self_).free_nodes_size < RLU_MAX_FREE_NODES);
    }
}

fn rlu_register_thread(self_ : *mut rlu_thread_data_t) {
    // NOTE: For perf we might want to change Ordering::SeqCst to Ordering::Relaxed
    unsafe {
        (*self_).run_counter.fetch_add(1, Ordering::SeqCst); // does this really need to be atomic in thread_data_t
        //(*self_).local_version = *(g_rlu_array[64 * 2].get_mut()) as u32;
        (*self_).local_version = *(g_rlu_writer_version.get_mut()) as u32;
        (*self_).local_commit_version = *(g_rlu_commit_version.get_mut()) as u32;
    }
}

fn rlu_unregister_thread(self_ : *mut rlu_thread_data_t) {
    // NOTE: For perf we might want to change Ordering::SeqCst to Ordering::Relaxed
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
        (*self_).is_steal = 1;
        if ((*self_).local_version - (*self_).local_commit_version) == 0 {
            (*self_).is_steal = 0 ;
        }
        (*self_).is_check_locks = 1;
        if (((*self_).local_version - (*self_).local_commit_version) == 0) && (((*self_).ws_tail_counter - (*self_).ws_wb_counter)) == 0{
            (*self_).is_check_locks = 0;
            (*self_).n_pure_readers = (*self_).n_pure_readers + 1;
        }
    }
}

fn rlu_add_ws_obj_header_to_write_set<T>(self_ : *mut rlu_thread_data_t, p_obj : *mut T, obj_size : usize) -> *mut T {
    // !!!! I am highly doubtful if this will work or not
    unsafe {
        let mut p_cur = (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur;
        let mut p_ws_obj_h = p_cur as *mut rlu_ws_obj_header_t;
        (*p_ws_obj_h).p_obj_actual = p_obj as *mut u32;
        (*p_ws_obj_h).obj_size = obj_size;
        (*p_ws_obj_h).run_counter = *(*self_).run_counter.get_mut();
        (*p_ws_obj_h).thread_id = (*self_).uniq_id;
        let offset = size_of::<rlu_ws_obj_header_t>();
        p_cur = MOVE_PTR_FORWARD(p_cur, offset);
        let mut p_obj_h = p_cur as *mut rlu_obj_header_t;
        (*p_obj_h).p_obj_copy.store(0x12341234 as *mut u32, Ordering::SeqCst);
        p_cur = MOVE_PTR_FORWARD(p_cur, size_of::<rlu_obj_header_t>());
        (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur = p_cur;
        return p_cur as *mut T;
    }
}

fn rlu_add_obj_copy_to_write_set<T>(self_ : *mut rlu_thread_data_t, p_obj : *mut T, obj_size: usize) {
    // !!!! I hope this works, I really doubt
    unsafe {
        let mut p_cur = (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur;
        ptr::copy(p_obj as *const char, p_cur as *mut char, obj_size);
        p_cur = MOVE_PTR_FORWARD(p_cur, ALIGN_OBJ_SIZE(obj_size));
        (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur = p_cur;
        // increment num_of_objs
        (*self_).obj_write_set[(*self_).ws_cur_id as usize].num_of_objs = (*self_).obj_write_set[(*self_).ws_cur_id as usize].num_of_objs + 1;
        let buffer_ptr = &mut((*self_).obj_write_set[(*self_).ws_cur_id].buffer[0]) as *mut char; 
        let cur_ws_size = (p_cur as usize) -  (buffer_ptr as usize);
        RLU_ASSERT!(cur_ws_size < RLU_MAX_WRITE_SET_BUFFER_SIZE);
    }
}

fn rlu_send_sync_request(th_id : usize) {
    unsafe {
        (*(g_rlu_threads[th_id])).is_sync = (*(g_rlu_threads[th_id])).is_sync + 1;
        //MEMBARSTLD(); see NOTE in rlu_thread_init()
    }
}

fn rlu_reader_unlock(self_ : *mut rlu_thread_data_t) {
    unsafe {
    (*self_).n_finish = (*self_).n_finish + 1;
    rlu_unregister_thread(self_);
    if (*self_).is_write_detected != 0  {
        (*self_).is_write_detected = 0 ;
        rlu_commit_write_set(self_);
        rlu_release_writer_locks(self_, (((*self_).ws_tail_counter -1 ) % RLU_MAX_WRITE_SETS) as usize);
    } else {
        rlu_release_writer_locks(self_, (*self_).ws_cur_id);
        rlu_release_writer_locks(self_, (*self_).ws_cur_id);
    }
    rlu_sync_checkpoint(self_);
    }
}

fn rlu_try_lock<T>(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut T, obj_size :usize) -> u32 {
    unsafe {
    let mut p_obj : *mut T  = *p_p_obj as *mut T;
    let mut p_obj_copy : *mut T = GET_COPY(p_obj) as *mut T;
    let mut th_id : usize = 0;
    if PTR_IS_COPY(p_obj_copy) {
        //TRACE_1(self, "tried to lock a copy of an object. p_obj = %p\n", p_obj);
        //TRACE_1(self, " => converting \n => copy: %p\n", p_obj);
        p_obj = GET_ACTUAL(p_obj) as *mut T;
        p_obj_copy = GET_COPY(p_obj) as *mut T;
        //TRACE_1(self, " => real: %p , p_obj_copy = %p\n", p_obj, p_obj_copy);
    }
    if (PTR_IS_LOCKED(GET_COPY(p_obj))) {
        let p_ws_obj_h = PTR_GET_WS_HEADER(p_obj_copy);
        th_id = WS_GET_THREAD_ID(p_ws_obj_h);
        if (th_id == (*self_).uniq_id) {
            if (*(*self_).run_counter.get_mut() == WS_GET_RUN_COUNTER(p_ws_obj_h)) {
                // p_obj already locked by current execution of this thread.
                // => return copy
                //TRACE_2(self, "[%ld] already locked by this thread. p_obj = %p th_id = %ld\n", self->local_version, p_obj, th_id);
                *p_p_obj = p_obj_copy;
                return 1;
            }
            //TRACE_1(self, "[%ld] already locked by another execution of this thread -> fail and sync. p_obj = %p th_id = %ld\n",                            self->local_version, p_obj, th_id);
            // p_obj is locked by another execution of this thread.
            (*self_).is_sync = (*self_).is_sync + 1;
            return 0;
        }
        // p_obj already locked by another thread.
        // => send sync request to the other thread
        // => in the meantime -> sync this thread
        //TRACE_1(self, "[%ld] already locked by another thread -> fail and request sync. p_obj = %p th_id = %ld\n", self->local_version, p_obj, th_id);
        rlu_send_sync_request(th_id);
        (*self_).is_sync = (*self_).is_sync + 1;
        return 0;
    }
    // p_obj is free
    
    // Indicate that write-set is updated
    if ((*self_).is_write_detected == 0) {
        (*self_).is_write_detected = 1;
        (*self_).is_check_locks = 1;
    }
    // Add write-set header for the object
    p_obj_copy = rlu_add_ws_obj_header_to_write_set(self_, p_obj, obj_size);
    // Try lock p_obj -> install pointer to copy
    if (!TRY_CAS_PTR_OBJ_COPY(p_obj, p_obj_copy)) {
        //TRACE_1(self, "[%ld] CAS failed\n", self->local_version);
        return 0;
    }
    
    // Locked successfully
    // Copy object to write-set
    rlu_add_obj_copy_to_write_set(self_, p_obj, obj_size);
    RLU_ASSERT!(GET_COPY(p_obj) == p_obj_copy); 
    //RLU_ASSERT_MSG(GET_COPY(p_obj) == p_obj_copy, self, "p_obj_copy = %p my_p_obj_copy = %p\n", GET_COPY(p_obj), p_obj_copy);
    //TRACE_2(self, "[%ld] p_obj = %p is locked, p_obj_copy = %p , th_id = %ld\n", self->local_version, p_obj, GET_COPY(p_obj), GET_THREAD_ID(p_obj));
    
    *p_p_obj = p_obj_copy;
    return 1;
    }
}

pub fn rlu_abort(self_ : *mut rlu_thread_data_t) {
    unsafe {
        (*self_).n_aborts = (*self_).n_aborts + 1;
        rlu_unregister_thread(self_);
        if (*self_).is_write_detected != 0 {
            (*self_).is_write_detected = 0 ;
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
        let ws_id = WS_INDEX(ws_counter);

        (*self_).obj_write_set[ws_id].num_of_objs = 0;
        let mut p_cur_temp =   &mut((*self_).obj_write_set[ws_id].buffer[0]) as *mut char;
        (*self_).obj_write_set[ws_id].p_cur = p_cur_temp as *mut u32;

	rlu_reset_writer_locks(self_, ws_id);
    }
}

//#define RLU_MAX_WRITE_SETS (200) // Minimum value is 2
// #define WS_INDEX(ws_counter) ((ws_counter) % RLU_MAX_WRITE_SETS)
fn WS_INDEX(ws_counter : usize) -> usize {
    ws_counter % 20
}

fn TRY_CAS_PTR_OBJ_COPY<T>(p_obj : *mut T, new_ptr_obj_copy: *mut T ) -> bool {
    unsafe {
    let p_obj_header = OBJ_TO_H(p_obj);
    let val = (*p_obj_header).p_obj_copy.compare_and_swap(std::ptr::null_mut(), new_ptr_obj_copy as *mut u32, Ordering::SeqCst);
    return val == std::ptr::null_mut();
    }
}
fn GET_ACTUAL<T>(p_obj_copy : *mut T) -> *mut T {
    unsafe {
        let p_ws_header = PTR_GET_WS_HEADER(p_obj_copy);
        return (*p_ws_header).p_obj_actual as *mut T;
    }
}

fn PTR_GET_WS_HEADER<T>(p_obj_copy : *mut T) -> *mut rlu_ws_obj_header_t {
    unsafe {
        return OBJ_COPY_TO_WS_H(p_obj_copy);
    }
}

fn OBJ_COPY_TO_WS_H<T>(p_obj_copy : *mut T) -> *mut rlu_ws_obj_header_t {
    unsafe{
        return MOVE_PTR_BACK(p_obj_copy, size_of::<rlu_obj_header_t>() + size_of::<rlu_ws_obj_header_t>()) as *mut rlu_ws_obj_header_t;
    }
}
fn GET_COPY<T>(p_obj: *mut T) -> *mut T{
    unsafe {
        let p_obj_header = OBJ_TO_H(p_obj);
        return (*p_obj_header).p_obj_copy.load(Ordering::SeqCst) as *mut T;
    }
}

fn UNLOCK<T>(p_obj : *mut T) {
    unsafe {
        let mut p_obj_h = OBJ_TO_H(p_obj);
        (*p_obj_h).p_obj_copy.store(std::ptr::null_mut() as *mut u32, Ordering::SeqCst);
        println!("{:?}", (*p_obj_h).p_obj_copy.load(Ordering::SeqCst) as usize);
    }
}

fn  OBJ_TO_H<T>(p_obj : *mut T) -> *mut rlu_obj_header_t {
    unsafe {
        return MOVE_PTR_BACK(p_obj, size_of::<rlu_obj_header_t>()) as *mut rlu_obj_header_t;
    }
}
fn H_TO_OBJ<T>(p_h_obj : *mut T) -> *mut T {
    unsafe {
        return MOVE_PTR_FORWARD(p_h_obj, size_of::<rlu_obj_header_t>());
    }
}

fn PTR_IS_LOCKED<T>(p_obj_copy : *mut T) -> bool {
    unsafe {
        return !p_obj_copy.is_null();
    }
}

fn PTR_IS_UNLOCKED<T>(p_obj_copy : *mut T) -> bool {
    !PTR_IS_LOCKED(p_obj_copy)
}

fn WS_GET_THREAD_ID(p_ws_obj_header : *mut rlu_ws_obj_header_t) -> usize {
    unsafe { 
        return (*p_ws_obj_header).thread_id;
    }
}

fn WS_GET_RUN_COUNTER(p_ws_obj_header : *mut rlu_ws_obj_header_t) -> usize {
    unsafe { 
        return (*p_ws_obj_header).run_counter;
    }
}

fn PTR_IS_COPY<T>(p_obj_copy : *mut T) -> bool {
    unsafe {
        if p_obj_copy as usize == (0x12341234 as usize) {
            return true;
        }
        return false;
    }
}

fn IS_COPY<T>(p_obj : *mut T) -> bool {
    unsafe {
        return PTR_IS_COPY(GET_COPY(p_obj));
    }
}

fn FORCE_ACTUAL<T>(p_obj: *mut T) -> *mut T {
    if IS_COPY(p_obj) {
        return GET_ACTUAL(p_obj);
    } else {
        return p_obj;
    }
}

fn MOVE_PTR_FORWARD<T>(ele: *mut T, size: usize) -> *mut T {
   unsafe {
      let t_ele = ele as *mut u8;
      ele.add(size) as *mut T
    }
}

fn MOVE_PTR_BACK<T>(ele: *mut T, size: usize) -> *mut T {
   unsafe {
      let t_ele = ele as *mut u8;
      ele.sub(size) as *mut T
    }
}

fn ALIGN_OBJ_SIZE(obj_size : usize) -> usize {
    let ALIGN_NUMBER = 8;
    let ALIGN_MASK = ALIGN_NUMBER - 1;
//#define PERFORM_ALIGNMENT(obj_size) (obj_size + (ALIGN_NUMBER - (obj_size & ALIGN_MASK)))
    if (obj_size & ALIGN_MASK != 0) {
        obj_size + (ALIGN_NUMBER - (obj_size & ALIGN_MASK))
    } else {
       obj_size
    }

}
fn rlu_writeback_write_set(self_ : *mut rlu_thread_data_t, ws_counter: usize) {

        unsafe {
            /*TODO: need to think this routine, it has pointer manipulation
             *      can encapsulated in the functions*/ 
            let ws_id = WS_INDEX(ws_counter );
            // NOTE: Converting into CHAR instead of u32
            let mut p_cur =   &mut((*self_).obj_write_set[ws_id].buffer[0]) as *mut char;
        //let mut p_cur = (*self_).obj_write_set[(*self_).ws_cur_id as usize].p_cur;
            
            for i in  0..(*self_).obj_write_set[ws_id].num_of_objs {
                let mut p_ws_obj_h = p_cur as *mut rlu_ws_obj_header_t; 

                let p_obj_actual  = (*p_ws_obj_h).p_obj_actual;
                let obj_size : usize  = (*p_ws_obj_h).obj_size;

                //p_cur = MOVE_PTR_FORWARD(p_cur as *mut u8, size_of::<rlu_ws_obj_header_t>()) as *mut char;
                p_cur = MOVE_PTR_FORWARD(p_cur, size_of::<rlu_ws_obj_header_t>());
                let p_obj_h = p_cur as *mut rlu_ws_obj_header_t;

                //RLU_ASSERT(p_obj_h->p_obj_copy == PTR_ID_OBJ_COPY);

                p_cur = MOVE_PTR_FORWARD(p_cur, size_of::<rlu_obj_header_t>());

                let p_obj_copy = p_cur;

               // TRACE_2(self, "[%ld] rlu_writeback_and_unlock: copy [%p] <- [%p] [%zu]\n",
               //         self->writer_version, p_obj_actual, p_obj_copy, obj_size);

                ptr::copy(p_obj_copy as *const char, p_obj_actual as *mut char, obj_size);

                p_cur = MOVE_PTR_FORWARD(p_cur, ALIGN_OBJ_SIZE(obj_size));

                /*RLU_ASSERT_MSG(GET_THREAD_ID(p_obj_actual) == self->uniq_id,
                        self, "th_id = %ld my_id = %ld\n p_obj_actual = %p num_of_objs = %u\n",
                
                    GET_THREAD_ID(p_obj_actual), self->uniq_id, p_obj_actual, self->obj_write_set[ws_id].num_of_objs);*/
                UNLOCK(p_obj_actual);
                }
	//RLU_ASSERT(p_cur == self->obj_write_set[ws_id].p_cur);*/

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
	    libc::free(OBJ_TO_H(p_obj) as *mut libc::c_void);
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

        (*self_).writer_version = (*(g_rlu_writer_version.get_mut()) as u32) + 1;
        g_rlu_writer_version.fetch_add(1, Ordering::SeqCst);
        rlu_synchronize(self_);

        let ws_wb_num = rlu_writeback_write_sets_and_unlock(self_);

        //RLU_ASSERT_MSG(ws_num == ws_wb_num, self, "failed: %ld != %ld\n", ws_num, ws_wb_num);
        // TODO:define max value
        //(*self_).writer_version = MAX_VERSION;
        (*self_).writer_version = 2147483647 -1;

        g_rlu_commit_version.fetch_add(1, Ordering::SeqCst);

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
        let  cur_lock = g_rlu_writer_locks[writer_lock_id].get_mut();
        let lock_id   = LOCK_ID((*self_).uniq_id);
        match *cur_lock {
        0 => {
              // check for Ordering
                let val = {
                    unsafe {
                             //TODO need to implement compare_and_swap
                            // let mut pa:  = &mut g_rlu_writer_locks[writer_lock_id];
                            g_rlu_writer_locks[writer_lock_id].compare_and_swap(0, lock_id, Ordering::SeqCst);
                            //pa.compare_and_swap(std::ptr::null_mut(), other_ptr, Ordering::SeqCst);
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
fn rlu_try_writer_lock(self_ : *mut rlu_thread_data_t, writer_lock_id : usize) -> u32 {
    //unimplemented!();
    // Valid only for coarse grain
    match (rlu_try_acquire_writer_lock(self_, writer_lock_id))
    {
        Some(x) => {
	            rlu_add_writer_lock(self_, writer_lock_id);
                     1
                   }
        None    => {
                      0
                   }
    }
}


fn rlu_lock<T>(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut T, obj_size :usize) {
    //unimplemented!();
    rlu_try_lock(self_, p_p_obj, obj_size);
}

fn rlu_deref_slow_path<T>(self_ : *mut rlu_thread_data_t, p_obj : *mut T) -> *mut T {
    unsafe {
    if p_obj.is_null() {
        return p_obj;
    }

    let mut th_id : usize = 0;
    let mut p_obj_copy : *mut T = GET_COPY(p_obj);

    if !PTR_IS_LOCKED(p_obj_copy) {
        return p_obj;
    }

    if PTR_IS_COPY(p_obj_copy) {
        // p_obj points to a copy -> it has been already dereferenced.
        //TRACE_1(self, "got pointer to a copy. p_obj = %p p_obj_copy = %p.\n", p_obj, p_obj_copy);
        return p_obj;
    }

    let mut p_ws_obj_h : *mut rlu_ws_obj_header_t = PTR_GET_WS_HEADER(p_obj_copy);
    th_id = WS_GET_THREAD_ID(p_ws_obj_h);
    if (th_id == (*self_).uniq_id) {
    // p_obj is locked by this thread -> return the copy
    // TRACE_1(self, "got pointer to a copy. p_obj = %p th_id = %ld.\n", p_obj, th_id);
        return p_obj_copy;
    }
    // p_obj is locked by another thread
    if ((*self_).is_steal != 0) && ((*g_rlu_threads[th_id]).writer_version <= (*self_).local_version) {
    // This thread started after the other thread updated g_writer_version.
    // and this thread observed a valid p_obj_copy (!= NULL)
    // => The other thread is going to wait for this thread to finish before reusing the write-set log
    //    (to which p_obj_copy points)
    // TRACE_1(self, "take copy from other writer th_id = %ld p_obj = %p p_obj_copy = %p\n", th_id, p_obj, p_obj_copy);
        (*self_).n_steals = (*self_).n_steals + 1;
        return p_obj_copy;
    }

    return p_obj;
    }
}

// this functions seems never used in original source, so may be okay to skip
// if in any case we want to return -1, 0, 1 we need to change return type
fn rlu_cmp_ptrs<T>(p_obj_1 : *mut T, p_obj_2 : *mut T) -> bool {
    unsafe {
    let mut ptr_obj_1 = p_obj_1;
    let mut ptr_obj_2 = p_obj_2;
    if !(p_obj_1.is_null()) {
        ptr_obj_1 = FORCE_ACTUAL(p_obj_1);
    }
    if !(p_obj_1.is_null()) {
        ptr_obj_2 = FORCE_ACTUAL(p_obj_2);
    }

    return (ptr_obj_1 as usize) == (ptr_obj_2 as usize);
    }
}

fn rlu_assign_pointer<T>(p_ptr: *mut *mut T, p_obj : *mut T) {
    let mut ptr = p_obj;
    unsafe {
        if !(p_obj.is_null()) {
            ptr = FORCE_ACTUAL(p_obj); 
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

// Externally exposed functions
pub fn RLU_GET_THREAD_DATA(uniq_id : usize) -> *mut rlu_thread_data_t {
    rlu_get_thread_data(uniq_id)
}

pub fn RLU_THREAD_INIT(self_ : *mut rlu_thread_data_t) ->   usize {
    rlu_thread_init(self_)
}

pub fn RLU_THREAD_FINISH(self_ : *mut rlu_thread_data_t) {
    rlu_thread_finish(self_)
}

pub fn RLU_ALLOC(obj_size : usize ) -> *mut u8 {
    rlu_alloc(obj_size)
}
pub fn RLU_INIT(type_ : u32, max_write_sets: usize) {
    rlu_init(type_, max_write_sets)
}

pub fn RLU_FINISH() {
    rlu_finish();
}
//#define RLU_PRINT_STATS() rlu_print_stats()

pub fn RLU_READER_LOCK(self_ : *mut rlu_thread_data_t) {
    rlu_reader_lock(self_)
}

pub fn RLU_READER_UNLOCK(self_ : *mut rlu_thread_data_t) {
    rlu_reader_unlock(self_)
}

pub fn RLU_FREE<T>(self_ : *mut rlu_thread_data_t, p_obj : *mut T) {
    rlu_free(self_, (p_obj))
}

pub fn RLU_TRY_WRITER_LOCK(self_ : *mut rlu_thread_data_t, writer_lock_id : usize) -> u32 {
    rlu_try_writer_lock(self_, writer_lock_id)
}

pub fn RLU_LOCK<T>(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut T) {
    rlu_lock(self_, p_p_obj, size_of::<T>())
}

pub fn RLU_TRY_LOCK<T>(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut T) -> u32 {
    rlu_try_lock(self_, p_p_obj, size_of::<T>())
}

pub fn RLU_ABORT(self_ : *mut rlu_thread_data_t) {
    rlu_abort(self_)
}

pub fn RLU_IS_SAME_PTRS<T>(p_obj_1 : *mut T, p_obj_2 : *mut T) -> bool {
    rlu_cmp_ptrs(p_obj_1, p_obj_2)
}

pub fn RLU_ASSIGN_POINTER<T>(p_ptr: *mut *mut T, p_obj : *mut T) {
    rlu_assign_pointer(p_ptr, p_obj)
}

pub fn RLU_DEREF<T>(self_: *mut rlu_thread_data_t, p_obj: *mut T) -> *mut T {
    RLU_DEREF_INTERNAL(self_, p_obj)
}

pub fn RLU_DEREF_INTERNAL<T>(self_: *mut rlu_thread_data_t, p_obj: *mut T) -> *mut T { 
        unsafe {
            if ((*self_).is_check_locks == 0) {
                    p_obj
            } else { 
                if (p_obj.is_null() && PTR_IS_UNLOCKED(p_obj)) {
                    p_obj
                } else {
                    rlu_deref_slow_path(self_, p_obj)
                }
            }
	}
}

