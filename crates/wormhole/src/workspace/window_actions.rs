//! A window's own actions: its terminals, panes and panels, handled on its `Workspace`'s root
//! (so the focus must be inside it).
//!
//! Part of the window: it must not use the surfaces above it (the command palette, the shortcut
//! sheet, the Settings and Studio windows, the status item, the app root).

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::sidebar::order::shortcut_target;

use crate::actions::{
    ActualSize, Bigger, CloseTerminal, CloseWindow, CopyPath, Find, FindNext, FindPrevious, FocusNextArea,
    FocusPaneDown, FocusPaneLeft, FocusPaneRight, FocusPaneUp, FocusPreviousArea, FocusTerminalOrPanel,
    GoToGroup, GoToTerminal, LayoutColumns, LayoutGrid, LayoutRows, LayoutSingle, LeaveSplit, NewNote,
    NewSession, NewTerminalInWorktree, NewTerminalNoGroup, NewTerminalWith, NextGroup, NextTerminal,
    PinTerminal, PreviousGroup, PreviousTerminal, RevealInFinder, RunProjectAction, ShowChanges, ShowHistory,
    ShowNotes, ShowProject, ShowUsage, Smaller, SplitDown, SplitRight, StashPrompt, ToggleRightPanel,
    ToggleSidebar, UseSelectionForFind, ZoomPane,
};
use crate::settings::settings;
use crate::terminal_view::FONT_SIZES;
use crate::workspace::{RightTab, Workspace, sidebar};

/// The zoom (points over the Settings size) after one more step, kept within [`FONT_SIZES`].
fn zoomed(base: f32, zoom: f32, step: f32) -> f32 {
    (base + zoom + step).clamp(*FONT_SIZES.start(), *FONT_SIZES.end()) - base
}

impl Workspace {
    /// The window's own actions, on its root element. An action that can't do anything for the
    /// terminal in front isn't handled at all, which is what greys its menu item out.
    pub(super) fn with_actions(&self, root: Div, cx: &mut Context<Self>) -> Div {
        use wormhole_core::view::panes::{Axis, Direction, Preset};
        let has_folder = self.active_terminal().read(cx).cwd.is_some();
        // A worktree needs a repository: the terminal's folder has a git status.
        let has_repo = self.active_session().git.status.is_some();
        let split = self.active_split().is_some();
        root.on_action(cx.listener(|this, _: &NewSession, window, cx| this.new_session(window, cx)))
            .on_action(cx.listener(|this, _: &NewTerminalNoGroup, window, cx| {
                let cwd = this.default_cwd(cx);
                this.open_session_in(Some(None), None, cwd, window, cx);
            }))
            .when(has_repo, |root| {
                root.on_action(cx.listener(|this, _: &NewTerminalInWorktree, window, cx| {
                    this.new_terminal_in_worktree(window, cx)
                }))
            })
            // Panes: a split or a layout only with the room for it, the rest only
            // in a split, so the menu greys them out otherwise.
            .when(self.can_split(Axis::Row), |root| {
                root.on_action(
                    cx.listener(|this, _: &SplitRight, window, cx| this.split_pane(Axis::Row, window, cx)),
                )
            })
            .when(self.can_split(Axis::Column), |root| {
                root.on_action(
                    cx.listener(|this, _: &SplitDown, window, cx| this.split_pane(Axis::Column, window, cx)),
                )
            })
            .when(split, |root| {
                root.on_action(cx.listener(|this, _: &ZoomPane, _, cx| this.toggle_zoom(cx)))
                    .on_action(cx.listener(|this, _: &LeaveSplit, _, cx| {
                        this.leave_split(this.active, cx);
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneLeft, window, cx| {
                        this.focus_pane_toward(Direction::Left, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneRight, window, cx| {
                        this.focus_pane_toward(Direction::Right, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneUp, window, cx| {
                        this.focus_pane_toward(Direction::Up, window, cx)
                    }))
                    .on_action(cx.listener(|this, _: &FocusPaneDown, window, cx| {
                        this.focus_pane_toward(Direction::Down, window, cx)
                    }))
            })
            .when(self.can_layout(Preset::TwoColumns), |root| {
                root.on_action(cx.listener(|this, _: &LayoutColumns, window, cx| {
                    this.apply_layout(Preset::TwoColumns, window, cx)
                }))
            })
            .when(self.can_layout(Preset::TwoRows), |root| {
                root.on_action(cx.listener(|this, _: &LayoutRows, window, cx| {
                    this.apply_layout(Preset::TwoRows, window, cx)
                }))
            })
            .when(self.can_layout(Preset::Grid), |root| {
                root.on_action(
                    cx.listener(|this, _: &LayoutGrid, window, cx| {
                        this.apply_layout(Preset::Grid, window, cx)
                    }),
                )
            })
            .on_action(
                cx.listener(|this, _: &LayoutSingle, window, cx| {
                    this.apply_layout(Preset::Single, window, cx)
                }),
            )
            .on_action(cx.listener(|this, with: &NewTerminalWith, window, cx| {
                this.new_session_with(Some(with.0.to_string()), window, cx)
            }))
            .on_action(cx.listener(|this, _: &CloseTerminal, window, cx| this.close_selected(window, cx)))
            .on_action(cx.listener(|this, _: &CloseWindow, window, cx| this.close_window(window, cx)))
            .on_action(cx.listener(|this, _: &NextTerminal, window, cx| this.step_session(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &PreviousTerminal, window, cx| this.step_session(-1, window, cx)),
            )
            .on_action(cx.listener(|this, n: &GoToTerminal, window, cx| this.go_to_numbered(n.0, window, cx)))
            .on_action(cx.listener(|this, _: &NextGroup, window, cx| this.step_group(true, window, cx)))
            .on_action(cx.listener(|this, _: &PreviousGroup, window, cx| this.step_group(false, window, cx)))
            .on_action(cx.listener(|this, n: &GoToGroup, window, cx| this.go_to_group(n.0, window, cx)))
            .on_action(cx.listener(|this, _: &PinTerminal, _, cx| this.toggle_pin(this.active, cx)))
            .when(has_folder, |root| {
                root.on_action(cx.listener(|this, _: &RevealInFinder, _, cx| {
                    if let Some(cwd) = this.active_terminal().read(cx).cwd.clone() {
                        sidebar::reveal_in_finder(&cwd, cx);
                    }
                }))
                .on_action(cx.listener(|this, _: &CopyPath, _, cx| {
                    if let Some(cwd) = this.active_terminal().read(cx).cwd.clone() {
                        cx.write_to_clipboard(ClipboardItem::new_string(cwd.to_string_lossy().into_owned()));
                    }
                }))
            })
            .on_action(cx.listener(|this, action: &RunProjectAction, window, cx| {
                this.run_project_action(&action.0, window, cx)
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &ToggleRightPanel, window, cx| this.toggle_notes(window, cx)))
            .on_action(cx.listener(|this, _: &ShowChanges, _, cx| this.show_tab(RightTab::Changes, cx)))
            .on_action(cx.listener(|this, _: &ShowHistory, _, cx| this.show_tab(RightTab::History, cx)))
            .when(self.usage_tab_on(cx), |root| {
                root.on_action(cx.listener(|this, _: &ShowUsage, _, cx| this.show_tab(RightTab::Usage, cx)))
            })
            // Any terminal whose folder is known has the tab, with a project or not.
            .when(has_folder, |root| {
                root.on_action(
                    cx.listener(|this, _: &ShowProject, _, cx| this.show_tab(RightTab::Project, cx)),
                )
            })
            .on_action(
                cx.listener(|this, _: &FocusTerminalOrPanel, window, cx| this.toggle_focus(window, cx)),
            )
            .on_action(cx.listener(|this, _: &FocusNextArea, window, cx| this.step_focus_area(1, window, cx)))
            .on_action(
                cx.listener(|this, _: &FocusPreviousArea, window, cx| this.step_focus_area(-1, window, cx)),
            )
            .on_action(cx.listener(|this, _: &Bigger, _, cx| this.zoom_font(Some(1.0), cx)))
            .on_action(cx.listener(|this, _: &Smaller, _, cx| this.zoom_font(Some(-1.0), cx)))
            .on_action(cx.listener(|this, _: &ActualSize, _, cx| this.zoom_font(None, cx)))
            .when(crate::workspace::notes::notes_on(cx), |root| {
                root.on_action(cx.listener(|this, _: &ShowNotes, _, cx| this.show_tab(RightTab::Notes, cx)))
                    .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(None, window, cx)))
                    .on_action(cx.listener(|this, _: &StashPrompt, window, cx| this.stash_prompt(window, cx)))
            })
            // Find works on the terminal in front, so not while a diff covers it.
            .when(self.diff_view.is_none(), |root| {
                root.on_action(cx.listener(|this, _: &Find, window, cx| {
                    let terminal = this.active_terminal().clone();
                    terminal.update(cx, |terminal, cx| {
                        let seed = terminal.find_seed();
                        terminal.open_find(seed, window, cx);
                    });
                }))
                .on_action(cx.listener(|this, _: &FindNext, window, cx| this.find_step(1, window, cx)))
                .on_action(cx.listener(|this, _: &FindPrevious, window, cx| this.find_step(-1, window, cx)))
                .on_action(cx.listener(|this, _: &UseSelectionForFind, window, cx| {
                    let terminal = this.active_terminal().clone();
                    terminal.update(cx, |terminal, cx| terminal.use_selection_for_find(window, cx));
                }))
            })
    }

    /// ⌘G / ⇧⌘G: the front terminal's next or previous match.
    fn find_step(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        let terminal = self.active_terminal().clone();
        terminal.update(cx, |terminal, cx| terminal.find_step(delta, window, cx));
    }

    /// ⌘1–9: the terminal whose keycap says ⌘n; past the last numbered one, that one.
    fn go_to_numbered(&mut self, n: u8, window: &mut Window, cx: &mut Context<Self>) {
        let numbered = self.session_listing(cx).numbered();
        if let Some(ix) = shortcut_target(&numbered, usize::from(n)) {
            self.activate(ix, window, cx);
        }
    }

    /// ⌘+ / ⌘- (`Some(±1)`) and ⌘0 (`None`): this window's terminal font a point bigger or
    /// smaller, or back to the size in Settings. A view setting, as in Terminal.app: not saved.
    pub(super) fn zoom_font(&mut self, step: Option<f32>, cx: &mut Context<Self>) {
        let zoom = step.map_or(0.0, |step| zoomed(settings(cx).font_size, self.font_zoom, step));
        if zoom == self.font_zoom {
            return;
        }
        self.font_zoom = zoom;
        for pane in self.sessions.iter().map(|session| &session.pane) {
            pane.terminal.update(cx, |terminal, cx| terminal.set_font_zoom(zoom, cx));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::zoomed;

    #[test]
    fn font_zoom_stays_within_what_settings_allow() {
        assert_eq!(zoomed(14.0, 0.0, 1.0), 1.0);
        assert_eq!(zoomed(14.0, 3.0, -1.0), 2.0);
        assert_eq!(zoomed(35.0, 1.0, 1.0), 1.0, "36 at most");
        assert_eq!(zoomed(9.0, -1.0, -1.0), -1.0, "8 at least");
    }
}
