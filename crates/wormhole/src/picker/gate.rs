//! How many disk jobs the pickers of the whole app have going at once, and for how long one counts.
//!
//! A read of a folder is a blocking call on a pool thread that nothing can cancel, and on a share that has
//! stopped answering it never comes back. So only a few run at a time, or each key typed over a dead share
//! would add a thread to the stuck ones. But a job that has run longer than the [`DEADLINE`] is taken for stuck
//! and stops counting: the next one may go, so one folder that never answers cannot keep every picker, in every
//! window, from reading anything until the app is restarted.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use gpui_kit::BackgroundExecutor;

/// How long a job may run before the picker says it is still reading, and before it stops counting against
/// the others.
pub(super) const DEADLINE: Duration = Duration::from_millis(1500);
/// How many jobs go at once: the providers of one picker and a decoration, with some room.
const READERS: usize = 4;
/// How often a job that waits for a place looks again.
const WAIT: Duration = Duration::from_millis(40);

/// The jobs that are running: a number for each, and since when.
static RUNNING: Mutex<(u64, Vec<(u64, Instant)>)> = Mutex::new((0, Vec::new()));

/// The right to run: held while a job runs, wherever it runs.
pub(super) struct Reading(u64);

impl Reading {
    /// A place, if there is one: fewer than [`READERS`] jobs that have not outlived the deadline.
    fn take() -> Option<Self> {
        let mut running = RUNNING.lock().ok()?;
        let live = running.1.iter().filter(|(_, since)| since.elapsed() < DEADLINE).count();
        if live >= READERS {
            return None;
        }
        running.0 += 1;
        let id = running.0;
        running.1.push((id, Instant::now()));
        Some(Reading(id))
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        if let Ok(mut running) = RUNNING.lock() {
            running.1.retain(|(id, _)| *id != self.0);
        }
    }
}

/// Wait for a place to run in.
pub(super) async fn enter(executor: &BackgroundExecutor) -> Reading {
    loop {
        if let Some(right) = Reading::take() {
            return right;
        }
        executor.timer(WAIT).await;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use super::{DEADLINE, READERS, RUNNING, Reading};

    /// The gate is one for the whole process: the tests that use it take turns.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    #[test]
    fn only_a_few_run_and_one_that_outlived_the_deadline_stops_counting() {
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let held: Vec<Reading> = (0..READERS).map(|_| Reading::take().expect("a place")).collect();
        assert!(Reading::take().is_none(), "the places are taken");
        // One job has been going for longer than the deadline: it is taken for stuck and a new one may go.
        RUNNING.lock().unwrap().1[0].1 =
            Instant::now().checked_sub(DEADLINE + Duration::from_millis(1)).unwrap();
        let next = Reading::take().expect("a place, since one is stuck");
        assert!(Reading::take().is_none(), "but only one more");
        drop(next);
        drop(held);
        assert!(RUNNING.lock().unwrap().1.is_empty(), "a job that ends gives its place back");
    }
}
