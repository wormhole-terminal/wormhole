//! Terminals & sidebar: the live preview, the funnel's options (grouping, density) and
//! the ⌘ hints, and what each row shows.

use wormhole_core::config::StatusPosition;
use wormhole_core::sidebar::{Grouping, SidebarNumbers, SidebarOrder, TabLayout, TerminalsIn};

use super::*;

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("sidebar-preview", preview::draw_sidebar_preview),
    Setting::segmented(
        "terminals-in",
        &[Segment::text("Sidebar"), Segment::text("Tabs")],
        |s| usize::from(s.terminals_in == TerminalsIn::Tabs),
        |s, ix| s.terminals_in = if ix == 0 { TerminalsIn::Sidebar } else { TerminalsIn::Tabs },
    ),
    Setting::segmented(
        "tab-layout",
        &[Segment::text("Two rows"), Segment::text("Stacks"), Segment::text("Bottom")],
        |s| TabLayout::ALL.iter().position(|&layout| layout == s.tab_layout).unwrap_or(0),
        |s, ix| s.tab_layout = TabLayout::ALL.get(ix).copied().unwrap_or_default(),
    ),
    Setting::custom("sidebar-group", draw_grouping),
    Setting::segmented(
        "sidebar-order",
        &[Segment::text("Stable"), Segment::text("Classic")],
        |s| usize::from(s.sidebar_order == SidebarOrder::Classic),
        |s, ix| s.sidebar_order = if ix == 0 { SidebarOrder::Stable } else { SidebarOrder::Classic },
    ),
    Setting::segmented(
        "sidebar-numbers",
        &[Segment::text("Stay with the terminal"), Segment::text("Follow the list")],
        |s| usize::from(s.sidebar_numbers == SidebarNumbers::List),
        |s, ix| s.sidebar_numbers = if ix == 0 { SidebarNumbers::Terminal } else { SidebarNumbers::List },
    ),
    Setting::custom("compact", draw_density),
    toggle!("cmd-hints", sidebar_cmd_hints),
    toggle!("sidebar-branch", sidebar_branch),
    toggle!("sidebar-changes", sidebar_changes),
    toggle!("sidebar-dot", sidebar_status_dot),
    Setting::segmented(
        "status-position",
        &[Segment::text("Lead column"), Segment::text("Right edge")],
        |s| usize::from(s.sidebar_status_position == StatusPosition::Right),
        |s, ix| {
            s.sidebar_status_position = if ix == 0 { StatusPosition::Left } else { StatusPosition::Right }
        },
    ),
    toggle!("sidebar-agent", sidebar_agent_icon),
    toggle!("sidebar-notes", sidebar_notes),
];

/// Group by, the same as the funnel's: it sets the grouping of every open window, and
/// of the windows still to come.
fn draw_grouping(this: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    // What the funnel of the window in front shows; the setting for a window that has none.
    let current = this.front.grouping.unwrap_or(settings(cx).sidebar_group);
    style::segmented_form(ui)
        .children(
            [
                (0usize, "Folder", Grouping::Folder),
                (1, "Groups", Grouping::Custom),
                (2, "None", Grouping::None),
            ]
            .map(|(ix, label, grouping)| {
                style::form_segment(("sidebar-group", ix), ui, None, label, current == grouping).on_click(
                    cx.listener(move |this, _, _, cx| {
                        set_grouping(grouping, cx);
                        this.front.grouping = Some(grouping);
                        cx.notify();
                    }),
                )
            }),
        )
        .into_any_element()
}

/// The grouping for new windows, and for every open one now.
fn set_grouping(grouping: Grouping, cx: &mut App) {
    update_settings(cx, |s| s.sidebar_group = grouping);
    for window in app(cx).windows.clone() {
        AppState::with_workspace(&window, cx, |ws, _, cx| ws.set_grouping(grouping, cx));
    }
}

/// Density: hovering an option shows the list in it beside the current one.
fn draw_density(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let compact = settings(cx).compact_sidebar;
    style::segmented_form(ui)
        .children([(0usize, "Comfortable", false), (1, "Compact", true)].map(|(ix, label, value)| {
            style::form_segment(("density", ix), ui, None, label, compact == value)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let look = hovered.then_some(SidebarLook::Compact(value));
                    if this.sidebar_preview != look
                        && (*hovered || this.sidebar_preview == Some(SidebarLook::Compact(value)))
                    {
                        this.sidebar_preview = look;
                        cx.notify();
                    }
                }))
                .on_click(move |_, _, cx| update_settings(cx, |s| s.compact_sidebar = value))
        }))
        .into_any_element()
}
