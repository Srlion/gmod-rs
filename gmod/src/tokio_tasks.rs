use std::sync::Mutex;
use std::time::Duration;

use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;
use tokio_util::task::TaskTracker;

use super::State as LuaState;

struct TokioState {
    runtime: Runtime,
    tracker: TaskTracker,
    graceful_shutdown_timeout_secs: u32,
}

static STATE: Mutex<Option<TokioState>> = Mutex::new(None);

pub(crate) fn load(l: LuaState) -> i32 {
    let worker_threads = get_max_worker_threads(l).max(1) as usize;
    let timeout = get_graceful_shutdown_timeout(l);

    let runtime = Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .thread_name(format!("gmod-goobie-rs:{}", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("failed to build tokio runtime");

    let tracker = TaskTracker::new();

    let mut g = STATE.lock().unwrap();
    g.replace(TokioState {
        runtime,
        tracker,
        graceful_shutdown_timeout_secs: timeout,
    });

    0
}

pub(crate) fn unload(_: LuaState) -> i32 {
    let mut g = STATE.lock().unwrap();

    let Some(s) = g.take() else { return 0 };

    s.tracker.close();
    if !s.tracker.is_empty() {
        let timeout = Duration::from_secs(s.graceful_shutdown_timeout_secs as u64);

        // print_goobie!(
        //     "Waiting up to {} seconds for {} connection(s) to complete...",
        //     timeout.as_secs(),
        //     task_tracker.len()
        // );

        s.runtime.block_on(async {
            tokio::select! {
                _ = s.tracker.wait() => {
                    // print_goobie!("All connections have completed!");
                },
                _ = tokio::time::sleep(timeout) => {
                    // print_goobie!("Timed out waiting for connections to complete!");
                },
            }
        });
    }
    s.runtime.shutdown_background();

    // state is dropped here, so is everything inside it

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
    Some(s.runtime.spawn(s.tracker.track_future(fut)))
}

#[inline(always)]
pub fn spawn_detached<F>(fut: F)
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let g = STATE.lock().unwrap();
    let Some(s) = g.as_ref() else {
        return;
    };
    s.runtime.spawn(s.tracker.track_future(fut));
}

#[inline(always)]
pub fn spawn_untracked<F>(fut: F) -> Option<JoinHandle<F::Output>>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let g = STATE.lock().unwrap();
    let s = g.as_ref()?;
    Some(s.runtime.spawn(fut))
}

#[inline(always)]
pub fn spawn_untracked_detached<F>(fut: F)
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let g = STATE.lock().unwrap();
    let Some(s) = g.as_ref() else {
        return;
    };
    s.runtime.spawn(fut);
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
