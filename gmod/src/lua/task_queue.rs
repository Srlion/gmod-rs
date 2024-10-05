use std::borrow::Borrow;
use std::iter::repeat_with;
use std::{
    borrow::Cow,
    ffi::c_void,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use gmod_macros::lua_function;

use super::State;
use crate as gmod;

type CallbackBoxed = Box<dyn FnOnce(State) + Send>;

#[repr(C)]
struct CallbackCtx<'a> {
    callback: CallbackBoxed,
    traceback: Cow<'a, str>,
}

pub struct TaskQueue {
    sender: flume::Sender<CallbackCtx<'static>>,
    receiver: flume::Receiver<CallbackCtx<'static>>,
}

impl Default for TaskQueue {
    fn default() -> Self {
        let (tx, rx) = flume::unbounded();
        Self {
            sender: tx,
            receiver: rx,
        }
    }
}

pub static COUNTER: AtomicUsize = AtomicUsize::new(0);
pub static mut TASK_QUEUE: MaybeUninit<TaskQueue> = MaybeUninit::uninit();
static mut GMOD_CLOSED: bool = false;

pub fn read<'a>() -> &'a TaskQueue {
    unsafe { TASK_QUEUE.assume_init_ref() }
}

pub fn load(l: State) {
    unsafe {
        TASK_QUEUE.write(TaskQueue::default());
    }

    let random_str: String = repeat_with(fastrand::alphanumeric).take(10).collect();
    let timer_name = format!("_GOOBIE_LUA_THINK_{random_str}_{:p}", Box::new(read()));

    l.get_global(c"timer");
    {
        l.get_field(-1, c"Create");
        {
            l.push_string(&timer_name);
            l.push_number(0);
            l.push_number(0);
            l.push_function(task_queue_think);
        }
        l.pcall_ignore(4, 0);
    }
    l.pop();
}

pub fn unload(l: State) {
    unsafe { GMOD_CLOSED = true };
    unsafe { TASK_QUEUE.assume_init_read() };
}

pub fn wait_lua_tick<F>(traceback: String, callback: F)
where
    F: FnOnce(State) + Send + 'static,
{
    if unsafe { GMOD_CLOSED } {
        return;
    }

    read().sender.send(CallbackCtx {
        callback: Box::new(callback),
        traceback: Cow::Owned(traceback),
    });
    COUNTER.fetch_add(1, Ordering::Release);
}

pub fn run_callbacks(l: State) {
    if unsafe { GMOD_CLOSED } {
        return;
    }

    if is_empty() {
        return;
    }

    let task_queue = read();
    while let Ok(callback_ctx) = task_queue.receiver.try_recv() {
        COUNTER.fetch_sub(1, Ordering::Release);
        process_callback(l, callback_ctx);
    }
}

pub fn len() -> usize {
    COUNTER.load(Ordering::Acquire)
}

pub fn is_empty() -> bool {
    len() == 0
}

fn process_callback(l: State, mut callback_ctx: CallbackCtx) {
    let traceback = std::mem::replace(&mut callback_ctx.traceback, Cow::Borrowed(""));

    let callback_ctx_ptr: *mut c_void = Box::into_raw(Box::new(callback_ctx)) as *mut c_void;
    l.cpcall_ignore(handle_task_queue, callback_ctx_ptr, Some(&traceback));
}

extern "C-unwind" fn handle_task_queue(l: State) -> i32 {
    let callback_ctx_ptr = l.to_userdata(1);
    let callback_ctx = unsafe { Box::from_raw(callback_ctx_ptr as *mut CallbackCtx) };

    let traceback = callback_ctx.traceback;
    let callback = callback_ctx.callback;

    callback(l);
    // Box::from_raw will automatically drop the callback

    0
}

extern "C-unwind" fn task_queue_think(l: State) -> i32 {
    run_callbacks(l);
    0
}
