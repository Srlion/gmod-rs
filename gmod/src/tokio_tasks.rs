use std::sync::OnceLock;

use tokio::runtime::{Builder, Runtime};
use tokio_util::task::TaskTracker;

use super::State as LuaState;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static TRACKER: OnceLock<TaskTracker> = OnceLock::new();

static GRACEFUL_SHUTDOWN_TIMEOUT: OnceLock<u32> = OnceLock::new();

pub(crate) fn load(l: LuaState) -> i32 {
    let worker_threads = get_max_worker_threads(l);
    let _ = RUNTIME.set(
        Builder::new_multi_thread()
            .worker_threads(worker_threads.max(1).into())
            .enable_all()
            .thread_name(format!("gmod-goobie-rs:{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build tokio runtime"),
    );

    let _ = TRACKER.set(TaskTracker::new());

    let _ = GRACEFUL_SHUTDOWN_TIMEOUT.set(get_graceful_shutdown_timeout(l));

    0
}

pub(crate) fn unload(_: LuaState) -> i32 {
    let tracker = tracker();
    tracker.close();

    let runtime = runtime();

    if !tracker.is_empty() {
        let timeout =
            std::time::Duration::from_secs(*GRACEFUL_SHUTDOWN_TIMEOUT.get().unwrap() as u64);

        // print_goobie!(
        //     "Waiting up to {} seconds for {} connection(s) to complete...",
        //     timeout.as_secs(),
        //     task_tracker.len()
        // );

        runtime.block_on(async {
            tokio::select! {
                _ = tracker.wait() => {
                    // print_goobie!("All connections have completed!");
                },
                _ = tokio::time::sleep(timeout) => {
                    // print_goobie!("Timed out waiting for connections to complete!");
                },
            }
        });
    }

    0
}

#[inline(always)]
fn tracker() -> &'static TaskTracker {
    TRACKER
        .get()
        .expect("goobie tokio task tracker not initialized")
}

#[inline(always)]
fn runtime() -> &'static Runtime {
    RUNTIME.get().expect("goobie tokio runtime not initialized")
}

/// Spawn a future and track it.
#[inline(always)]
pub fn spawn<F>(f: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    runtime().spawn(tracker().track_future(f))
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
