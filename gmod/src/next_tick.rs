use super::{next_tick_queue::NextTickQueue, State};
use std::sync::OnceLock;

static NEXT_TICK: OnceLock<NextTickQueue> = OnceLock::new();

pub fn init(state: State) {
    let _ = NEXT_TICK.set(NextTickQueue::new(state));
}

#[inline(always)]
fn global() -> &'static NextTickQueue {
    NEXT_TICK.get().expect("NEXT_TICK not initialized")
}

#[inline(always)]
pub fn next_tick<F>(callback: F)
where
    F: FnOnce(State) + Send + 'static,
{
    let q = global();
    q.queue(callback);
}

#[inline(always)]
pub fn flush_next_tick(l: State) {
    let q = global();
    q.flush(l);
}
