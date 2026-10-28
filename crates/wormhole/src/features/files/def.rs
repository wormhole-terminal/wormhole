//! Files' entry in the feature table: one tab at the end of the panel, the folders it lists read again
//! while it shows, the viewer it lays over the terminal, and the terminals' lifetime. The tab is behind the `files` flag while it is unfinished.

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::config::{FeatureId, TabId};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::view::spoken::{self, PanelTab};

use super::{Files, keys, view};
use crate::actions::ShowFiles;
use crate::window::Workspace;
use crate::window::feature::{CoverDef, FeatureDef, StateDef, TabDef, show_tab};

/// The window's `Files`, from the handle the window keeps. The entry makes it, so it is one.
pub(super) fn typed(entity: &AnyEntity) -> Entity<Files> {
    entity.clone().downcast::<Files>().expect("the files entity is made by this entry's `make`")
}

/// The Files entry: the tab, what showing it does, and the periodic look at the disk.
pub(crate) const DEF: FeatureDef = FeatureDef {
    id: FeatureId::Files,
    tabs: &[TabDef {
        id: TabId::Files,
        icon: IconName::FolderTree,
        label: "Files",
        action: ActionId::ShowFiles,
        show: |root| root.on_action(show_tab::<ShowFiles>),
        // Showing the tab reads the folders again: they may have changed while it was hidden.
        on_show: Some(|entity, facts, _, cx| {
            typed(entity).update(cx, |files, cx| files.look_again(facts, cx));
        }),
        shown: |_, _| true,
        count: None,
        spoken: |_, _, _| spoken::panel_tab(PanelTab::Files),
        view: view::tab,
        walk: Some(keys::WALK),
        actions: None,
        // Not the window's redraw every so often: the entity redraws it when a read finds a change.
        refresh_ticks: None,
        returns_to_previous: false,
    }],
    tick: Some(|entity, facts, _, cx| {
        typed(entity).update(cx, |files, cx| {
            files.tick_viewer(facts, cx);
            files.tick(facts, cx);
        });
    }),
    // The viewer is laid over the terminal as a diff is; it reads no repository.
    cover: Some(CoverDef {
        open: |entity, cx| typed(entity).read(cx).viewing(),
        view: |entity, facts, window, cx| {
            typed(entity)
                .update(cx, |files, cx| files.render_viewer(facts, window, cx))
                .unwrap_or_else(|| div().into_any_element())
        },
        title: |_, _, _| None,
        close: |entity, facts, window, cx| {
            if let Some(active) = facts.active() {
                typed(entity).update(cx, |files, cx| files.close_viewer(active.terminal, window, cx));
            }
        },
        dismiss: |entity, cx| typed(entity).update(cx, |files, cx| files.dismiss_viewer(cx)),
        repo: None,
    }),
    state: Some(STATE),
    ..FeatureDef::NONE
};

const STATE: StateDef = StateDef {
    make: |window, cx| {
        let entity = cx.new(|_| Files::new());
        Workspace::listen(&entity, window, cx)
    },
    selected: Some(|entity, _, cx| typed(entity).update(cx, |files, cx| files.selected(cx))),
    focus_in: None,
    record: |_, _, _, _| {},
    opened: |_, _, _, _| {},
    // Nothing of the tree is saved with a terminal: it is read from the disk again when shown.
    closed: |entity, session, cx| typed(entity).update(cx, |files, cx| files.forget(session, cx)),
    left: |entity, session, cx| typed(entity).update(cx, |files, cx| files.forget(session, cx)),
};
