//! Looking at a path on the disk without being the thread that a dead mount keeps.
//!
//! A `stat` or a read of a folder on a share that stopped answering never returns, and the thread that made it
//! is stuck for good. A picker looks at many paths (the recent folders, every row's chevron and branch), so one
//! dead share could take a thread for each look, every few seconds, until the app has none left. So a look
//! goes through [`guarded`]: a path that is already being looked at is not looked at by a second thread, and a
//! path whose look has taken longer than [`STUCK`] is taken for hung and not looked at again in this run.
//! Whoever asked gets nothing for such a path (no row, no chevron), which is the honest answer.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a look may take before the path is taken for hung.
pub(super) const STUCK: Duration = Duration::from_millis(1500);

#[derive(Default)]
struct Looks {
    /// The looks that are going on, and since when.
    going: HashMap<PathBuf, Instant>,
    /// The paths whose look took too long, which a look that came back late does take off.
    hung: HashSet<PathBuf>,
}

static LOOKS: Mutex<Option<Looks>> = Mutex::new(None);

fn with<T>(f: impl FnOnce(&mut Looks) -> T) -> Option<T> {
    let mut looks = LOOKS.lock().ok()?;
    Some(f(looks.get_or_insert_with(Looks::default)))
}

/// Whether the path is known to hang: a look took too long, and none has come back since.
fn hung(path: &Path, looks: &mut Looks) -> bool {
    if looks.hung.contains(path) {
        return true;
    }
    if looks.going.get(path).is_some_and(|since| since.elapsed() >= STUCK) {
        looks.hung.insert(path.to_path_buf());
        return true;
    }
    false
}

/// `look` at `path`, unless the path hangs or is being looked at already (then `None`). The look runs on this
/// thread, and the path is on record while it does.
pub(super) fn guarded<T>(path: &Path, look: impl FnOnce() -> T) -> Option<T> {
    let go = with(|looks| {
        if hung(path, looks) || looks.going.contains_key(path) {
            return false;
        }
        looks.going.insert(path.to_path_buf(), Instant::now());
        true
    })?;
    if !go {
        return None;
    }
    let found = look();
    with(|looks| {
        looks.going.remove(path);
        // A look that did come back: the path answers after all.
        looks.hung.remove(path);
    });
    Some(found)
}

/// Whether the path is one a look would be refused for, without looking: for a list that would rather skip it.
pub(super) fn is_hung(path: &Path) -> bool {
    with(|looks| hung(path, looks)).unwrap_or(false)
}

/// Take `path` for hung, as a look that took too long would.
#[cfg(test)]
pub(super) fn mark_hung(path: &Path) {
    with(|looks| looks.hung.insert(path.to_path_buf()));
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Instant;

    use super::{STUCK, guarded, is_hung, with};

    #[test]
    fn a_look_runs_and_is_off_the_record_when_it_comes_back() {
        let path = Path::new("/looks/fine");
        assert_eq!(guarded(path, || 7), Some(7));
        assert!(with(|l| !l.going.contains_key(path) && !l.hung.contains(path)).unwrap_or(false));
    }

    #[test]
    fn a_second_look_at_a_path_that_is_being_looked_at_is_not_a_second_thread() {
        let path = Path::new("/looks/slow");
        let inner = guarded(path, || guarded(path, || 1));
        assert_eq!(inner, Some(None), "the look inside the look was refused");
    }

    #[test]
    fn a_look_that_has_gone_on_too_long_makes_the_path_hung_and_it_is_not_looked_at_again() {
        let path = Path::new("/looks/dead-mount");
        // A thread that is stuck on it: on the record for longer than a look may take.
        let long_ago = Instant::now().checked_sub(STUCK + std::time::Duration::from_millis(1)).unwrap();
        with(|l| l.going.insert(path.to_path_buf(), long_ago));
        assert!(is_hung(path));
        assert_eq!(guarded(path, || 1), None, "no new thread goes there");
        // It stays so even after the stuck thread's record is gone from `going`: the verdict is kept.
        with(|l| l.going.remove(path));
        assert_eq!(guarded(path, || 2), None);
        // A look that did come back (the share is back) clears it.
        with(|l| l.hung.remove(path));
        assert_eq!(guarded(path, || 3), Some(3));
    }
}
