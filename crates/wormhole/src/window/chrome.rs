//! The window's chrome: the 38 pt title rows across the top. The
//! sidebar's collapse button sits at one spot of the window whether the sidebar is open or
//! hidden (so does the right panel's toggle in the top right corner, and the traffic lights
//! macOS draws at the left); the sidebar's title row leaves room for the lights above the
//! "Terminals" header, the terminal's carries the selected terminal's breadcrumb and the split
//! buttons, and the right panel's its tab bar.
use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::focus::Area;
use wormhole_core::view::layout::{Edge, PanelToggle};
use wormhole_core::view::spoken::{self, PanelTab};

use crate::app_state::app;
use crate::settings::{store_in, ui_in};
use crate::style;
use crate::window::Workspace;
use crate::window::focus::TitleButton;
use wormhole_core::config::TabId;

/// The traffic lights start 12 pt in, on the line the sidebar's "Terminals" starts on (the user's
/// call over the board's 20, which left the lights hanging right of everything below them). They
/// are assumed to be three 12 pt ones 8 apart (x 12 to 64); later macOS versions draw them larger, so the span is measured (`platform::window`)
/// by whatever has to clear the real lights.
const LIGHTS_X: f32 = 12.0;
const ASSUMED_LIGHTS_SPAN: f32 = 52.0;
/// The height of AppKit's frame round each light, which is what its position places.
const LIGHT_FRAME: f32 = 14.0;
/// The sidebar's collapse button starts here (its glyph is centred in it, at x 82–98), at
/// 12 after the lights. It doesn't follow the sidebar; it moves right only
/// when macOS draws the lights wider than assumed, so it never covers them.
const COLLAPSE_X: f32 = 76.0;
/// The gap between the lights and the collapse button's frame (64 to 76).
const AFTER_LIGHTS: f32 = COLLAPSE_X - (LIGHTS_X + ASSUMED_LIGHTS_SPAN);
/// The profile chip starts this far after the sidebar's toggle (x 110 with the assumed lights).
const AFTER_TOGGLE: f32 = 6.0;
/// A title row's own content starts this far after the collapse button when the sidebar is
/// hidden and the button sits over it.
const AFTER_COLLAPSE: f32 = 8.0;
/// The panel toggle's distance from the window's right edge: its glyph ends at the 12 pt edge.
const PANEL_TOGGLE_RIGHT: f32 = style::PANEL_INSET - (style::HIT_MIN - style::TITLE_GLYPH) / 2.0;
/// What the toggle takes from the title row or the tab bar it floats over: it sits flush against the
/// row's last button, as the row's buttons sit against each other (12 pt between their glyphs).
pub(super) const PANEL_TOGGLE_SLOT: f32 = PANEL_TOGGLE_RIGHT + style::HIT_MIN;
/// The terminal's title row's padding beside the open sidebar, from the column's edge: with the
/// breadcrumb's own 8 the folder's icon starts on the terminal surface's 12 pt edge.
const BESIDE_COLUMN: f32 = 4.0;
/// Its padding beside the open panel: the room round the last button's glyph, so the glyph ends on
/// the surface's 12 pt edge.
const BESIDE_PANEL: f32 = PANEL_TOGGLE_RIGHT;
/// A title row's own padding at an edge without the lights or the toggle: the room round a
/// button's glyph, so the glyph and text line up with the 12 pt edge below.
const TITLE_ROW_PAD: f32 = PANEL_TOGGLE_RIGHT;

/// A stretch of title bar: pressing there drags the window, and a double-click does what the
/// system setting says (zoom or minimize). The app owns the title bar (`app_owns_titlebar_drag`):
/// otherwise AppKit takes every double-click in its strip, a button's too.
///
/// A press reaches the front-most element first, so what sits on the row keeps it: a Kit button
/// marks it handled (`prevent_default`), and a title row's icon button or clickable strip takes
/// the pointer (`.occlude()`, as `IconButton::in_title_row` does). A strip laid over a window
/// goes first among its siblings, behind what it covers.
pub(crate) trait TitleDrag: InteractiveElement + Sized {
    fn title_drag(self) -> Self {
        self.window_control_area(WindowControlArea::Drag).on_mouse_down(
            MouseButton::Left,
            |event: &MouseDownEvent, window, _| {
                if window.default_prevented() {
                    return;
                }
                if event.click_count == 2 {
                    window.titlebar_double_click();
                } else {
                    window.start_window_move();
                }
            },
        )
    }
}

impl<E: InteractiveElement> TitleDrag for E {}

/// Where macOS draws the traffic lights (the close button's frame): from the 12 pt inset,
/// centred in a top row `row` tall. Every window of the app puts them in a title row; a workspace
/// with a taller tab bar moves them down to its middle.
pub(crate) fn traffic_light_position(row: f32) -> Point<Pixels> {
    point(px(LIGHTS_X), px((row - LIGHT_FRAME) / 2.0))
}

/// Where the traffic lights end in this window, from the left edge: for whatever has to clear
/// the real lights (the Studio's title, the sidebar's collapse button).
pub(crate) fn lights_end(window: &Window) -> f32 {
    LIGHTS_X + crate::platform::window::span(window).unwrap_or(ASSUMED_LIGHTS_SPAN)
}

/// Where the sidebar's collapse button starts, after lights that end at `lights_end`.
fn collapse_x(lights_end: f32) -> f32 {
    COLLAPSE_X.max(lights_end + AFTER_LIGHTS)
}

/// What the collapse button takes from a title row it sits over, and the gap after it.
fn collapse_slot(lights_end: f32) -> f32 {
    collapse_x(lights_end) + style::HIT_MIN + AFTER_COLLAPSE
}

impl Workspace {
    /// The profile this window is in and its colour, when that is not the profile Wormhole is in
    /// (the odd window), as the theme draws it.
    pub(super) fn odd_profile(&self, cx: &App) -> Option<(wormhole_core::config::profile::Profile, Hsla)> {
        let state = app(cx);
        if &self.profile == state.active_profile() {
            return None;
        }
        let profile = state.profiles.get(&self.profile)?.clone();
        let color = crate::theme_bridge::hsla(profile.color.color(&store_in(&self.profile, cx).tokens));
        Some((profile, color))
    }

    /// The edge across the top of the odd window, in its profile's colour. Over everything else
    /// and out of the way of the pointer, so the title rows keep their height.
    pub(super) fn render_profile_edge(&self, cx: &App) -> Option<AnyElement> {
        let (_, color) = self.odd_profile(cx)?;
        Some(div().absolute().top_0().left_0().right_0().child(style::profile_edge(color)).into_any_element())
    }

    /// The sidebar's collapse button and the right panel's toggle. They float over the title
    /// rows at fixed spots in the window, so they never move as the panels open and close.
    pub(super) fn render_window_controls(&self, window: &Window, cx: &mut Context<Self>) -> [AnyElement; 2] {
        let ui = ui_in(&self.profile, cx);
        let sidebar_open = self.panes.sidebar_open;
        let sidebar = div()
            .id("sidebar-toggle")
            .occlude()
            .absolute()
            .top_0()
            .left(px(collapse_x(lights_end(window))))
            .h(px(style::TITLE_ROW_H))
            .flex()
            .items_center()
            .child(
                style::icon_button(
                    "toggle-sidebar",
                    ui,
                    IconName::PanelLeft,
                    keymap::with_key(
                        if sidebar_open { "Hide sidebar" } else { "Show sidebar" },
                        ActionId::ToggleSidebar,
                    ),
                )
                .in_title_row()
                .focus_ring(self.title_ring(TitleButton::Sidebar, window, cx))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx))),
            );
        let look = self.panes.panel_toggle(self.window_width);
        // One glyph in every state; the tooltip says what a click does.
        let tooltip = match look {
            PanelToggle::Open => keymap::with_key("Hide right panel", ActionId::ToggleRightPanel),
            PanelToggle::Closed { hides_sidebar: false } => {
                keymap::with_key("Show right panel", ActionId::ToggleRightPanel)
            }
            PanelToggle::Closed { hides_sidebar: true } | PanelToggle::Waiting => format!(
                "{} · hides the sidebar to make room",
                keymap::with_key("Show right panel", ActionId::ToggleRightPanel)
            ),
        };
        let panel = div()
            .id("panel-toggle")
            .occlude()
            .absolute()
            .top_0()
            .right(px(PANEL_TOGGLE_RIGHT))
            .h(px(style::TITLE_ROW_H))
            .flex()
            .items_center()
            .child(
                style::icon_button("toggle-panel", ui, IconName::PanelRight, tooltip)
                    .in_title_row()
                    // Wanted but hidden for lack of room: the dot says it comes back.
                    .marked(look == PanelToggle::Waiting)
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_notes(window, cx))),
            );
        [sidebar.into_any_element(), panel.into_any_element()]
    }

    /// The top of the sidebar: one row, the traffic lights and the collapse button floating over its
    /// start, the profile chip after them (while there are two profiles), and the + that starts a
    /// terminal at its end (a right-click offers the other shells). It goes with the sidebar.
    pub(super) fn render_sidebar_header(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        div()
            .id("sidebar-title-row")
            .h(px(style::TITLE_ROW_H))
            .flex_none()
            .flex()
            .items_center()
            // The lights and the toggle float over the row's start; the profile chip is
            // the first thing after them.
            .pl(px(collapse_x(lights_end(window)) + style::HIT_MIN + AFTER_TOGGLE))
            .pr(px(TITLE_ROW_PAD))
            .title_drag()
            .children(self.render_profile_chip(cx))
            .child(div().flex_1())
            .child(
                self.with_shell_menu(
                    "new-session-shells",
                    style::icon_button(
                        "new-session",
                        ui,
                        IconName::Plus,
                        keymap::with_key("New terminal", ActionId::NewTerminal),
                    )
                    .in_title_row()
                    .focus_ring(self.title_ring(TitleButton::NewTerminal, window, cx))
                    .on_click(cx.listener(|this, _, window, cx| this.new_session(window, cx))),
                    cx,
                ),
            )
            .into_any_element()
    }

    /// The terminal's title row's padding at its left and right end. Beside an open panel the
    /// breadcrumb starts a small gap from its hairline; at the window's edge they make room for the lights or
    /// the panel toggle. A column sliding in is over the terminal, not beside it, until it is in.
    pub(super) fn title_row_pads(&self, window: &Window) -> (f32, f32) {
        let sidebar = self.sidebar_shown() && self.in_flow(Edge::Sidebar);
        let panel = self.notes_visible() && self.in_flow(Edge::Panel);
        // The row is the same with terminals as tabs (their bar is under it): after the lights, and
        // the collapse button when the sidebar is only hidden.
        let lights = if self.tabs_on_top() {
            lights_end(window) + AFTER_COLLAPSE
        } else {
            collapse_slot(lights_end(window))
        };
        (if sidebar { BESIDE_COLUMN } else { lights }, if panel { BESIDE_PANEL } else { PANEL_TOGGLE_SLOT })
    }

    /// How wide the terminal's title row is inside its padding: what the breadcrumb has to fit in (worked out from the columns, so it is right in the frame that
    /// draws them).
    pub(super) fn title_row_width(&self, window: &Window) -> f32 {
        let (left, right) = self.title_row_pads(window);
        self.title_column() - left - right
    }

    /// How wide the terminal's column is: the window less the panels beside it.
    pub(super) fn title_column(&self) -> f32 {
        let columns = self.columns();
        let mut column = self.window_width;
        if self.sidebar_shown() && self.in_flow(Edge::Sidebar) {
            column -= columns.sidebar;
        }
        if self.notes_visible() && self.in_flow(Edge::Panel) {
            column -= columns.panel;
        }
        column.max(crate::terminal_view::MIN_WIDTH)
    }

    /// The right panel's title row: its icon tabs from the 16 pt inset, the Notes tab's stash
    /// button, and room for the panel toggle that floats over its right end.
    pub(super) fn render_panel_tab_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        // The rule under the row is the terminal title row's, at the same height (outside the
        // 38 pt row, as there), so the two read as one line across the window.
        let row = div()
            .h(px(style::TITLE_ROW_H))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_1()
            .pl(px(TITLE_ROW_PAD))
            .pr(px(PANEL_TOGGLE_SLOT))
            .title_drag()
            .child(self.render_panel_tabs(window, cx))
            // A tab of the table brings its own buttons.
            .children(self.table_tab_actions(window, cx))
            // The Notes tab's one action sits beside the panel toggle rather than
            // on a row of its own above the list.
            .when(self.right_tab == TabId::Notes && crate::window::features::notes::notes_on(cx), |d| {
                d.child(
                    div().id("panel-actions").occlude().flex_none().child(
                        style::icon_tab(
                            "stash-prompt",
                            ui,
                            IconName::Inbox,
                            keymap::with_key("Stash prompt into a new note", ActionId::StashPrompt),
                            false,
                            None,
                        )
                        .on_click(cx.listener(|this, _, window, cx| this.stash_prompt(window, cx))),
                    ),
                )
            });
        // No rule under it: the tab's list draws one while rows are scrolled up beneath it, as the
        // sidebar's does.
        div().flex_none().flex().flex_col().child(row).into_any_element()
    }

    /// The buttons the tab on show brings beside the tabs, if its feature is in the table.
    fn table_tab_actions(&self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (at, tab) = self.tab_entry(self.right_tab)?;
        let actions = tab.actions?;
        actions(self.feature_entity(at)?, &self.facts(cx), window, cx)
    }

    /// The panel's tabs: a tab list VoiceOver reads with its counts in words, one
    /// keyboard stop whose ← → switch tabs at once.
    fn render_panel_tabs(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let ui = ui_in(&self.profile, cx);
        let session = self.active_session();
        let open_steps = session.tasks.iter().filter(|t| !t.done).count();
        let notes = session
            .notes
            .iter()
            .filter(|n| !crate::window::features::notes::is_blank_note(n.state.read(cx)))
            .count();
        let tabs = self.panel_tab_list(cx);
        let facts = self.facts(cx);
        let ring = self.ring(Area::PanelTabs, window);
        let tab =
            |icon: IconName, label: &str, key: ActionId, count: Option<usize>, spoken: String, tab: TabId| {
                let active = self.right_tab == tab;
                style::icon_tab(tab_element_id(tab), ui, icon, keymap::with_key(label, key), active, count)
                    .relative()
                    .role(Role::Tab)
                    .aria_label(spoken)
                    .aria_selected(active)
                    .aria_position_in_set(tabs.iter().position(|&t| t == tab).map_or(1, |at| at + 1))
                    .aria_size_of_set(tabs.len())
                    // The active tab is where the keys are (automatic activation).
                    .when(active, |d| d.aria_active_descendant())
                    .when(active && ring, |d| {
                        d.child(style::focus_ring(ui, style::RingPlacement::Outside, ui.radius))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.show_tab(tab, cx)))
            };
        let bar = div()
            .id("panel-tabs")
            .role(Role::TabList)
            .aria_label("Right panel")
            // Clicks here are for the tabs, not for dragging the window.
            .occlude()
            .flex()
            .items_center()
            .gap_1()
            .children(tabs.iter().filter_map(|&id| match self.tab_entry(id) {
                // A feature in the table says what its tab shows and says. It comes last in the
                // panel's order when it is the Project, so the tabs before it never shift when it
                // comes and goes.
                Some((at, def)) => {
                    let entity = self.feature_entity(at)?;
                    let count = def.count.and_then(|count| count(entity, &facts, cx));
                    Some(tab(def.icon, def.label, def.action, count, (def.spoken)(entity, &facts, cx), id))
                }
                None => Some(match id {
                    TabId::Changes => return None,
                    TabId::History => tab(
                        IconName::GitCommitHorizontal,
                        "History",
                        ActionId::ShowHistory,
                        None,
                        spoken::panel_tab(PanelTab::History),
                        id,
                    ),
                    TabId::Notes => tab(
                        IconName::FileText,
                        "Notes",
                        ActionId::ShowNotes,
                        None,
                        spoken::panel_tab(PanelTab::Notes { open_steps, notes }),
                        id,
                    ),
                    TabId::Usage => tab(
                        IconName::Gauge,
                        "Usage",
                        ActionId::ShowUsage,
                        None,
                        spoken::panel_tab(PanelTab::Usage),
                        id,
                    ),
                    // Whenever the terminal has a folder: its colour is set there, and its project made.
                    TabId::Project => tab(
                        IconName::Folder,
                        "Project",
                        ActionId::ShowProject,
                        None,
                        spoken::panel_tab(PanelTab::Project),
                        id,
                    ),
                }),
            }));
        self.area_container(Area::PanelTabs, bar, cx).into_any_element()
    }
}

/// A tab's element id, which the tests and devctl find it by.
const fn tab_element_id(tab: TabId) -> &'static str {
    match tab {
        TabId::Changes => "tab-changes",
        TabId::History => "tab-history",
        TabId::Notes => "tab-notes",
        TabId::Usage => "tab-usage",
        TabId::Project => "tab-project",
    }
}

/// The terminal title row's padding while `edge`'s column is `offset` points in, from its padding
/// at rest (`title_row_pads`, with the column out of the layout). The row's content stays clear of
/// the button floating at that side until the column's edge reaches it, then rides the edge; its
/// other end stays where it is on screen though the sidebar carries the terminal. Each end meets
/// the padding beside the column when it lands, so nothing jumps.
pub(super) fn sliding_title_pads(edge: Edge, (left, right): (f32, f32), offset: f32) -> (f32, f32) {
    match edge {
        Edge::Sidebar => ((left - offset).max(BESIDE_COLUMN), right + offset),
        Edge::Panel => (left, right.max(offset + BESIDE_PANEL)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ASSUMED_LIGHTS_SPAN, BESIDE_COLUMN, BESIDE_PANEL, LIGHTS_X, PANEL_TOGGLE_SLOT, collapse_slot,
        collapse_x, sliding_title_pads,
    };
    use crate::style;
    use wormhole_core::view::layout::Edge;

    /// With the assumed lights the collapse button's glyph is at x 82–98 and
    /// the title row of a hidden sidebar's window starts after the button.
    #[test]
    fn the_collapse_button_sits_after_the_assumed_lights() {
        let lights_end = LIGHTS_X + ASSUMED_LIGHTS_SPAN;
        let glyph = collapse_x(lights_end) + (style::HIT_MIN - style::TITLE_GLYPH) / 2.0;
        assert_eq!((glyph, glyph + style::TITLE_GLYPH), (82.0, 98.0));
        assert_eq!(collapse_slot(lights_end), 76.0 + style::HIT_MIN + 8.0);
        // Narrower lights leave it where it is.
        assert_eq!(collapse_x(lights_end - 6.0), 76.0);
    }

    /// Lights wider than assumed push the button (and the title row after it) right, so it
    /// keeps its 8 pt gap and never covers the zoom button.
    #[test]
    fn the_collapse_button_clears_wider_lights() {
        let lights_end = LIGHTS_X + 70.0;
        assert_eq!(collapse_x(lights_end), lights_end + 12.0);
        assert_eq!(collapse_slot(lights_end), lights_end + 12.0 + style::HIT_MIN + 8.0);
    }

    /// While a column slides, the title row's ends start at its padding with the column out
    /// and land on its padding beside the column, so the breadcrumb and buttons never jump.
    #[test]
    fn the_title_row_follows_a_sliding_column_to_where_it_lands() {
        let rest = (collapse_slot(LIGHTS_X + ASSUMED_LIGHTS_SPAN), PANEL_TOGGLE_SLOT);
        let sidebar = 240.0;
        assert_eq!(sliding_title_pads(Edge::Sidebar, rest, 0.0), rest);
        // Landed: the sidebar carried the terminal its width right; the left end sits beside it,
        // and the right end is where it was on screen.
        assert_eq!(sliding_title_pads(Edge::Sidebar, rest, sidebar), (BESIDE_COLUMN, rest.1 + sidebar));
        // Until the edge reaches the content, the content stays put after the collapse button.
        assert_eq!(sliding_title_pads(Edge::Sidebar, rest, 20.0).0 + 20.0, rest.0);
        let panel = 320.0;
        assert_eq!(sliding_title_pads(Edge::Panel, rest, 0.0), rest);
        assert_eq!(sliding_title_pads(Edge::Panel, rest, panel), (rest.0, panel + BESIDE_PANEL));
    }
}
