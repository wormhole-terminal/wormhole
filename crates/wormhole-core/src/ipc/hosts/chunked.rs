//! Asking many hosts at once without a thread for each: a person can have dozens, and each question may
//! wait for a stuck host. Threads, not an async runtime; a few at a time, answers in the order of the
//! hosts. Used by ending every host, by the survey and by the check of which orphans take the app.

/// How many hosts are asked at once.
pub(super) const AT_ONCE: usize = 16;

/// `f` of every item, [`AT_ONCE`] at a time on threads of their own; the results in the order of `items`.
/// A thread that panics gives `fallback` for its item.
pub(super) fn map_chunked<T: Sync, R: Send>(
    items: &[T],
    fallback: impl Fn() -> R,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let mut out = Vec::with_capacity(items.len());
    for chunk in items.chunks(AT_ONCE) {
        std::thread::scope(|scope| {
            let running: Vec<_> = chunk.iter().map(|item| scope.spawn(|| f(item))).collect();
            out.extend(running.into_iter().map(|handle| handle.join().unwrap_or_else(|_| fallback())));
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn results_come_back_in_order_and_never_more_than_a_chunk_runs_at_once() {
        let running = AtomicUsize::new(0);
        let most = AtomicUsize::new(0);
        let items: Vec<usize> = (0..50).collect();
        let doubled = map_chunked(
            &items,
            || 0,
            |n| {
                let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(2));
                running.fetch_sub(1, Ordering::SeqCst);
                n * 2
            },
        );
        assert_eq!(doubled, items.iter().map(|n| n * 2).collect::<Vec<_>>());
        assert!(most.load(Ordering::SeqCst) <= AT_ONCE, "{} at once", most.load(Ordering::SeqCst));
    }

    #[test]
    fn a_panicking_question_gives_the_fallback_for_its_item_only() {
        let out = map_chunked(&[1, 2, 3], || -1, |n| if *n == 2 { panic!("stuck host") } else { *n });
        assert_eq!(out, [1, -1, 3]);
    }
}
