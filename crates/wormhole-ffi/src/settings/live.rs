//! The settings the app holds of each profile while it runs (every open [`SettingsHandle`](super::SettingsHandle)'s),
//! so that a change made in Wormhole by something that is not a Settings control (Don't Keep, the first run's steps, an
//! import, a pin, a folder's colour, the terminal welcome) goes through them, as the GPUI app's goes through the
//! profile's settings store (`settings::update_settings` and `update_in`, `settings/profile.rs:182-198`). Written around
//! them from a load of the file, such a change is undone by the next write of a handle that has not read the file
//! again, and that load writes back every key as it read it, so a change of another key made in between is lost.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use wormhole_core::config::Settings;
use wormhole_core::storage::paths::ProfilePaths;

use super::handle::Shared;
use super::state::State;

static LIVE: Mutex<Vec<(PathBuf, Weak<Shared>)>> = Mutex::new(Vec::new());

/// One change at a time where no handle holds the profile's settings: two writers that each read the file and then
/// save (a pin and a folder colour) would each write a copy that lacks the other's change.
static UNHELD: Mutex<()> = Mutex::new(());

/// The folder as the key: the same profile, however its data folder was spelled.
fn key(dir: &Path) -> PathBuf {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

/// `shared` holds the settings of the profile whose folder is `dir`, for as long as it lives.
pub(super) fn register(dir: &Path, shared: &Arc<Shared>) {
    let mut live = LIVE.lock().unwrap_or_else(PoisonError::into_inner);
    live.retain(|(_, held)| held.strong_count() > 0);
    live.push((key(dir), Arc::downgrade(shared)));
}

fn of(dir: &Path) -> Vec<Arc<Shared>> {
    let dir = key(dir);
    let live = LIVE.lock().unwrap_or_else(PoisonError::into_inner);
    live.iter().filter(|(held, _)| *held == dir).filter_map(|(_, shared)| shared.upgrade()).collect()
}

/// Changes `profile`'s settings as an edit made in Wormhole: in the settings the app holds of it, read again from the
/// file first, saved by them and told to their window; every other holder reads the file again at once. With none
/// open, in the file as it is now.
pub(crate) fn update(profile: &ProfilePaths, change: impl FnOnce(&mut Settings)) {
    let live = of(profile.dir());
    let Some((first, others)) = live.split_first() else {
        let _one_at_a_time = UNHELD.lock().unwrap_or_else(PoisonError::into_inner);
        State::load(profile).update(change);
        return;
    };
    {
        let mut state = first.lock();
        state.poll();
        state.update(change);
    }
    // Told whether or not the values moved: what was read again may be the theme's or a font's, as after any wake.
    first.wake();
    for other in others {
        if other.lock().poll() {
            other.wake();
        }
    }
}
