//! The controls of the notifications page the native window writes through: each setting's menu or switch over the settings.

use crate::settings::Settings;
use wormhole_core::config::{DockBadge, InFront, NotifySound};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::{Current, MenuOption, Setting, toggle};

/// A kind's switch, read through the master switch: with `notifications` off nothing is on.
fn kind_on(s: &Settings, on: bool) -> bool {
    s.notify_kind_on(on)
}

/// Change one kind's switch, keeping the master switch on while any kind is on (core's rule, which the
/// terminal welcome follows too).
fn set_kind(s: &mut Settings, change: impl FnOnce(&mut Settings)) {
    s.change_notify_kind(change);
}

/// What "A command finishes" offers besides Off: how long a command runs before it counts, in seconds.
const AFTER: [u32; 3] = [10, 30, 60];

/// "After 10 s", "After 1 min".
fn after_label(secs: u32) -> String {
    if secs >= 60 {
        settings_notifications_after_minutes(&(secs / 60).to_string())
    } else {
        settings_notifications_after_seconds(&secs.to_string())
    }
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting::toggle(
        "notify-needs-you",
        |s| kind_on(s, s.notify_needs_you),
        |s, on| set_kind(s, |s| s.notify_needs_you = on),
    ),
    Setting::toggle(
        "notify-agent-done",
        |s| kind_on(s, s.notify_agent_done),
        |s, on| set_kind(s, |s| s.notify_agent_done = on),
    ),
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

#[cfg(test)]
mod tests {
    use wormhole_core::config::Settings;

    use super::{AFTER, SETTINGS, set_kind};
    use crate::settings_window::native::controls::{Control, Current, Menu};

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
