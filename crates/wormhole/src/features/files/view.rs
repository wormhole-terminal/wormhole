//! How the Files tab draws: a card whose head is the path button and whose body is the tree, in the same
//! virtual list as the Changes rows (`style::panel_list`), or a note where there is no folder.

use std::path::PathBuf;
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::input::InputState;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::file_tree::{Row, RowKind};
use wormhole_text::{keys, text};

use super::def::typed;
use super::head::{HeadFacts, path_button};
use super::rename;
use super::row::{RowLook, entry_row, note_row};
use super::{Files, Line};
use crate::platform::Anchor;
use crate::settings::{SettingsStore, UiColors, ui_in};
use crate::style;
use crate::window::facts::WindowFacts;

/// The tab's content, built inline from `Workspace::render` on the feature's entity.
pub(super) fn tab(
    entity: &AnyEntity,
    facts: &WindowFacts<'_>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    typed(entity).update(cx, |files, cx| files.render_tab(facts, window, cx))
}

/// What the list's rows are drawn from, captured for the render callback.
struct FilesView {
    session: i64,
    selected: Option<PathBuf>,
    head: HeadFacts,
    head_at: Anchor,
    /// The row whose name is a field now.
    rename: Option<(PathBuf, Entity<InputState>)>,
    this: WeakEntity<Files>,
}

impl Files {
    fn render_tab(&mut self, facts: &WindowFacts<'_>, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.follow(facts);
        let Some(active) = facts.active() else { return div().into_any_element() };
        let session = active.id;
        // The tree follows the terminal's folder; this draw is the redraw, so nothing is notified.
        self.follow_terminal_folder(session, active.band_folder(), cx);
        self.sync_list(session);
        let ui = ui_in(facts.profile, cx);
        let panel = div().flex().flex_col().flex_1().min_h_0();
        let Some(of) = self.of.get(&session) else { return panel.into_any_element() };
        let Some(root) = of.root.clone() else {
            return panel
                .child(style::panel_card(
                    ui,
                    style::empty_note(ui, text(keys::FILES_NO_FOLDER))
                        .debug_selector(|| "files-no-folder".into()),
                ))
                .into_any_element();
        };
        let view = Rc::new(FilesView {
            session,
            head: HeadFacts::of(&root, crate::platform::boot_volume_name),
            selected: of.selected.clone(),
            head_at: of.head_at.clone(),
            rename: self.renaming.as_ref().filter(|r| r.session() == session).map(|r| r.target()),
            this: cx.weak_entity(),
        });
        let ring = facts.panel_cursor.map(|(row, shown)| (row, shown.then_some(ui)));
        let list = &of.list;
        panel
            .child(list.with_edges(
                list.element_carded(ring, ui, Line::part, move |line, _, cx| view.render_line(line, cx)),
                ui,
                "files-scrollbar",
                cx,
            ))
            .into_any_element()
    }
}

impl FilesView {
    /// One visible line of the list.
    fn render_line(&self, line: &Line, cx: &App) -> AnyElement {
        let ui = crate::settings::ui(cx);
        match line {
            Line::Head => self.render_head(ui, cx),
            Line::Row(row) if row.is_entry() => self.render_entry(row, ui, cx),
            Line::Row(row) => note_row(row, ui).into_any_element(),
            Line::Spacer => div().h(px(16.0)).into_any_element(),
        }
    }

    fn render_head(&self, ui: UiColors, cx: &App) -> AnyElement {
        let icons = cx.global::<SettingsStore>().icons.as_deref();
        let icon = crate::icons::folder(icons, &self.head.name, false, ui.muted)
            .unwrap_or_else(|| style::icon(IconName::Folder).text_color(ui.muted).into_any_element());
        let (this, session, at) = (self.this.clone(), self.session, self.head_at.clone());
        let below = self.head_at.clone();
        path_button(ui, &self.head, icon, &at, move |_, window, cx| {
            let at = below.below();
            this.update(cx, |files, cx| files.show_path_menu(session, at, window, cx)).ok();
        })
    }

    fn render_entry(&self, row: &Row, ui: UiColors, cx: &App) -> AnyElement {
        let session = self.session;
        let look = RowLook {
            ui,
            icons: cx.global::<SettingsStore>().icons.as_deref(),
            selected: self.selected.as_deref() == Some(row.path.as_path()),
        };
        let (clicked, menu_row) = (row.clone(), row.clone());
        let (this, menu_this) = (self.this.clone(), self.this.clone());
        let field = (self.rename.as_ref())
            .filter(|(path, _)| *path == row.path)
            .map(|(_, input)| rename::field(self.this.clone(), input));
        entry_row(row, &look, field)
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                cx.stop_propagation();
                let row = menu_row.clone();
                menu_this
                    .update(cx, |files, cx| files.show_row_menu(session, row, event.position, window, cx))
                    .ok();
            })
            .on_click(move |event, _, cx| {
                // A right click arrives here too, after the menu it opened.
                if !event.standard_click() {
                    return;
                }
                let row = clicked.clone();
                this.update(cx, |files, cx| files.click(session, &row, cx)).ok();
            })
            .into_any_element()
    }
}

impl Files {
    /// A click on an entry: it is selected; a folder opens or closes, a file shows in the viewer.
    pub(super) fn click(&mut self, session: i64, row: &Row, cx: &mut Context<Self>) {
        match row.kind {
            RowKind::File => self.show_file(session, &row.path, cx),
            RowKind::Folder { .. } => {
                self.select(session, row, cx);
                self.toggle(session, row, cx);
            }
            _ => self.select(session, row, cx),
        }
    }
}
