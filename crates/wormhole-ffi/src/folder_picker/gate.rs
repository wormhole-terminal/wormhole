//! How many disk jobs the pickers of the whole process have going at once, and for how long one counts: the
//! app's policy (`crates/wormhole/src/picker/gate.rs`), over threads of our own.
//!
//! A read of a folder is a blocking call that nothing can cancel, and on a share that has stopped answering it
//! never comes back. So only a few run at a time, or each key typed over a dead share would add a thread to the
//! stuck ones. A job that has run longer than the [`DEADLINE`] is taken for stuck and stops counting as one that
//! reads, so one folder that never answers cannot keep every picker from reading; but the stuck threads are
//! still there, so the live ones and the stuck together are capped too, and past the cap nothing starts until
//! one of them returns. What keeps one dead path from using them up one request at a time is not here but in
//! core's readers (`folder_picker::looks`: a path that hung is not looked at again).

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long a job may run before it stops counting against the others.
pub(super) const DEADLINE: Duration = Duration::from_millis(1500);
/// How many jobs read at once: the providers of one picker and a decoration, with some room.
pub(super) const READERS: usize = 4;
/// How many threads the pickers may have in use, those that read and those that are stuck.
const THREADS: usize = READERS * 2;

struct Running {
    next: u64,
    jobs: Vec<(u64, Instant)>,
}

static RUNNING: Mutex<Running> = Mutex::new(Running { next: 0, jobs: Vec::new() });
/// Told when a job gives its place back; a waiting job also looks again by itself when the deadline of a
/// stuck one passes.
static FREED: Condvar = Condvar::new();

/// The right to run: held while a job runs, given back when it ends.
pub(super) struct Reading(u64);

impl Reading {
    /// A place, if there is one: fewer than [`READERS`] jobs that have not outlived the deadline, and fewer
    /// than [`THREADS`] that have not ended, stuck or not.
    fn take(running: &mut Running) -> Option<Self> {
        let live = running.jobs.iter().filter(|(_, since)| since.elapsed() < DEADLINE).count();
        if live >= READERS || running.jobs.len() >= THREADS {
            return None;
        }
        running.next += 1;
        let id = running.next;
        running.jobs.push((id, Instant::now()));
        Some(Reading(id))
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        RUNNING.lock().unwrap_or_else(PoisonError::into_inner).jobs.retain(|(id, _)| *id != self.0);
        FREED.notify_all();
    }
}

/// Wait for a place to run in; `None` when `wanted` says the answer is no longer wanted before one is free.
pub(super) fn enter(wanted: impl Fn() -> bool) -> Option<Reading> {
    let mut running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner);
    loop {
        if !wanted() {
            return None;
        }
        if let Some(place) = Reading::take(&mut running) {
            return Some(place);
        }
        // Sleep until a place is given back, or the oldest live job outlives the deadline, or a moment passes
        // (a cancel does not wake anyone).
        let (guard, _) =
            FREED.wait_timeout(running, Duration::from_millis(40)).unwrap_or_else(PoisonError::into_inner);
        running = guard;
    }
}

/// The tests that read through the gate take turns: it is one for the whole process.
#[cfg(test)]
pub(super) static TURN: Mutex<()> = Mutex::new(());

#[cfg(test)]
pub(super) fn count() -> usize {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner).jobs.len()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{DEADLINE, READERS, RUNNING, Reading, THREADS, TURN, enter};

    fn take() -> Option<Reading> {
        Reading::take(&mut RUNNING.lock().expect("lock"))
    }

    fn make_stuck(id: u64) {
        let long_ago = Instant::now().checked_sub(DEADLINE + Duration::from_millis(1)).expect("instant");
        RUNNING.lock().expect("lock").jobs.iter_mut().filter(|(i, _)| *i == id).for_each(|e| e.1 = long_ago);
    }

    #[test]
    fn only_a_few_run_and_one_that_outlived_the_deadline_stops_counting() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let held: Vec<Reading> = (0..READERS).map(|_| take().expect("a place")).collect();
        assert!(take().is_none(), "the places are taken");
        make_stuck(held[0].0);
        let next = take().expect("a place, since one is stuck");
        assert!(take().is_none(), "but only one more");
        drop(next);
        drop(held);
        assert_eq!(super::count(), 0, "a job that ends gives its place back");
    }

    #[test]
    fn stuck_jobs_are_capped_too_so_they_cannot_use_up_every_thread() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let mut held = Vec::new();
        while let Some(job) = take() {
            make_stuck(job.0);
            held.push(job);
            assert!(held.len() <= THREADS, "no more than the cap");
        }
        assert_eq!(held.len(), THREADS, "all of them could be stuck, and no more were started");
        held.pop();
        let next = take().expect("a place");
        assert!(take().is_none());
        drop(next);
        drop(held);
        assert_eq!(super::count(), 0);
    }

    #[test]
    fn a_waiting_job_that_is_no_longer_wanted_stops_waiting() {
        let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
        let held: Vec<Reading> = (0..READERS).map(|_| take().expect("a place")).collect();
        let asked = std::cell::Cell::new(0);
        let place = enter(|| {
            asked.set(asked.get() + 1);
            asked.get() < 3
        });
        assert!(place.is_none());
        drop(held);
    }
}
