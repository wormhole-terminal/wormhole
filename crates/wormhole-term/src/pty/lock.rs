//! The lock the reader thread and the UI share: a mutex that is handed over in the order
//! threads asked for it.
//!
//! The reader parses in slices and takes the lock for each, and under a flood it asks again the
//! moment it lets go. A plain mutex then lets the reader win the next round far more often than
//! not (the thread that just released it is running, the waiter still has to be woken), and the
//! UI's snapshot waits out the flood (see the measurement in the tests below).
//!
//! Here a thread first takes a turn (`next`) and only then the data, keeping the turn until it
//! has the data: the reader, back for the next slice, queues behind the UI at the turn instead of
//! overtaking it, so a thread waits for at most the holder it found, never for the holder's
//! successors. Two uncontended mutexes cost about 40 ns, against microseconds of parsing in each
//! slice. A UI asking for the lock again and again during a flood
//! (`measure::ui_lock_under_a_flood`, four alternating runs, M5 Pro) waited 2 to 3 µs on average
//! for it here, 58 to 74 µs under the event loop of the library this replaced.

use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::perf::{self, Held, Side};

/// A mutex with fair hand-over (see the module docs). A thread that panicked while holding it
/// does not poison it for the others: the screen is read for what it shows, whatever state a bug
/// left it in, which beats a UI that panics on every lock.
pub(crate) struct FairLock<T> {
    next: Mutex<()>,
    data: Mutex<T>,
}

/// The data under [`FairLock`]. The data's guard comes first, so it is let go of before `held`
/// counts how long it was held (only while `perf-profile` counts).
pub(crate) struct FairGuard<'a, T> {
    data: MutexGuard<'a, T>,
    _held: Held,
}

impl<T> FairLock<T> {
    /// A lock around `value`.
    pub(crate) fn new(value: T) -> Self {
        Self { next: Mutex::new(()), data: Mutex::new(value) }
    }

    /// Wait for the lock behind everyone who asked before.
    pub(crate) fn lock(&self) -> FairGuard<'_, T> {
        self.lock_as(Side::Ui)
    }

    /// [`lock`](Self::lock), counted as `side`'s wait and hold when profiling.
    pub(crate) fn lock_as(&self, side: Side) -> FairGuard<'_, T> {
        let asked = perf::clock();
        let turn = self.next.lock().unwrap_or_else(PoisonError::into_inner);
        let data = self.data.lock().unwrap_or_else(PoisonError::into_inner);
        // The turn goes on only now: whoever asks next queues behind the data's next holder.
        drop(turn);
        FairGuard { data, _held: Held::new(side, asked) }
    }
}

impl<T> Deref for FairGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for FairGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use super::FairLock;

    /// Runs a closure on the counter under a lock.
    type With = Box<dyn Fn(&mut dyn FnMut(&mut usize)) + Send + Sync>;

    /// How many holds of a thread that takes the lock again the moment it lets go (as the reader
    /// does under a flood) pass before a thread that asks every millisecond gets in, at worst.
    fn worst_wait(with: With) -> usize {
        const HOLD: Duration = Duration::from_micros(200);
        let with = Arc::new(with);
        let done = Arc::new(AtomicBool::new(false));
        let holds = Arc::new(AtomicUsize::new(0));
        let hog = {
            let (with, done, holds) = (with.clone(), done.clone(), holds.clone());
            std::thread::spawn(move || {
                while !done.load(Ordering::Acquire) {
                    with(&mut |count| {
                        let until = Instant::now() + HOLD;
                        while Instant::now() < until {
                            std::hint::spin_loop();
                        }
                        *count += 1;
                        holds.store(*count, Ordering::Release);
                    });
                }
            })
        };
        let mut worst = 0;
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(1));
            let asked_at = holds.load(Ordering::Acquire);
            let mut got_at = 0;
            with(&mut |count| got_at = *count);
            worst = worst.max(got_at.saturating_sub(asked_at));
        }
        done.store(true, Ordering::Release);
        hog.join().expect("the hog finishes");
        worst
    }

    /// The waiter gets in after at most a hold or two of the other thread, not after the whole
    /// run (a plain `Mutex` in the same test let 300 to 500 holds, 60 to 100 ms, pass).
    #[test]
    fn a_thread_that_keeps_asking_cannot_starve_one_that_asked_first() {
        let data = FairLock::new(0usize);
        let worst = worst_wait(Box::new(move |f| f(&mut data.lock())));
        assert!(worst <= 3, "the waiter let {worst} holds go by");
    }

    /// The same measure for a plain mutex, to see what the fair lock buys. Run with
    /// `--ignored --nocapture`.
    #[test]
    #[ignore = "measurement; run with --ignored --nocapture"]
    fn a_plain_mutex_lets_the_reader_win_again_and_again() {
        let data = std::sync::Mutex::new(0usize);
        let worst = worst_wait(Box::new(move |f| f(&mut data.lock().expect("not poisoned"))));
        eprintln!("a plain mutex: the waiter let {worst} holds go by");
    }

    /// The counters see each side take the lock, and nothing once stopped. Other tests' terminals
    /// may be reading meanwhile and add to them, so the counts are lower bounds.
    #[cfg(feature = "perf-counters")]
    #[test]
    fn profiling_counts_each_sides_locks_and_stops_when_asked() {
        use crate::perf::{self, COUNTING_TESTS, Side};
        let _turn = COUNTING_TESTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let lock = FairLock::new(0u32);
        perf::start();
        *lock.lock_as(Side::Reader) += 1;
        *lock.lock() += 1;
        perf::parsed(100, perf::clock());
        let counted = perf::stop();
        assert!(counted.reader.locks >= 1 && counted.ui.locks >= 1, "{counted:?}");
        assert!(counted.bytes >= 100 && counted.slices >= 1, "{counted:?}");
        // Stopped: the clock is off, so a call site counts nothing, however much it parsed.
        assert!(perf::clock().is_none());
        perf::parsed(1 << 40, perf::clock());
        assert!(perf::read().bytes < 1 << 40, "a stopped profile still counted");
    }

    #[test]
    fn a_panic_under_the_lock_leaves_it_usable() {
        let lock = Arc::new(FairLock::new(5));
        let other = lock.clone();
        let _ = std::thread::spawn(move || {
            let _guard = other.lock();
            panic!("a bug under the lock");
        })
        .join();
        assert_eq!(*lock.lock(), 5);
    }
}
