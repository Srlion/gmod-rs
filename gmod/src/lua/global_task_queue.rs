use std::mem::MaybeUninit;

use super::task_queue::TaskQueue;
use super::State;

pub static mut TASK_QUEUE: MaybeUninit<TaskQueue> = MaybeUninit::uninit();

pub fn read<'a>() -> &'a TaskQueue {
    unsafe { TASK_QUEUE.assume_init_ref() }
}

pub fn load(l: State) {
    unsafe {
        TASK_QUEUE.write(TaskQueue::new(l));
    }
}

pub fn unload(_: State) {
    unsafe { TASK_QUEUE.assume_init_drop() };
}

pub fn wait_lua_tick<F>(callback: F)
where
    F: FnOnce(State) + Send + 'static,
{
    read().add(callback);
}

pub fn poll(l: State) {
    read().poll(l);
}
