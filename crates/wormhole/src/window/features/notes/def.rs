//! Notes' entry in the feature table: its tab and the hooks the window calls. Each takes the window's
//! entity once, downcasts it and works on the whole set (every row, every terminal being closed).
use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::notes::stake;

use super::{Notes, list, view};
use crate::actions::{NewNote, StashPrompt};
use crate::sidebar::{RowExtra, RowExtras};
use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{CloseHooks, FeatureDef, Root, Stake, StateDef, TabDef};

/// The window's `Notes`, from the handle the window keeps. The entry makes it, so it is one.
pub(super) fn typed(entity: &AnyEntity) -> Entity<Notes> {
    entity.clone().downcast::<Notes>().expect("the notes entity is made by this entry's `make`")
}

/// The notes entry: the Notes tab and its stash button, the keys for a new note and the stash, the
/// count on a terminal's row, the close question's stake and the terminals' notes and tasks.
pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Notes,
    tabs: &[TabDef {
        id: TabId::Notes,
        icon: IconName::FileText,
        label: "Notes",
        action: ActionId::ShowNotes,
        on_show: None,
        shown: |_, _| true,
        count: None,
        spoken: view::spoken_tab,
        view: view::tab,
        walk: Some(list::WALK),
        actions: Some(view::stash_button),
        refresh_ticks: None,
        returns_to_previous: false,
    }],
    bind: Some(bind),
    row_extras: Some(row_extras),
    close: Some(CloseHooks { stake: Some(close_stake), ..CloseHooks::NONE }),
    tick: Some(|entity, _, window, cx| {
        // The backstop for a new note that never got the focus it would be thrown away on losing.
        typed(entity).update(cx, |notes, cx| notes.prune_blank_notes(window, cx));
    }),
    state: Some(STATE),
    ..FeatureDef::NONE
};

/// New note and Stash prompt, for the terminal on screen at this frame.
fn bind(entity: &AnyEntity, facts: &WindowFacts<'_>, root: Root, _: &App) -> Root {
    let Some(active) = facts.active() else { return root };
    let (session, terminal, width) = (active.id, active.terminal.clone(), f32::from(facts.panel_width));
    let (new, stash) = (typed(entity), typed(entity));
    root.on_action(move |_: &NewNote, window, cx| {
        new.update(cx, |notes, cx| notes.new_note(session, None, width, window, cx));
    })
    .on_action(move |_: &StashPrompt, window, cx| {
        stash.update(cx, |notes, cx| notes.stash_prompt(session, &terminal, width, window, cx));
    })
}

/// The notes and tasks still open, on each terminal's row.
fn row_extras(entity: &AnyEntity, _: &WindowFacts<'_>, extras: &mut RowExtras<'_>, cx: &App) {
    let notes = typed(entity);
    let notes = notes.read(cx);
    for (id, slot) in extras.iter_mut() {
        let count = notes.open_items(&[id]);
        *slot = (count > 0).then_some(RowExtra { count });
    }
}

/// What closing these terminals discards.
fn close_stake(entity: &AnyEntity, sessions: &[i64], cx: &App) -> Option<Stake> {
    let count = typed(entity).read(cx).open_items(sessions);
    stake::discarded(count).map(|sentence| Stake { count, sentence: sentence.to_string() })
}

const STATE: StateDef = StateDef {
    make: |window, cx| {
        let entity = cx.new(|_| Notes::new());
        Workspace::listen(&entity, window, cx)
    },
    selected: Some(|entity, _, cx| typed(entity).update(cx, |notes, _| notes.selected())),
    focus_in: Some(|entity, facts, session, window, cx| {
        let width = f32::from(facts.panel_width);
        typed(entity).update(cx, |notes, cx| notes.focus_in(session, width, window, cx))
    }),
    record: |entity, session, record, cx| typed(entity).read(cx).record(session, record, cx),
    opened: |entity, record, window, cx| {
        typed(entity).update(cx, |notes, cx| notes.open(record, window, cx));
    },
    closed: |entity, session, cx| typed(entity).update(cx, |notes, _| notes.forget(session)),
    // Nothing is stored beside the record, which the other window reads back, so leaving and closing
    // are the same here.
    left: |entity, session, cx| typed(entity).update(cx, |notes, _| notes.forget(session)),
};
