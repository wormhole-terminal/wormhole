//! Worktrees' entry in the feature table: the action that asks for a worktree (bound only where the
//! terminal in front is in a repository), the extra the close question gets, and the ports a terminal
//! in a worktree Wormhole made starts with. No tab, nothing saved with a terminal.

use gpui_kit::*;
use wormhole_core::config::FeatureId;

use super::{Worktrees, close};
use crate::actions::NewTerminalInWorktree;
use crate::app_state::app;
use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{CloseHooks, FeatureDef, Root, SpawnAdd, StateDef};

/// The window's `Worktrees`, from the handle the window keeps. The entry makes it, so it is one.
pub(super) fn typed(entity: &AnyEntity) -> Entity<Worktrees> {
    entity.clone().downcast::<Worktrees>().expect("the worktrees entity is made by this entry's `make`")
}

/// The worktrees entry. Last in the table: its environment comes after the project's, so a worktree's own
/// ports win over what a project says.
pub(crate) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Worktrees,
    tabs: &[],
    close: Some(CloseHooks {
        extra: Some(|_, facts, sessions, of, cx| close::extra(facts, sessions, of, cx)),
        ..CloseHooks::NONE
    }),
    bind: Some(bind),
    // A worktree Wormhole made: its own ports, whatever the project says.
    // A window moved to another profile: a dialog that is up follows that profile's approvals.
    refresh: Some(|entity, facts, cx| {
        typed(entity).update(cx, |worktrees, _| worktrees.follow_profile(facts.profile))
    }),
    spawn: Some(|_, cwd, cx| SpawnAdd {
        env: cwd
            .and_then(|cwd| app(cx).worktrees.find(cwd))
            .map(|made| made.env().into_iter().map(|(name, value)| (name.to_string(), value)).collect())
            .unwrap_or_default(),
        shell: None,
    }),
    state: Some(STATE),
    ..FeatureDef::NONE
};

/// New Terminal in New Worktree… for the terminal in front. A worktree needs a repository: the action is
/// not handled (so the menu greys it out) while the terminal's folder has no git status.
fn bind(entity: &AnyEntity, facts: &WindowFacts<'_>, root: Root, cx: &App) -> Root {
    let Some(active) = facts.active().filter(|active| active.git.status.is_some()) else { return root };
    let (this, folder, profile) = (typed(entity), active.band_folder(cx), facts.profile.clone());
    root.on_action(move |_: &NewTerminalInWorktree, window, cx| {
        this.update(cx, |worktrees, cx| {
            worktrees.new_terminal_in_worktree(&profile, folder.clone(), window, cx)
        });
    })
}

const STATE: StateDef = StateDef {
    make: |window, cx| {
        let entity = cx.new(|_| Worktrees::new());
        Workspace::listen(&entity, window, cx)
    },
    selected: None,
    focus_in: None,
    // Nothing of it is a terminal's: what a worktree is lives in the app's list of the ones it made.
    record: |_, _, _, _| {},
    opened: |_, _, _, _| {},
    closed: |_, _, _| {},
    left: |_, _, _| {},
};
