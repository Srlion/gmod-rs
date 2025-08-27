use super::{next_tick_queue::NextTickQueue, State};
use std::sync::Mutex;

static NEXT_TICK: Mutex<Option<NextTickQueue>> = Mutex::new(None);

pub fn load(l: State) {
    let mut q = NEXT_TICK.lock().unwrap();
    q.replace(NextTickQueue::new(l));
}

pub fn unload(state: State) {
    let mut q = NEXT_TICK.lock().unwrap();
    if let Some(q) = q.take() {
        q.flush(state);
    }
}

#[inline(always)]
fn with_next_tick<F>(f: F)
where
    F: FnOnce(&NextTickQueue),
{
    let q = NEXT_TICK.lock().unwrap();
    if let Some(q) = q.as_ref() {
        f(q);
    }
}

#[inline(always)]
pub fn next_tick<F>(callback: F)
where
    F: FnOnce(State) + Send + 'static,
{
    with_next_tick(|q| q.queue(callback));
}

#[inline(always)]
pub fn flush_next_tick(l: State) {
    with_next_tick(|q| q.flush(l));
}
