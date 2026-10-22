//! Which profile's files the app reads now. Settings, fonts, icon themes and the theme list are
//! all looked up through one answer here, so a profile switch (`AppState::switch_profile`) moves
//! every one of them at once and a helper far from the app state needn't be handed a path.

use std::sync::RwLock;

use wormhole_core::paths::{DataRoot, ProfilePaths};

static ACTIVE: RwLock<Option<ProfilePaths>> = RwLock::new(None);

/// The profile Wormhole is in. Default's (the data folder's own files) until one is set.
pub fn paths() -> ProfilePaths {
    ACTIVE
        .read()
        .ok()
        .and_then(|active| active.clone())
        .unwrap_or_else(|| DataRoot::current().default_profile())
}

pub fn set(paths: ProfilePaths) {
    if let Ok(mut active) = ACTIVE.write() {
        *active = Some(paths);
    }
}
