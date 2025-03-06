use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{ffi::c_void, sync::atomic::Ordering};

use super::State;

type CallbackBoxed = Box<dyn FnOnce(State) + Send>;

struct LuaReceiver {
    rx: mpsc::Receiver<CallbackBoxed>,
    timer_name: String,
    counter_ptr: usize,   // pointer of the counter AtomicUsize
    is_closed_ptr: usize, // pointer of the is_closed AtomicBool
}

impl LuaReceiver {
    fn get_counter(&self) -> &AtomicUsize {
        unsafe { &mut *(self.counter_ptr as *mut AtomicUsize) }
    }

    fn increment_counter(&self) {
        self.get_counter().fetch_add(1, Ordering::Release);
    }

    fn count(&self) -> usize {
        self.get_counter().load(Ordering::Acquire)
    }

    fn get_is_closed(&self) -> &AtomicBool {
        unsafe { &*(self.is_closed_ptr as *const AtomicBool) }
    }

    fn set_closed(&self) {
        self.get_is_closed().store(true, Ordering::Release);
    }

    fn is_closed(&self) -> bool {
        self.get_is_closed().load(Ordering::Acquire)
    }

    pub fn poll(&self, l: State) {
        // we max it to avoid starving the main thread OR lagging it
        for _ in 0..5 {
            match self.rx.try_recv() {
                Ok(callback) => {
                    self.get_counter().fetch_sub(1, Ordering::Release);
                    l.set_top(0); // clear the stack
                    callback(l);
                }
                Err(_) => break, // exit if there is no callback
            }
        }
    }
}

impl Drop for LuaReceiver {
    fn drop(&mut self) {
        let _ = unsafe { Box::from_raw(self.counter_ptr as *mut AtomicUsize) };
        let _ = unsafe { Box::from_raw(self.is_closed_ptr as *mut AtomicBool) };
    }
}

#[derive(Clone)]
pub struct TaskQueue {
    sender: mpsc::Sender<CallbackBoxed>,
    lua_receiver_ptr: usize,
}

impl TaskQueue {
    pub fn new(l: State) -> Self {
        let (tx, rx) = mpsc::channel();

        let counter_ptr = {
            let counter = Box::new(AtomicUsize::new(0));
            Box::into_raw(counter) as usize
        };
        let is_closed_ptr = {
            let is_closed = Box::new(AtomicBool::new(false));
            Box::into_raw(is_closed) as usize
        };

        let mut task_queue = {
            Self {
                sender: tx,
                lua_receiver_ptr: 0,
            }
        };

        let timer_name = {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos();
            format!(
                "_GOOBIE_LUA_THINK_{nanos}_{:p}_{:p}",
                Box::new(nanos),
                Box::new(&task_queue)
            )
        };

        let lua_receiver = Box::new(LuaReceiver {
            rx,
            timer_name: timer_name.clone(),
            counter_ptr,
            is_closed_ptr,
        });
        let lua_receiver_ptr = Box::into_raw(lua_receiver);
        task_queue.lua_receiver_ptr = lua_receiver_ptr as usize;

        l.get_global(c"timer");
        {
            l.get_field(-1, c"Create");
            l.pcall_ignore(|| {
                l.push_string(&timer_name);
                l.push_number(0); // interval
                l.push_number(0); // repetitions

                l.push_lightuserdata(lua_receiver_ptr as *mut c_void);
                l.push_closure(task_queue_think, 1);

                0
            });
        }
        l.pop(); // pop the timer table

        task_queue
    }

    fn lua_receiver(&self) -> &LuaReceiver {
        // SAFETY: lua receiver should be alive as long as the task queue is alive
        // also we are accessing it on the main thread, so it should be safe (lua state exists with us wink wink)
        let lua_receiver_ptr = self.lua_receiver_ptr as *mut LuaReceiver;
        unsafe { &*lua_receiver_ptr }
    }

    pub fn add<F>(&self, callback: F)
    where
        F: FnOnce(State) + Send + 'static,
    {
        if super::is_closed() {
            return;
        }
        let _ = self.sender.send(Box::new(callback));
        self.lua_receiver().increment_counter();
    }

    pub fn poll(&self, l: State) {
        self.lua_receiver().poll(l);
    }
}

impl Drop for TaskQueue {
    fn drop(&mut self) {
        self.lua_receiver().set_closed();
    }
}

fn remove_timer(l: State, timer_name: &str) {
    l.get_global(c"timer");
    {
        l.get_field(-1, c"Remove");
        l.pcall_ignore(|| {
            l.push_string(timer_name);
            0
        });
    }
    l.pop(); // pop the timer table
}

unsafe extern "C-unwind" fn task_queue_think(l: State) -> i32 {
    l.push_closure_arg(1); // 1 = lua think receiver
    let lua_receiver_ptr = l.to_userdata(-1) as *mut LuaReceiver;
    let lua_receiver = &*lua_receiver_ptr;

    if lua_receiver.count() == 0 {
        // if no more tasks and the task queue is closed, we need to remove the timer and drop the receiver
        if lua_receiver.is_closed() {
            remove_timer(l, &lua_receiver.timer_name);
            let _ = Box::from_raw(lua_receiver_ptr); // consume the receiver to drop it to avoid memory leak
        }

        return 0;
    }

    lua_receiver.poll(l);

    0
}
