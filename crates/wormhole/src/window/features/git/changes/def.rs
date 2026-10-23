//! Changes as an entry of the feature table (`window::feature`): its tab, what showing it does, its
//! count and words, its keys over the list, the diff it lays over the terminal, what it asks the
//! window's repository read to watch, and the terminals' lifetime.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use super::{Changes, list, view};
use crate::window::Workspace;
use crate::window::facts::WindowFacts;
use crate::window::feature::{BranchChip, CoverDef, FeatureDef, FeatureEvent, RepoRequest, StateDef, TabDef};
use crate::window::features::git::diff::DiffView;

/// The window's `Changes`, from the handle the window keeps. The entry makes it, so it is one.
pub(super) fn typed(entity: &AnyEntity) -> Entity<Changes> {
    entity.clone().downcast::<Changes>().expect("the changes entity is made by this entry's `make`")
}

/// The changed files the active terminal's repository has, if git has answered.
fn files(facts: &WindowFacts<'_>) -> Option<usize> {
    facts.active()?.git.status.as_ref().map(|status| status.files.len())
}

/// The Changes entry: the tab, what showing it does, its count and words, its panel and its keys, the
/// diff over the terminal and the terminals' state.
pub(in crate::window) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Changes,
    tabs: &[TabDef {
        id: TabId::Changes,
        icon: IconName::GitCompare,
        label: "Changes",
        action: ActionId::ShowChanges,
        // Showing the tab reads the repository again (the picker of an earlier visit is closed by
        // `tab_changed`, which the window runs first).
        on_show: Some(|entity, _, session, cx| {
            typed(entity).update(cx, |_, cx| cx.emit(FeatureEvent::Repo(RepoRequest::Refresh { session })));
        }),
        shown: |_, _| true,
        count: Some(|_, facts, _| files(facts).filter(|&n| n > 0)),
        spoken: |_, facts, _| spoken::panel_tab(PanelTab::Changes { files: files(facts) }),
        view: view::tab,
        walk: Some(list::WALK),
        actions: None,
        refresh_ticks: None,
        returns_to_previous: false,
    }],
    tick: Some(|entity, facts, _, cx| {
        typed(entity).update(cx, |changes, cx| {
            changes.follow(facts);
            changes.sync_diff(cx);
        });
    }),
    cover: Some(CoverDef {
        open: |entity, cx| typed(entity).read(cx).diff.is_some(),
        view: |entity, facts, window, cx| {
            typed(entity)
                .update(cx, |changes, cx| changes.render_diff(facts, window, cx))
                .unwrap_or_else(|| div().into_any_element())
        },
        title: |entity, facts, cx| {
            typed(entity).update(cx, |changes, cx| changes.render_diff_title(facts, cx))
        },
        close: |entity, facts, window, cx| {
            if let Some(active) = facts.active() {
                typed(entity).update(cx, |changes, cx| changes.close_diff(active.terminal, window, cx));
            }
        },
        watch: |entity, cx| typed(entity).read(cx).diff.as_ref().and_then(DiffView::watched_path),
        read: |entity, facts, session, observed, cx| {
            typed(entity).update(cx, |changes, cx| changes.refresh_open_diff(facts, session, observed, cx));
        },
    }),
    // Any tab being shown closes the picker of this one.
    tab_changed: Some(|entity, _, cx| {
        typed(entity).update(cx, |changes, cx| {
            if changes.picker.is_open() {
                changes.picker.close();
                cx.notify();
            }
        });
    }),
    branch_chip: Some(BranchChip {
        tab: TabId::Changes,
        pick: |entity, facts, window, cx| {
            let Some(active) = facts.active() else { return };
            typed(entity).update(cx, |changes, cx| {
                changes.picker.toggle(active.id, active.terminal, window, cx);
            });
        },
    }),
    state: Some(STATE),
    ..FeatureDef::NONE
};

const STATE: StateDef = StateDef {
    make: |window, cx| {
        let entity = cx.new(|cx| Changes::new(window, cx));
        Workspace::listen(&entity, window, cx)
    },
    // The diff and the picker belong to the terminal they were opened from.
    selected: Some(|entity, _, cx| typed(entity).update(cx, |changes, cx| changes.selected(cx))),
    focus_in: None,
    record: |_, _, _, _| {},
    opened: |entity, record, _, cx| {
        typed(entity).update(cx, |changes, _| {
            changes.of_mut(record.id);
        });
    },
    // Nothing of Changes is saved with a terminal: its status is read again from git, and the comments
    // not yet pasted go with it.
    closed: |entity, session, cx| typed(entity).update(cx, |changes, cx| changes.forget(session, cx)),
    left: |entity, session, cx| typed(entity).update(cx, |changes, cx| changes.forget(session, cx)),
};
