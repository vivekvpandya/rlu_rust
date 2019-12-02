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
    size : u32,
    ids : [u32; 20] //array , size must be known at compile time, according to rust spec this lives on stack
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
    ws_cur_id : u32,
    obj_write_set : [obj_list_t; 200],
    free_nodes_size : u32,
    free_nodes : [*mut u32; 200],

    n_starts : u32,
    n_finish : u32,
    n_writers: u32,
    n_writer_writeback: u32,
    n_pure_readers : u32,
    n_aborts: u32,
    n_steals : u32,
    n_writer_sync_waits: u32,
    n_writerback_q_iters: u32,
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
static mut g_rlu_cur_threads : u32 = 0;
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

fn rlu_try_write_lock(self_ : *mut rlu_thread_data_t, writer_lock_id : u32) -> u32 {
    unimplemented!();
}


fn rlu_lock(self_ : *mut rlu_thread_data_t, p_p_obj : *mut*mut u32, obj_size :u32) {
    unimplemented!();
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
    unimplemented!();
}
