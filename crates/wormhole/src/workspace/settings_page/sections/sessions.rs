//! Terminals & sidebar: the live preview, the density, the ⌘ hints, and what each row shows.

use wormhole_core::config::StatusPosition;
use wormhole_core::sidebar::{SidebarNumbers, TerminalsIn};

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
