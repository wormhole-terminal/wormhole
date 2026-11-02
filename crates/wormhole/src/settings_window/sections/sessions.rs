//! Terminals & sidebar: the live preview, the density, the ⌘ hints, and what each row shows.

use std::rc::Rc;

use gpui_kit::*;
use wormhole_core::config::{ChangesScope, PlainChanges, SharedChanges};
use wormhole_core::view::sidebar::{Density, SidebarNumbers, TerminalsIn};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use crate::select::{self, MenuLine, MenuOptions};
use crate::settings::update_settings;
use crate::settings::{Settings, settings};
use crate::settings_window::registry::row_label;
use crate::settings_window::registry::{MenuOption, Setting, toggle};
use crate::settings_window::{SettingsWindow, SidebarLook, ui};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("sidebar-preview", super::draw_sidebar_preview),
    Setting::menu(
        "terminals-in",
        &[MenuOption::text(SETTINGS_SESSIONS_SIDEBAR), MenuOption::text(SETTINGS_SESSIONS_TABS)],
        |s| usize::from(s.terminals_in == TerminalsIn::Tabs),
        |s, ix| s.terminals_in = if ix == 0 { TerminalsIn::Sidebar } else { TerminalsIn::Tabs },
    ),
    Setting::custom("density", draw_density).default_words(|| Settings::default().density.label().into()),
    toggle!("cards", show_groups_as_cards),
    toggle!("fade-group-colors", fade_group_colors),
    toggle!("show-group-folders", show_group_folders),
    Setting::menu(
        "sidebar-numbers",
        &[
            MenuOption::text(SETTINGS_SESSIONS_STAY_WITH_THE_TERMINAL),
            MenuOption::text(SETTINGS_SESSIONS_FOLLOW_THE_LIST),
        ],
        |s| usize::from(s.sidebar_numbers == SidebarNumbers::List),
        |s, ix| s.sidebar_numbers = if ix == 0 { SidebarNumbers::Terminal } else { SidebarNumbers::List },
    ),
    toggle!("cmd-hints", sidebar_cmd_hints),
    toggle!("sidebar-changes", sidebar_changes),
    Setting::menu(
        "sidebar-changes-scope",
        &[
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_THIS_TERMINAL),
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_REPOSITORY),
        ],
        |s| usize::from(s.sidebar_changes_scope == ChangesScope::Repository),
        |s, ix| {
            s.sidebar_changes_scope = if ix == 0 { ChangesScope::Terminal } else { ChangesScope::Repository }
        },
    ),
    Setting::menu(
        "sidebar-changes-plain",
        &[
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_PLAIN_FINISHING),
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_PLAIN_ANY),
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_PLAIN_NONE),
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
    Setting::menu(
        "sidebar-changes-shared",
        &[
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_SHARED_EACH),
            MenuOption::text(SETTINGS_SESSIONS_CHANGES_SHARED_NONE),
        ],
        |s| usize::from(s.sidebar_changes_shared == SharedChanges::None),
        |s, ix| s.sidebar_changes_shared = if ix == 0 { SharedChanges::Each } else { SharedChanges::None },
    ),
    toggle!("sidebar-changes-remember", sidebar_changes_remember),
    toggle!("sidebar-dot", sidebar_status_dot),
    toggle!("sidebar-notes", sidebar_notes),
];

/// Density: the option the pointer or the keyboard is on shows the list in it beside the current one.
fn draw_density(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let current = settings(cx).density;
    let window = cx.weak_entity();
    select::menu_select("density", ui, row_label("density"), current.label(), move |_| MenuOptions {
        items: Density::ALL.into_iter().map(|density| MenuLine::new(density.label())).collect(),
        current: Density::ALL.iter().position(|density| *density == current),
        on_pick: Rc::new(|ix, _, cx| {
            if let Some(density) = Density::ALL.get(ix) {
                update_settings(cx, |s| s.density = *density);
            }
        }),
        on_hover: Some(Rc::new(move |at, _, cx| {
            let look = at.and_then(|ix| Density::ALL.get(ix)).map(|density| SidebarLook::Density(*density));
            // Deferred: a menu can close from inside an update of this window (its closing hook runs
            // wherever the close happens), and updating it again there would panic.
            let window = window.clone();
            cx.defer(move |cx| {
                window
                    .update(cx, |this, cx| {
                        if this.sidebar_preview != look {
                            this.sidebar_preview = look;
                            cx.notify();
                        }
                    })
                    .ok();
            });
        })),
    })
    .into_any_element()
}
