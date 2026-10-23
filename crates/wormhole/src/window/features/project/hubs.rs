//! Each profile's hub, held by the feature and not by the app's shared state. One profile is shown at a
//! time and its hub is what everything reads ([`hub`]); the hub of every other profile the app has opened
//! is kept beside it, and a profile switch swaps them, as the rest of a profile's data is swapped. A
//! window of a profile that isn't shown reads the shown profile's, as it always has.

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
}

impl Global for Hubs {}

impl Hubs {
    /// The hub of `profile`, shown or not.
    fn of_mut(&mut self, profile: &ProfileId) -> Option<&mut ProjectHub> {
        if *profile == self.shown { Some(&mut self.hub) } else { self.parked.get_mut(profile) }
    }
}

/// A profile's hub from its folder, with the ids handed out from now on already taken into account.
fn load(profile: &ProfileId, cx: &App) -> ProjectHub {
    let mut hub = ProjectHub::load_from(app(cx).root.profile(profile).dir());
    hub.ids_start_at(app(cx).next_id());
    hub
}

/// The shown profile's hub.
pub(crate) fn hub(cx: &App) -> &ProjectHub {
    &cx.global::<Hubs>().hub
}

pub(crate) fn hub_mut(cx: &mut App) -> &mut ProjectHub {
    &mut cx.global_mut::<Hubs>().hub
}

/// The shown profile is in `AppState` (at launch, after a switch): its hub is the one `hub` gives. The
/// first call, at launch, reads it from the profile's folder.
pub(crate) fn profile_shown(cx: &mut App) {
    let shown = app(cx).active_profile().clone();
    if !cx.has_global::<Hubs>() {
        let hub = load(&shown, cx);
        cx.set_global(Hubs { shown, hub, parked: HashMap::new() });
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
