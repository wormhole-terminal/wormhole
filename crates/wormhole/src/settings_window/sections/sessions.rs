//! Terminals & sidebar: the live preview, the density, the ⌘ hints, and what each row shows.

use gpui_kit::*;
use wormhole_core::config::{ChangesScope, PlainChanges, SharedChanges};
use wormhole_core::view::sidebar::{Density, SidebarNumbers, TerminalsIn};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::preview;
use crate::settings::settings;
use crate::settings::update_settings;
use crate::settings_window::registry::{Segment, Setting, toggle};
use crate::settings_window::{SettingsWindow, SidebarLook, ui};
use crate::style;

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("sidebar-preview", preview::draw_sidebar_preview),
    Setting::segmented(
        "terminals-in",
        &[Segment::text(SETTINGS_SESSIONS_SIDEBAR), Segment::text(SETTINGS_SESSIONS_TABS)],
        |s| usize::from(s.terminals_in == TerminalsIn::Tabs),
        |s, ix| s.terminals_in = if ix == 0 { TerminalsIn::Sidebar } else { TerminalsIn::Tabs },
    ),
    Setting::custom("density", draw_density),
    toggle!("cards", show_groups_as_cards),
    toggle!("fade-group-colors", fade_group_colors),
    toggle!("show-group-folders", show_group_folders),
    Setting::segmented(
        "sidebar-numbers",
        &[
            Segment::text(SETTINGS_SESSIONS_STAY_WITH_THE_TERMINAL),
            Segment::text(SETTINGS_SESSIONS_FOLLOW_THE_LIST),
        ],
        |s| usize::from(s.sidebar_numbers == SidebarNumbers::List),
        |s, ix| s.sidebar_numbers = if ix == 0 { SidebarNumbers::Terminal } else { SidebarNumbers::List },
    ),
    toggle!("cmd-hints", sidebar_cmd_hints),
    toggle!("sidebar-changes", sidebar_changes),
    Setting::segmented(
        "sidebar-changes-scope",
        &[
            Segment::text(SETTINGS_SESSIONS_CHANGES_THIS_TERMINAL),
            Segment::text(SETTINGS_SESSIONS_CHANGES_REPOSITORY),
        ],
        |s| usize::from(s.sidebar_changes_scope == ChangesScope::Repository),
        |s, ix| {
            s.sidebar_changes_scope = if ix == 0 { ChangesScope::Terminal } else { ChangesScope::Repository }
        },
    ),
    Setting::segmented(
        "sidebar-changes-plain",
        &[
            Segment::text(SETTINGS_SESSIONS_CHANGES_PLAIN_FINISHING),
            Segment::text(SETTINGS_SESSIONS_CHANGES_PLAIN_ANY),
            Segment::text(SETTINGS_SESSIONS_CHANGES_PLAIN_NONE),
        ],
        |s| match s.sidebar_changes_plain {
            PlainChanges::Finishing => 0,
            PlainChanges::Any => 1,
            PlainChanges::None => 2,
        },
        |s, ix| {
            s.sidebar_changes_plain = match ix {
                0 => PlainChanges::Finishing,
                1 => PlainChanges::Any,
                _ => PlainChanges::None,
            }
        },
    ),
    Setting::segmented(
        "sidebar-changes-shared",
        &[
            Segment::text(SETTINGS_SESSIONS_CHANGES_SHARED_EACH),
            Segment::text(SETTINGS_SESSIONS_CHANGES_SHARED_NONE),
        ],
        |s| usize::from(s.sidebar_changes_shared == SharedChanges::None),
        |s, ix| s.sidebar_changes_shared = if ix == 0 { SharedChanges::Each } else { SharedChanges::None },
    ),
    toggle!("sidebar-changes-remember", sidebar_changes_remember),
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
