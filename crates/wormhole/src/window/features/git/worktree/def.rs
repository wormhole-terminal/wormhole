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
use crate::window::feature::{FeatureDef, Root, SpawnAdd, StateDef};

/// The window's `Worktrees`, from the handle the window keeps. The entry makes it, so it is one.
pub(super) fn typed(entity: &AnyEntity) -> Entity<Worktrees> {
    entity.clone().downcast::<Worktrees>().expect("the worktrees entity is made by this entry's `make`")
}

/// The worktrees entry. Last in the table: its environment comes after the project's, so a worktree's own
/// ports win over what a project says.
pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Worktrees,
    tabs: &[],
    close_extra: Some(|_, facts, sessions, of, cx| close::extra(facts, sessions, of, cx)),
    bind: Some(bind),
    row_extras: None,
    close_stake: None,
    menu: None,
    tick: None,
    profile_shown: None,
    profiles: None,
    refresh: None,
    approves_programs: None,
    // A worktree Wormhole made: its own ports, whatever the project says.
    spawn: Some(|cwd, cx| SpawnAdd {
        env: cwd
            .and_then(|cwd| app(cx).worktrees.find(cwd))
            .map(|made| made.env().into_iter().map(|(name, value)| (name.to_string(), value)).collect())
            .unwrap_or_default(),
        shell: None,
    }),
    cover: None,
    tab_changed: None,
    repo: None,
    state: Some(STATE),
};

/// New Terminal in New Worktree… for the terminal in front. A worktree needs a repository: the action is
/// not handled (so the menu greys it out) while the terminal's folder has no git status.
fn bind(entity: &AnyEntity, facts: &WindowFacts<'_>, root: Root, cx: &App) -> Root {
    let Some(active) = facts.active().filter(|active| active.git.status.is_some()) else { return root };
    let (this, folder) = (typed(entity), active.band_folder(cx));
    root.on_action(move |_: &NewTerminalInWorktree, window, cx| {
        this.update(cx, |worktrees, cx| worktrees.new_terminal_in_worktree(folder.clone(), window, cx));
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
