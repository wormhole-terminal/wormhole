//! The right panel's tabs: a 28 pt strip down the window's right edge that stays when the panel
//! is closed or has no room. A click on a tab opens the panel on it, a click on the open tab
//! closes the panel; the keyboard's ⌘-keys and the menu do the same through `show_tab` and
//! `toggle_notes`. The tab list is one keyboard stop whose ← → switch tabs at once and VoiceOver
//! reads it as a tab list with the counts in words. The strip has no button at its foot: stashing
//! the typed prompt is the palette's, the menu's and the key's.
use wormhole_core::focus::Area;
use wormhole_core::spoken::{self, PanelTab};

use super::*;

impl Workspace {
    /// The strip, in the window's last column.
    pub(super) fn render_panel_strip(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui(cx);
        let session = self.active_session();
        let files = session.git.status.as_ref().map(|s| s.files.len());
        let new_changes = session.git.has_unseen_changes();
        let open_steps = session.tasks.iter().filter(|t| !t.done).count();
        let notes = session.notes.iter().filter(|n| !super::notes::is_blank_note(n.state.read(cx))).count();
        let tabs = self.panel_tab_list(cx);
        let visible = self.notes_visible();
        let ring = self.ring(Area::PanelTabs, window);
        let tab = |id: &'static str, icon: IconName, label: &str, key: ActionId, tab: RightTab| {
            // Only the tab on show while the panel is on screen is "open".
            let open = visible && self.right_tab == tab;
            let new = tab == RightTab::Changes && new_changes && !open;
            let spoken = spoken::panel_tab(match tab {
                RightTab::Notes => PanelTab::Notes { open_steps, notes },
                RightTab::Changes => PanelTab::Changes { files, new },
                RightTab::History => PanelTab::History,
                RightTab::Project => PanelTab::Project,
                RightTab::Usage => PanelTab::Usage,
            });
            style::strip_tab(id, ui, icon, keymap::with_key(label, key), open, new)
                .role(Role::Tab)
                .aria_label(spoken)
                .aria_selected(open)
                .aria_position_in_set(tabs.iter().position(|&t| t == tab).map_or(1, |at| at + 1))
                .aria_size_of_set(tabs.len())
                // The tab shown is where the keys are (automatic activation).
                .when(self.right_tab == tab, |d| d.aria_active_descendant())
                .when(self.right_tab == tab && ring, |d| {
                    d.child(style::focus_ring(ui, style::RingPlacement::Outside, ui.radius_chip))
                })
                .on_click(cx.listener(move |this, _, window, cx| this.press_strip_tab(tab, window, cx)))
        };
        let list = div()
            .id("panel-tabs")
            .role(Role::TabList)
            .aria_label("Right panel")
            .flex()
            .flex_col()
            .items_center()
            .gap(px(style::STRIP_GAP))
            .child(tab(
                "tab-changes",
                IconName::GitCompare,
                "Changes",
                ActionId::ShowChanges,
                RightTab::Changes,
            ))
            .child(tab(
                "tab-history",
                IconName::GitCommitHorizontal,
                "History",
                ActionId::ShowHistory,
                RightTab::History,
            ))
            .child(tab("tab-notes", IconName::FileText, "Notes", ActionId::ShowNotes, RightTab::Notes))
            .child(tab("tab-usage", IconName::Gauge, "Usage", ActionId::ShowUsage, RightTab::Usage))
            // Whenever the terminal has a folder: its colour is set there, and its project made.
            // Last, so the tabs before it never shift when it comes and goes.
            .when(tabs.contains(&RightTab::Project), |d| {
                d.child(tab(
                    "tab-project",
                    IconName::Folder,
                    "Project",
                    ActionId::ShowProject,
                    RightTab::Project,
                ))
            });
        style::panel_strip(ui)
            .id("panel-strip")
            // Not the window's drag area: a press between the buttons does nothing.
            .occlude()
            .child(self.area_container(Area::PanelTabs, list, cx))
            .into_any_element()
    }

    /// A click on a strip tab: the open tab closes the panel, any other opens it on that tab.
    pub(super) fn press_strip_tab(&mut self, tab: RightTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.notes_visible() && self.right_tab == tab {
            self.toggle_notes(window, cx);
        } else {
            self.open_panel_tab(tab, cx);
        }
    }

    /// Show `tab`, opening the panel with a slide when it was closed.
    pub(crate) fn open_panel_tab(&mut self, tab: RightTab, cx: &mut Context<Self>) {
        let before = self.columns().panel;
        self.show_tab(tab, cx);
        self.start_slide(Edge::Panel, before, self.columns().panel, cx);
    }
}
