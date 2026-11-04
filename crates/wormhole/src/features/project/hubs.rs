//! Each profile's hub, held by the feature and not by the app's shared state. One profile is shown at a
//! time; the hub of every other profile the app has opened is kept beside it, and a profile switch swaps
//! them, as the rest of a profile's data is swapped.
//!
//! Who reads which: a window asks for its own profile's hub ([`hub_of`]), shown or not, so what it trusts
//! and runs follows that profile's approvals. Settings and the status item belong to the shown profile
//! and read [`hub`].

use std::collections::HashMap;

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;

use super::hub::ProjectHub;
use crate::app_state::app;

/// The shown profile's hub and the others'.
struct Hubs {
    shown: ProfileId,
    hub: ProjectHub,
    parked: HashMap<ProfileId, ProjectHub>,
    /// What a profile that has no hub reads: no preferences, nothing trusted, nothing loaded.
    none: ProjectHub,
}

impl Global for Hubs {}

impl Hubs {
    /// The hub of `profile`, shown or not.
    fn of_mut(&mut self, profile: &ProfileId) -> Option<&mut ProjectHub> {
        if *profile == self.shown { Some(&mut self.hub) } else { self.parked.get_mut(profile) }
    }

    fn of(&self, profile: &ProfileId) -> Option<&ProjectHub> {
        if *profile == self.shown { Some(&self.hub) } else { self.parked.get(profile) }
    }
}

/// A profile's hub from its folder, with the ids handed out from now on already taken into account.
fn load(profile: &ProfileId, cx: &App) -> ProjectHub {
    let mut hub = ProjectHub::load_from(app(cx).root.profile(profile).dir());
    hub.ids_start_at(app(cx).next_id());
    hub
}

/// The shown profile's hub: what Settings, the status item and the tests read. A window reads
/// [`hub_of`].
pub(crate) fn hub(cx: &App) -> &ProjectHub {
    &cx.global::<Hubs>().hub
}

/// The hub of `profile`, shown or not: a window's own, so its trust and preferences are the profile's it
/// lives in. A profile no window has opened has no hub, and reads one that trusts nothing.
pub(crate) fn hub_of<'a>(profile: &ProfileId, cx: &'a App) -> &'a ProjectHub {
    let hubs = cx.global::<Hubs>();
    hubs.of(profile).unwrap_or(&hubs.none)
}

/// [`hub_of`] to change; `None` for a profile that has no hub, where nothing is changed.
pub(crate) fn hub_of_mut<'a>(profile: &ProfileId, cx: &'a mut App) -> Option<&'a mut ProjectHub> {
    cx.global_mut::<Hubs>().of_mut(profile)
}

/// Every profile that has a hub: the shown one first.
pub(super) fn profiles(cx: &App) -> Vec<ProfileId> {
    let hubs = cx.global::<Hubs>();
    std::iter::once(hubs.shown.clone()).chain(hubs.parked.keys().cloned()).collect()
}

/// The profile whose hub [`hub`] reads.
pub(crate) fn shown_profile(cx: &App) -> &ProfileId {
    &cx.global::<Hubs>().shown
}

#[cfg(test)]
pub(crate) fn hub_mut(cx: &mut App) -> &mut ProjectHub {
    &mut cx.global_mut::<Hubs>().hub
}

/// The shown profile is in `AppState` (at launch, after a switch): its hub is the one `hub` gives. The
/// first call, at launch, reads it from the profile's folder.
pub(crate) fn profile_shown(cx: &mut App) {
    let shown = app(cx).active_profile().clone();
    if !cx.has_global::<Hubs>() {
        let hub = load(&shown, cx);
        cx.set_global(Hubs { shown, hub, parked: HashMap::new(), none: ProjectHub::default() });
        return;
    }
    if cx.global::<Hubs>().shown == shown {
        return;
    }
    let incoming = match cx.global_mut::<Hubs>().parked.remove(&shown) {
        Some(mut hub) => {
            hub.ids_start_at(app(cx).next_id());
            hub
        }
        None => load(&shown, cx),
    };
    let hubs = cx.global_mut::<Hubs>();
    let leaving = std::mem::replace(&mut hubs.hub, incoming);
    let left = std::mem::replace(&mut hubs.shown, shown);
    hubs.parked.insert(left, leaving);
}

/// A profile other than the shown one was opened (a window lives in it): its hub is read now, as the rest
/// of its data is.
pub(crate) fn profile_opened(profile: &ProfileId, cx: &mut App) {
    if !cx.has_global::<Hubs>() || cx.global::<Hubs>().shown == *profile {
        return;
    }
    if !cx.global::<Hubs>().parked.contains_key(profile) {
        let hub = load(profile, cx);
        cx.global_mut::<Hubs>().parked.insert(profile.clone(), hub);
    }
}

/// A profile was deleted: its hub goes with it.
pub(crate) fn profile_deleted(profile: &ProfileId, cx: &mut App) {
    if cx.has_global::<Hubs>() {
        cx.global_mut::<Hubs>().parked.remove(profile);
    }
}

/// `id` was just handed out in `profile`: whatever notes base is kept under it there belongs to a
/// session that is gone.
pub(crate) fn id_allocated(profile: &ProfileId, id: i64, cx: &mut App) {
    if !cx.has_global::<Hubs>() {
        return;
    }
    if let Some(hub) = cx.global_mut::<Hubs>().of_mut(profile) {
        hub.id_allocated(id);
    }
}
