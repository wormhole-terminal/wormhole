//! Terminals & sidebar: the live preview, the density, the ⌘ hints, and what each row shows.

use gpui_kit::*;
use wormhole_core::sidebar::{Density, SidebarNumbers, TerminalsIn};

use super::preview;
use crate::settings::settings;
use crate::style;
use crate::workspace::settings_page::registry::{Segment, Setting, toggle};
use crate::workspace::settings_page::{SettingsWindow, SidebarLook, ui, update_settings};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("sidebar-preview", preview::draw_sidebar_preview),
    Setting::segmented(
        "terminals-in",
        &[Segment::text("Sidebar"), Segment::text("Tabs")],
        |s| usize::from(s.terminals_in == TerminalsIn::Tabs),
        |s, ix| s.terminals_in = if ix == 0 { TerminalsIn::Sidebar } else { TerminalsIn::Tabs },
    ),
    Setting::custom("density", draw_density),
    toggle!("cards", show_groups_as_cards),
    toggle!("fade-group-colors", fade_group_colors),
    toggle!("show-group-folders", show_group_folders),
    Setting::segmented(
        "sidebar-numbers",
        &[Segment::text("Stay with the terminal"), Segment::text("Follow the list")],
        |s| usize::from(s.sidebar_numbers == SidebarNumbers::List),
        |s, ix| s.sidebar_numbers = if ix == 0 { SidebarNumbers::Terminal } else { SidebarNumbers::List },
    ),
    toggle!("cmd-hints", sidebar_cmd_hints),
    toggle!("sidebar-changes", sidebar_changes),
    toggle!("sidebar-dot", sidebar_status_dot),
    toggle!("sidebar-notes", sidebar_notes),
];

/// Density: hovering an option shows the list in it beside the current one.
fn draw_density(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let current = settings(cx).density;
    style::segmented_form(ui)
        .children(Density::ALL.map(|density| {
            style::form_segment(("density", density as usize), ui, None, density.label(), current == density)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let look = hovered.then_some(SidebarLook::Density(density));
                    if this.sidebar_preview != look
                        && (*hovered || this.sidebar_preview == Some(SidebarLook::Density(density)))
                    {
                        this.sidebar_preview = look;
                        cx.notify();
                    }
                }))
                .on_click(move |_, _, cx| update_settings(cx, |s| s.density = density))
        }))
        .into_any_element()
}
