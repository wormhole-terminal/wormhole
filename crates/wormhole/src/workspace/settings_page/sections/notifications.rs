//! Notifications: whether macOS lets Wormhole notify, what reaches you while you aren't
//! looking at a terminal (agent or plain shell), how it is delivered, and the Dock. The rules
//! behind each switch are core's `notify`; the permission comes from the notification center.
//!
//! `notifications` stays the master switch: it is on while any kind is on, so turning every kind
//! off turns it off.

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::*;
use wormhole_core::config::{DockBadge, InFront, NotifySound};

use crate::platform::notifications::Permission;
use crate::settings::update_settings;
use crate::settings::{Settings, settings};
use crate::style::{self, SettingParts, SettingState};
use crate::workspace::notifications;
use crate::workspace::settings_page::registry::{Segment, Setting, toggle};
use crate::workspace::settings_page::{SettingsWindow, ui};

/// The command-finished threshold's steps, in seconds.
const AFTER: [u32; 5] = [5, 10, 30, 60, 300];

pub(super) const SETTINGS: &[Setting] = &[
    Setting::block("notify-permission", draw_permission),
    Setting::toggle(
        "notify-needs-you",
        |s| kind_on(s, s.notify_needs_you),
        |s, on| set_kind(s, |s| s.notify_needs_you = on),
    )
    .describe(describe_needs_you)
    .chip(|_, cx| notifications::time_sensitive(cx).then(|| "Time Sensitive".into())),
    Setting::toggle(
        "notify-agent-done",
        |s| kind_on(s, s.notify_agent_done),
        |s, on| set_kind(s, |s| s.notify_agent_done = on),
    ),
    Setting::custom("notify-command-done", draw_command_done),
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
    Setting::segmented(
        "notify-in-front",
        &[Segment::text("Banner"), Segment::text("Nothing")],
        |s| match s.notify_in_front {
            InFront::Banner => 0,
            InFront::None => 1,
        },
        |s, ix| s.notify_in_front = [InFront::Banner, InFront::None][ix.min(1)],
    ),
    Setting::segmented(
        "notify-sound",
        &[Segment::text("Needs you only"), Segment::text("All"), Segment::text("None")],
        |s| match s.notify_sound {
            NotifySound::NeedsYou => 0,
            NotifySound::All => 1,
            NotifySound::None => 2,
        },
        |s, ix| s.notify_sound = [NotifySound::NeedsYou, NotifySound::All, NotifySound::None][ix.min(2)],
    ),
    toggle!("notify-group-bursts", notify_group_bursts),
    Setting::segmented(
        "dock-badge",
        &[Segment::text("Off"), Segment::text("Needs you"), Segment::text("Needs you + finished")],
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
        Permission::Checking => ("Asking macOS…", ""),
        Permission::NotAsked => ("Not asked yet", "macOS asks once; nothing is sent before"),
        Permission::Allowed { time_sensitive: true } => {
            ("Allowed in System Settings", "Banners, sounds and Time Sensitive")
        }
        Permission::Allowed { time_sensitive: false } => ("Allowed in System Settings", "Banners and sounds"),
        Permission::Denied => ("Off in System Settings", "Nothing is delivered until turned on there"),
        Permission::OutsideBundle => {
            ("Outside the app bundle", "cargo run: osascript, no actions or grouping")
        }
    }
}

/// The permission's row: what macOS allows, in the row's own words, and what to do about it.
fn draw_permission(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let permission = notifications::permission(cx);
    let (title, hint) = permission_text(permission);
    let send_test = || {
        Button::new("notify-send-test")
            .small()
            .outline()
            .label("Send test")
            .on_click(|_, _, cx| notifications::send_test(cx))
    };
    let system_settings = || {
        Button::new("notify-open-settings")
            .small()
            .outline()
            .label("Open System Settings")
            .on_click(|_, _, cx| notifications::open_system_settings(cx))
    };
    let buttons: Vec<AnyElement> = match permission {
        Permission::Allowed { .. } => {
            vec![send_test().into_any_element(), system_settings().into_any_element()]
        }
        Permission::NotAsked => vec![
            Button::new("notify-allow")
                .small()
                .primary()
                .label("Allow notifications")
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
            chip: None,
            key: "",
            state: SettingState::Default,
            found: None,
            found_description: None,
            control: div().flex().gap_2().children(buttons).into_any_element(),
            extra: None,
            on_reset: None,
        },
        cx,
    )
    .into_any_element()
}

/// "A terminal needs you", and when this build can't deliver it as Time Sensitive, says so
/// (the entitlement is only in Developer ID builds with the profile).
fn describe_needs_you(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> Option<SharedString> {
    let entitled = notifications::time_sensitive(cx);
    Some(if entitled {
        "An agent asks a question or for permission".into()
    } else {
        "An agent asks a question or for permission. Delivered as a normal notification in this build".into()
    })
}

/// The threshold's step: `AFTER` nearest the setting.
fn after_step(secs: u32) -> usize {
    AFTER.iter().position(|&step| step >= secs).unwrap_or(AFTER.len() - 1)
}

fn after_label(secs: u32) -> String {
    if secs >= 60 { format!("{} min", secs / 60) } else { format!("{secs} s") }
}

/// "A command finishes": the threshold (5 s … 5 min) beside the switch.
fn draw_command_done(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx);
    let on = kind_on(s, s.notify_command_done);
    let after = s.notify_command_after;
    let nudge = |delta: isize| {
        move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            let step = after_step(settings(cx).notify_command_after).saturating_add_signed(delta);
            let secs = AFTER[step.min(AFTER.len() - 1)];
            update_settings(cx, |s| s.notify_command_after = secs);
        }
    };
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(style::stepper(ui, "notify-command-after", after_label(after), nudge(-1), nudge(1)))
        .child(style::toggle("notify-command-done", on, |on, _, cx| {
            update_settings(cx, |s| set_kind(s, |s| s.notify_command_done = on))
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{AFTER, after_label, after_step};

    #[test]
    fn the_threshold_steps_through_its_list() {
        assert_eq!(after_step(10), 1);
        assert_eq!(after_step(12), 2, "a hand-written value rounds up to the next step");
        assert_eq!(after_step(999), AFTER.len() - 1);
        assert_eq!(
            (after_label(10), after_label(60), after_label(300)),
            ("10 s".into(), "1 min".into(), "5 min".into())
        );
    }
}
