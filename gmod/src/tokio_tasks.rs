// the weird design in here is because, if you spawn the async runtime on main thread, it stops dlclose on linux to actually
// deload the module, which causes `static`s to be alive, so if they are loaded again, they are basically with old memory
// so to go around this, chatgpt told me a nice solution which is spawning the async runtime on a separate thread
// which actually solves the issue and dlclose safely close the process

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tokio::runtime::{Builder, Handle};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;

use super::State as LuaState;

struct TokioState {
    handle: Handle,
    tracker: TaskTracker,
    stop_tx: oneshot::Sender<()>,
    join: Option<thread::JoinHandle<()>>,
}

static STATE: Mutex<Option<TokioState>> = Mutex::new(None);

pub(crate) fn load(l: LuaState) -> i32 {
    let worker_threads = get_max_worker_threads(l).max(1) as usize;
    let timeout_secs = get_graceful_shutdown_timeout(l);

    // channel to stop the supervisor
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    // channel to get initialized artifacts back from the supervisor
    let (ready_tx, ready_rx) = oneshot::channel::<(Handle, TaskTracker)>();

    let thread_name = format!("gmod-goobie-rs:{}", env!("CARGO_PKG_VERSION"));
    let join = thread::Builder::new()
        .name(thread_name.clone())
        .spawn(move || {
            // Build the multi-thread Tokio runtime *inside this thread*.
            let runtime = Builder::new_multi_thread()
                .worker_threads(worker_threads)
                .enable_all()
                .thread_name(thread_name)
                .build()
                .expect("failed to build tokio runtime");

            let tracker = TaskTracker::new();

            // Hand a clone of the handle & tracker back to the caller thread.
            let _ = ready_tx.send((runtime.handle().clone(), tracker.clone()));

            // Wait for stop signal.
            let _ = stop_rx.blocking_recv();

            // Graceful shutdown happens here, on the same thread that created the runtime.
            tracker.close();
            let timeout = Duration::from_secs(timeout_secs as u64);
            let _ = runtime.block_on(async { tokio::time::timeout(timeout, tracker.wait()).await });
            runtime.shutdown_timeout(timeout);
            // Thread exits -> its TLS destructors run -> loader may unmap the DSO.
        })
        .expect("failed to spawn supervisor thread");

    // Receive the handle/tracker produced by the supervisor.
    let (handle, tracker) = ready_rx.blocking_recv().expect("supervisor init failed");

    let mut g = STATE.lock().unwrap();
    *g = Some(TokioState {
        handle,
        tracker,
        stop_tx,
        join: Some(join),
    });

    0
}

pub(crate) fn unload(_: LuaState) -> i32 {
    let state = {
        let mut g = STATE.lock().unwrap();
        g.take()
    };
    let Some(mut s) = state else { return 0 };

    // Tell the supervisor to stop and join it.
    let _ = s.stop_tx.send(());
    if let Some(j) = s.join.take() {
        let _ = j.join();
    }
    0
}

#[inline(always)]
pub fn spawn<F>(fut: F) -> Option<JoinHandle<F::Output>>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let g = STATE.lock().unwrap();
    let s = g.as_ref()?;
    Some(s.handle.spawn(s.tracker.track_future(fut)))
}

#[inline(always)]
pub fn spawn_untracked<F>(fut: F) -> Option<JoinHandle<F::Output>>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let g = STATE.lock().unwrap();
    let s = g.as_ref()?;
    Some(s.handle.spawn(fut))
}

fn get_max_worker_threads(l: LuaState) -> u16 {
    let mut max_worker_threads = 2;

    l.get_global(c"CreateConVar");
    let ok = l.pcall_ignore(|| {
        l.push_string("GOOBIE_WORKER_THREADS");
        l.push_number(max_worker_threads);
        l.create_table(2, 0);
        {
            l.get_global(c"FCVAR_ARCHIVE");
            l.raw_seti(-2, 1);

            l.get_global(c"FCVAR_PROTECTED");
            l.raw_seti(-2, 2);
        }
        l.push_string("Number of worker threads for the mysql connection pool");
        1
    });

    if ok {
        l.get_field(-1, c"GetInt");
        let ok2 = l.pcall_ignore(|| {
            l.push_value(-2);
            1
        });
        if ok2 {
            max_worker_threads = l.to_number(-1) as u16;
            l.pop();
        }
        l.pop();
    }

    max_worker_threads
}

fn get_graceful_shutdown_timeout(l: LuaState) -> u32 {
    let mut timeout = 20;

    l.get_global(c"CreateConVar");
    let ok = l.pcall_ignore(|| {
        l.push_string("GOOBIE_GRACEFUL_SHUTDOWN_TIMEOUT");
        l.push_number(timeout);
        l.create_table(2, 0);
        {
            l.get_global(c"FCVAR_ARCHIVE");
            l.raw_seti(-2, 1);

            l.get_global(c"FCVAR_PROTECTED");
            l.raw_seti(-2, 2);
        }
        l.push_string("Timeout for graceful shutdown of the mysql connections, in seconds");
        1
    });

    if ok {
        l.get_field(-1, c"GetInt");
        let ok2 = l.pcall_ignore(|| {
            l.push_value(-2);
            1
        });
        if ok2 {
            timeout = l.to_number(-1) as u32;
            l.pop();
        }
        l.pop();
    }

    timeout
}
