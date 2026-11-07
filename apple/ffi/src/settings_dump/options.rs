//! The options of the settings' choices: a stored value and the words the interface shows for it.
//! Values come from the real enums (`serde`'s spelling is what `config.json` holds), words from the
//! catalogs, so neither can drift from the app's.

use serde::Serialize;
use serde_json::{Value, json};
use wormhole_core::config::clipboard::{ClipboardRead, ClipboardWrite};
use wormhole_core::config::{
    BranchColor, ChangesScope, DockBadge, HotkeyAction, InFront, MenuBarIcon, NotifySound, OptionAsMeta,
    PlainChanges, SharedChanges, SizeUnits, TextSize, ThemeMode,
};
use wormhole_core::update::UpdateChannel;
use wormhole_core::view::motion::{ColumnMotion, FoldMotion, InsertMotion, MotionChoice, StatusMotion};
use wormhole_core::view::sidebar::{Density, SidebarNumbers, TerminalsIn};
#[allow(clippy::wildcard_imports, reason = "the keys are generated, one constant per message")]
use wormhole_text::keys::*;
use wormhole_text::{Key, text};

/// One option: `value` as the file stores it, shown as `label`, with an optional SF Symbol.
pub(super) fn option(value: &impl Serialize, label: Key, icon: Option<&str>) -> Value {
    let mut option = json!({ "value": value, "label": text(label) });
    if let Some(icon) = icon {
        option["icon"] = json!(icon);
    }
    option
}

fn options<T: Serialize>(pairs: &[(T, Key)]) -> Vec<Value> {
    pairs.iter().map(|(value, label)| option(value, *label, None)).collect()
}

/// The options of the choice stored under `key`, if it is a fixed list.
pub(super) fn of(key: &str) -> Option<Vec<Value>> {
    Some(match key {
        "theme_mode" => vec![
            option(&ThemeMode::Dark, SETTINGS_APPEARANCE_DARK, Some("moon")),
            option(&ThemeMode::Light, SETTINGS_APPEARANCE_LIGHT, Some("sun.max")),
            option(&ThemeMode::System, SETTINGS_APPEARANCE_MATCH_SYSTEM, Some("display")),
        ],
        "branch_color" => options(&[
            (BranchColor::Theme, SETTINGS_APPEARANCE_BRANCH_THEME),
            (BranchColor::Primary, SETTINGS_APPEARANCE_BRANCH_PRIMARY),
            (BranchColor::Custom, SETTINGS_APPEARANCE_BRANCH_CUSTOM),
        ]),
        "settings_text_size" => options(&[
            (TextSize::Small, SETTINGS_FONTS_TEXT_SIZE_SMALL),
            (TextSize::Default, SETTINGS_FONTS_TEXT_SIZE_DEFAULT),
            (TextSize::Large, SETTINGS_FONTS_TEXT_SIZE_LARGE),
            (TextSize::Larger, SETTINGS_FONTS_TEXT_SIZE_LARGER),
        ]),
        "option_as_meta" => options(&[
            (OptionAsMeta::Off, SETTINGS_TERMINAL_OFF),
            (OptionAsMeta::Left, SETTINGS_TERMINAL_LEFT),
            (OptionAsMeta::Right, SETTINGS_TERMINAL_RIGHT),
            (OptionAsMeta::Both, SETTINGS_TERMINAL_BOTH),
        ]),
        "clipboard_read" => options(&[
            (ClipboardRead::Ask, SETTINGS_TERMINAL_ASK),
            (ClipboardRead::Allow, SETTINGS_TERMINAL_ALLOW),
            (ClipboardRead::Deny, SETTINGS_TERMINAL_DENY),
        ]),
        "clipboard_write" => options(&[
            (ClipboardWrite::Allow, SETTINGS_TERMINAL_ALLOW),
            (ClipboardWrite::Deny, SETTINGS_TERMINAL_DENY),
        ]),
        "scrollback_lines" => [1_000_u32, 5_000, 10_000, 50_000, 100_000, 200_000]
            .iter()
            .map(|n| json!({ "value": n, "label": settings_terminal_scrollback_lines(&grouped(*n)) }))
            .collect(),
        "terminals_in" => options(&[
            (TerminalsIn::Sidebar, SETTINGS_SESSIONS_SIDEBAR),
            (TerminalsIn::Tabs, SETTINGS_SESSIONS_TABS),
        ]),
        "density" => Density::ALL.iter().map(|d| json!({ "value": d, "label": d.label() })).collect(),
        "sidebar_numbers" => options(&[
            (SidebarNumbers::Terminal, SETTINGS_SESSIONS_STAY_WITH_THE_TERMINAL),
            (SidebarNumbers::List, SETTINGS_SESSIONS_FOLLOW_THE_LIST),
        ]),
        "sidebar_changes_scope" => options(&[
            (ChangesScope::Terminal, SETTINGS_SESSIONS_CHANGES_THIS_TERMINAL),
            (ChangesScope::Repository, SETTINGS_SESSIONS_CHANGES_REPOSITORY),
        ]),
        "sidebar_changes_plain" => options(&[
            (PlainChanges::Finishing, SETTINGS_SESSIONS_CHANGES_PLAIN_FINISHING),
            (PlainChanges::Any, SETTINGS_SESSIONS_CHANGES_PLAIN_ANY),
            (PlainChanges::None, SETTINGS_SESSIONS_CHANGES_PLAIN_NONE),
        ]),
        "sidebar_changes_shared" => options(&[
            (SharedChanges::Each, SETTINGS_SESSIONS_CHANGES_SHARED_EACH),
            (SharedChanges::None, SETTINGS_SESSIONS_CHANGES_SHARED_NONE),
        ]),
        "changes_tree" => {
            vec![option(&false, SETTINGS_GIT_LIST, None), option(&true, SETTINGS_GIT_TREE, None)]
        }
        "diff_split" => vec![
            option(&true, SETTINGS_GIT_SPLIT, Some("rectangle.split.2x1")),
            option(&false, SETTINGS_GIT_UNIFIED, Some("rectangle.split.1x2")),
        ],
        "size_units" => {
            options(&[(SizeUnits::Decimal, SETTINGS_GIT_MB), (SizeUnits::Binary, SETTINGS_GIT_MIB)])
        }
        "notify_in_front" => options(&[
            (InFront::Banner, SETTINGS_NOTIFICATIONS_BANNER),
            (InFront::None, SETTINGS_NOTIFICATIONS_NOTHING),
        ]),
        "notify_sound" => options(&[
            (NotifySound::NeedsYou, SETTINGS_NOTIFICATIONS_NEEDS_YOU_ONLY),
            (NotifySound::All, SETTINGS_NOTIFICATIONS_ALL),
            (NotifySound::None, SETTINGS_NOTIFICATIONS_NONE),
        ]),
        "dock_badge" => options(&[
            (DockBadge::Off, SETTINGS_NOTIFICATIONS_OFF),
            (DockBadge::NeedsYou, SETTINGS_NOTIFICATIONS_NEEDS_YOU),
            (DockBadge::NeedsYouDone, SETTINGS_NOTIFICATIONS_NEEDS_YOU_FINISHED),
        ]),
        "menu_bar_icon" => options(&[
            (MenuBarIcon::StatusCount, SETTINGS_MENU_BAR_STATUS_AND_COUNT),
            (MenuBarIcon::Status, SETTINGS_MENU_BAR_STATUS),
            (MenuBarIcon::Plain, SETTINGS_MENU_BAR_ICON_ONLY),
        ]),
        "hotkey_action" => options(&[
            (HotkeyAction::QuickPanel, SETTINGS_HOTKEY_QUICK_PANEL),
            (HotkeyAction::ToggleApp, SETTINGS_HOTKEY_SHOW_OR_HIDE_WORMHOLE),
        ]),
        "update_channel" => options(&[
            (UpdateChannel::Stable, SETTINGS_UPDATES_STABLE),
            (UpdateChannel::Unstable, SETTINGS_UPDATES_UNSTABLE),
        ]),
        "motion.insert" => motion::<InsertMotion>(&[
            SETTINGS_MOTION_CLASSIC,
            SETTINGS_MOTION_INSERT_REVEAL,
            SETTINGS_MOTION_INSERT_FROM_SIDE,
            SETTINGS_MOTION_INSERT_SOFT_SPRING,
        ]),
        "motion.fold" => motion::<FoldMotion>(&[
            SETTINGS_MOTION_CLASSIC,
            SETTINGS_MOTION_FOLD_DRAWER,
            SETTINGS_MOTION_FOLD_TUCK,
        ]),
        "motion.status" => motion::<StatusMotion>(&[
            SETTINGS_MOTION_CLASSIC,
            SETTINGS_MOTION_STATUS_CROSSFADE,
            SETTINGS_MOTION_STATUS_BADGE_POP,
            SETTINGS_MOTION_STATUS_WASH,
            SETTINGS_MOTION_STATUS_FLARE,
            SETTINGS_MOTION_STATUS_NUDGE,
            SETTINGS_MOTION_STATUS_LINE_ROLL,
        ]),
        "motion.column" => motion::<ColumnMotion>(&[
            SETTINGS_MOTION_CLASSIC,
            SETTINGS_MOTION_COLUMN_SOFT_SPRING,
            SETTINGS_MOTION_COLUMN_CONTENT_AFTER,
        ]),
        _ => return None,
    })
}

/// "10,000" for the scrollback options, as the app groups its digits.
fn grouped(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (ix, c) in digits.chars().enumerate() {
        if ix > 0 && (digits.len() - ix).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A motion choice's options in the order of `MotionChoice::ALL`, named as the file spells them.
fn motion<T: MotionChoice>(labels: &[Key]) -> Vec<Value> {
    T::ALL.iter().zip(labels).map(|(choice, label)| option(&choice.name(), *label, None)).collect()
}

/// What a motion choice is while the file says nothing.
pub(super) fn motion_default(key: &str) -> Option<&'static str> {
    Some(match key {
        "motion.insert" => InsertMotion::default().name(),
        "motion.fold" => FoldMotion::default().name(),
        "motion.status" => StatusMotion::default().name(),
        "motion.column" => ColumnMotion::default().name(),
        _ => return None,
    })
}
