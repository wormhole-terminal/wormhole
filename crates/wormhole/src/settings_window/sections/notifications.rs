//! Notifications: whether macOS lets Wormhole notify, what reaches you while you aren't
//! looking at a terminal (agent or plain shell), how it is delivered, and the Dock. The rules
//! behind each switch are core's `notify`; the permission comes from the notification center.
//!
//! `notifications` stays the master switch: it is on while any kind is on, so turning every kind
//! off turns it off.

use gpui_kit::component::button::ButtonVariants;
use gpui_kit::*;
use wormhole_core::config::{DockBadge, InFront, NotifySound};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use crate::notifications;
use crate::platform::notifications::Permission;
use crate::settings::Settings;
use crate::settings_window::registry::{Current, MenuOption, Setting, toggle};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, RowKeys, SettingParts, SettingState};

/// What "A command finishes" offers besides Off: how long a command runs before it counts, in seconds.
const AFTER: [u32; 3] = [10, 30, 60];

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("notifications-preview", |_, cx| super::draw_notifications_preview(cx)),
    Setting::block("notify-permission", draw_permission),
    Setting::toggle(
        "notify-needs-you",
        |s| kind_on(s, s.notify_needs_you),
        |s, on| set_kind(s, |s| s.notify_needs_you = on),
    )
    .describe(describe_needs_you)
    .chip(|_, cx| {
        notifications::time_sensitive(cx).then(|| text(SETTINGS_NOTIFICATIONS_TIME_SENSITIVE).into())
    }),
    Setting::toggle(
        "notify-agent-done",
        |s| kind_on(s, s.notify_agent_done),
        |s, on| set_kind(s, |s| s.notify_agent_done = on),
    ),
    // One menu for the switch and its threshold: Off turns the kind off and leaves the threshold where it
    // was, After turns it on with that threshold. A threshold written in config.json that the menu
    // doesn't list shows as it is and stays until another is chosen.
    Setting::menu_of_values(
        "notify-command-done",
        &[
            MenuOption::counted(after_label, AFTER[0]),
            MenuOption::counted(after_label, AFTER[1]),
            MenuOption::counted(after_label, AFTER[2]),
            MenuOption::text(SETTINGS_NOTIFICATIONS_OFF),
        ],
        |s| match (kind_on(s, s.notify_command_done), AFTER.iter().position(|&n| n == s.notify_command_after))
        {
            (false, _) => Current::At(AFTER.len()),
            (true, Some(at)) => Current::At(at),
            (true, None) => Current::Other(after_label(s.notify_command_after)),
        },
        |s, ix| match AFTER.get(ix) {
            Some(&secs) => {
                s.notify_command_after = secs;
                set_kind(s, |s| s.notify_command_done = true);
            }
            None => set_kind(s, |s| s.notify_command_done = false),
        },
    ),
    Setting::toggle(
        "notify-command-failed",
        |s| kind_on(s, s.notify_command_failed),
        |s, on| set_kind(s, |s| s.notify_command_failed = on),
    ),
    Setting::toggle(
        "notify-bell",
        |s| kind_on(s, s.notify_bell),
        |s, on| set_kind(s, |s| s.notify_bell = on),
    ),
    toggle!("notify-only-unseen", notify_only_unseen),
    Setting::menu(
        "notify-in-front",
        &[MenuOption::text(SETTINGS_NOTIFICATIONS_BANNER), MenuOption::text(SETTINGS_NOTIFICATIONS_NOTHING)],
        |s| match s.notify_in_front {
            InFront::Banner => 0,
            InFront::None => 1,
        },
        |s, ix| s.notify_in_front = [InFront::Banner, InFront::None][ix.min(1)],
    ),
    Setting::menu(
        "notify-sound",
        &[
            MenuOption::text(SETTINGS_NOTIFICATIONS_NEEDS_YOU_ONLY),
            MenuOption::text(SETTINGS_NOTIFICATIONS_ALL),
            MenuOption::text(SETTINGS_NOTIFICATIONS_NONE),
        ],
        |s| match s.notify_sound {
            NotifySound::NeedsYou => 0,
            NotifySound::All => 1,
            NotifySound::None => 2,
        },
        |s, ix| s.notify_sound = [NotifySound::NeedsYou, NotifySound::All, NotifySound::None][ix.min(2)],
    ),
    toggle!("notify-group-bursts", notify_group_bursts),
    Setting::menu(
        "dock-badge",
        &[
            MenuOption::text(SETTINGS_NOTIFICATIONS_OFF),
            MenuOption::text(SETTINGS_NOTIFICATIONS_NEEDS_YOU),
            MenuOption::text(SETTINGS_NOTIFICATIONS_NEEDS_YOU_FINISHED),
        ],
        |s| match s.dock_badge {
            DockBadge::Off => 0,
            DockBadge::NeedsYou => 1,
            DockBadge::NeedsYouDone => 2,
        },
        |s, ix| s.dock_badge = [DockBadge::Off, DockBadge::NeedsYou, DockBadge::NeedsYouDone][ix.min(2)],
    ),
    toggle!("dock-bounce", dock_bounce),
];

/// Change one kind's switch, keeping the master switch on while any kind is on (core's rule, which the
/// terminal welcome follows too).
fn set_kind(s: &mut Settings, change: impl FnOnce(&mut Settings)) {
    s.change_notify_kind(change);
}

/// A kind's switch, read through the master switch: with `notifications` off nothing is on.
fn kind_on(s: &Settings, on: bool) -> bool {
    s.notify_kind_on(on)
}

/// What the permission row says and offers, by state.
fn permission_text(permission: Permission) -> (&'static str, &'static str) {
    match permission {
        Permission::Checking => (text(SETTINGS_NOTIFICATIONS_ASKING_MACOS), ""),
        Permission::NotAsked => (
            text(SETTINGS_NOTIFICATIONS_NOT_ASKED_YET),
            text(SETTINGS_NOTIFICATIONS_MACOS_ASKS_ONCE_NOTHING_IS_SENT),
        ),
        Permission::Allowed { time_sensitive: true } => (
            text(SETTINGS_NOTIFICATIONS_ALLOWED_IN_SYSTEM_SETTINGS),
            text(SETTINGS_NOTIFICATIONS_BANNERS_SOUNDS_AND_TIME_SENSITIVE),
        ),
        Permission::Allowed { time_sensitive: false } => (
            text(SETTINGS_NOTIFICATIONS_ALLOWED_IN_SYSTEM_SETTINGS),
            text(SETTINGS_NOTIFICATIONS_BANNERS_AND_SOUNDS),
        ),
        Permission::Denied => {
            (text(SETTINGS_NOTIFICATIONS_OFF_IN_SYSTEM_SETTINGS), text(SETTINGS_NOTIFICATIONS_OFF_HINT))
        }
        Permission::OutsideBundle => (
            text(SETTINGS_NOTIFICATIONS_OUTSIDE_THE_APP_BUNDLE),
            text(SETTINGS_NOTIFICATIONS_OUTSIDE_BUNDLE_HINT),
        ),
    }
}

/// The permission's row: what macOS allows, in the row's own words, and what to do about it.
fn draw_permission(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let permission = notifications::permission(cx);
    let (title, hint) = permission_text(permission);
    let send_test = || {
        style::form_button("notify-send-test")
            .outline()
            .label(text(SETTINGS_NOTIFICATIONS_SEND_TEST))
            .on_click(|_, _, cx| notifications::send_test(cx))
    };
    let system_settings = || {
        style::form_button("notify-open-settings")
            .outline()
            .label(text(SETTINGS_NOTIFICATIONS_OPEN_SYSTEM_SETTINGS))
            .on_click(|_, _, cx| notifications::open_system_settings(cx))
    };
    let buttons: Vec<AnyElement> = match permission {
        Permission::Allowed { .. } => {
            vec![send_test().into_any_element(), system_settings().into_any_element()]
        }
        Permission::NotAsked => vec![
            style::form_button("notify-allow")
                .primary()
                .label(text(SETTINGS_NOTIFICATIONS_ALLOW_NOTIFICATIONS))
                .on_click(|_, _, cx| notifications::ask_permission(cx))
                .into_any_element(),
        ],
        Permission::Denied => vec![system_settings().into_any_element()],
        Permission::OutsideBundle => vec![send_test().into_any_element()],
        Permission::Checking => Vec::new(),
    };
    style::setting(
        "setting-notify-permission",
        ui,
        SettingParts {
            label: title.into(),
            description: (!hint.is_empty()).then(|| hint.into()),
            visible_hint: true,
            chip: None,
            keys: RowKeys::NONE,
            state: SettingState::Default,
            found: None,
            found_description: None,
            control: div().flex().gap_2().children(buttons).into_any_element(),
            tall: false,
            extra: None,
            on_reset: None,
        },
        cx,
    )
    .into_any_element()
}

/// "A terminal needs input", and when this build can't deliver it as Time Sensitive, says so
/// (the entitlement is only in Developer ID builds with the profile).
fn describe_needs_you(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let entitled = notifications::time_sensitive(cx);
    Some(if entitled {
        text(SETTINGS_NOTIFICATIONS_NEEDS_YOU_HINT).into()
    } else {
        text(SETTINGS_NOTIFICATIONS_NEEDS_YOU_HINT_PLAIN).into()
    })
}

/// "After 10 s", "After 1 min".
fn after_label(secs: u32) -> String {
    if secs >= 60 {
        settings_notifications_after_minutes(&(secs / 60).to_string())
    } else {
        settings_notifications_after_seconds(&secs.to_string())
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::config::Settings;

    use super::{AFTER, SETTINGS, set_kind};
    use crate::settings_window::registry::{Control, Current, Menu};

    fn menu() -> &'static Menu {
        SETTINGS
            .iter()
            .find_map(|s| match &s.control {
                Control::Menu(menu) if s.id == "notify-command-done" => Some(menu),
                _ => None,
            })
            .expect("the command-finished menu")
    }

    fn shown(s: &Settings) -> String {
        match menu().current(s) {
            Current::At(at) => format!("option {at}"),
            Current::Other(words) => words,
        }
    }

    #[test]
    fn the_menu_is_off_or_after_a_threshold_and_keeps_the_other_half() {
        let mut s = Settings::default();
        set_kind(&mut s, |s| s.notify_command_done = true);
        s.notify_command_after = 30;
        assert_eq!(shown(&s), "option 1");
        (menu().set)(&mut s, AFTER.len());
        assert!(!s.notify_command_done, "Off turns the kind off");
        assert_eq!(s.notify_command_after, 30, "and leaves the threshold where it was");
        assert_eq!(shown(&s), format!("option {}", AFTER.len()));
        (menu().set)(&mut s, 2);
        assert!(s.notify_command_done && s.notify_command_after == 60, "After turns it on at that time");
    }

    #[test]
    fn a_threshold_written_by_hand_shows_as_it_is_and_stays_until_another_is_chosen() {
        let mut s = Settings::default();
        set_kind(&mut s, |s| s.notify_command_done = true);
        s.notify_command_after = 45;
        assert_eq!(shown(&s), "After 45 s");
        s.notify_command_after = 300;
        assert_eq!(shown(&s), "After 5 min", "a step the menu no longer lists");
        assert_eq!(s.notify_command_after, 300, "reading never writes");
        (menu().set)(&mut s, 0);
        assert_eq!(s.notify_command_after, AFTER[0]);
    }
}
