//! Updates: the version, automatic updates, and where an update stands in every phase
//! (up to date, checking, available with its notes, downloading with a meter, installed,
//! development build, failed), its status and actions on one row.

use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::progress::Progress;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::update::{
    Release, Security, Severity, UpdateChannel, counted_user_agent, current_platform, macos_version,
    plain_user_agent,
};

use crate::settings::{UiColors, settings};
use crate::settings_window::registry::{Segment, Setting, toggle};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, TextRole};
use crate::update::{UpdatePhase, Updates};

pub(super) const SETTINGS: &[Setting] = &[
    Setting::custom("version", |_, cx| {
        let ui = ui(cx);
        div()
            .text_body()
            .tabular_nums()
            .text_color(ui.muted)
            .child(env!("CARGO_PKG_VERSION"))
            .into_any_element()
    }),
    toggle!("auto-update", auto_update),
    Setting::segmented(
        "update-channel",
        &[Segment::text(SETTINGS_UPDATES_STABLE), Segment::text(SETTINGS_UPDATES_UNSTABLE)],
        |s| usize::from(s.update_channel == UpdateChannel::Unstable),
        |s, ix| s.update_channel = if ix == 1 { UpdateChannel::Unstable } else { UpdateChannel::Stable },
    ),
    Setting::block("update-status", draw_status),
    toggle!("anonymous-count", anonymous_count),
    Setting::block("count-details", draw_count_details),
];

/// Privacy → what an update check sends: this Mac's own user agent word for word, the daily and
/// monthly flag, what is never sent, and in the first day how long until the first count.
fn draw_count_details(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    // The version and the Mac don't change while the app runs; the system's version is read once.
    static MAC: OnceLock<Option<(semver::Version, String, Option<semver::Version>)>> = OnceLock::new();
    let mac = MAC.get_or_init(|| {
        let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
        Some((version, current_platform(), macos_version()))
    });
    let ui = ui(cx);
    let (code, _) = crate::fonts::code_font(cx);
    let (counting, channel) = (settings(cx).anonymous_count, settings(cx).update_channel);
    let agent = mac.as_ref().map(|(version, platform, system)| match counting {
        true => counted_user_agent(version, channel, platform, system.as_ref()),
        false => plain_user_agent(version),
    });
    let starts = counting
        .then(|| cx.global::<Updates>().count_starts_in())
        .flatten()
        .map(|left| settings_updates_count_starts_in(whole(left.as_secs().div_ceil(3600))));
    let code_line = |line: SharedString| div().font_family(code.clone()).text_color(ui.text).child(line);
    let line = |label: &'static str, values: Vec<AnyElement>, why: Option<SharedString>| {
        div()
            .flex()
            .gap_4()
            .py_2()
            .rule_bottom(ui)
            .child(div().w(px(DETAIL_LABEL_W)).flex_none().text_body().text_color(ui.muted).child(label))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_body()
                    .children(values)
                    .children(why.map(|why| div().text_caption().text_color(ui.muted).child(why))),
            )
    };
    let every = line(
        text(SETTINGS_UPDATES_COUNT_EVERY_CHECK),
        agent.map(|agent| code_line(agent.into()).into_any_element()).into_iter().collect(),
        Some(
            text(if counting {
                SETTINGS_UPDATES_COUNT_EVERY_CHECK_COUNTED
            } else {
                SETTINGS_UPDATES_COUNT_EVERY_CHECK_PLAIN
            })
            .into(),
        ),
    );
    let first = counting.then(|| {
        line(
            text(SETTINGS_UPDATES_COUNT_FIRST_CHECK),
            COUNT_HEADERS.iter().map(|header| code_line((*header).into()).into_any_element()).collect(),
            Some(text(SETTINGS_UPDATES_COUNT_FIRST_CHECK_WHY).into()),
        )
    });
    let never = line(
        text(SETTINGS_UPDATES_COUNT_NEVER),
        vec![div().text_color(ui.text).child(text(SETTINGS_UPDATES_COUNT_NEVER_WHAT)).into_any_element()],
        None,
    );
    div()
        .flex()
        .flex_col()
        .pt_2()
        .when_some(starts, |d, starts| {
            d.child(div().pb_2().child(style::banner(ui, style::BannerKind::Info, false, starts)))
        })
        .child(every)
        .children(first)
        .child(never)
        .into_any_element()
}

/// The width of the labels of what an update check sends.
const DETAIL_LABEL_W: f32 = 220.0;
/// The header of the first check of a day and of a month, as `update::Count` sends it.
const COUNT_HEADERS: [&str; 2] = ["Wormhole-Count: day", "Wormhole-Count: month"]; // text: protocol, sent as is

/// The meter of a download.
const METER_W: f32 = 120.0;

/// "Checked 4 minutes ago", from the last check's time.
fn checked_ago(last: u64, now: u64) -> String {
    let minutes = now.saturating_sub(last) / 60;
    match minutes {
        0 => text(SETTINGS_UPDATES_CHECKED_JUST_NOW).into(),
        1 => text(SETTINGS_UPDATES_CHECKED_A_MINUTE_AGO).into(),
        2..=59 => settings_updates_checked_minutes_ago(whole(minutes)),
        60..=119 => text(SETTINGS_UPDATES_CHECKED_AN_HOUR_AGO).into(),
        _ if minutes < 48 * 60 => settings_updates_checked_hours_ago(whole(minutes / 60)),
        _ => settings_updates_checked_days_ago(whole(minutes / (24 * 60))),
    }
}

/// "Security update available: Wormhole 0.3.1." for a security update, else "Version 0.3.1 is available."
fn available_text(release: &Release) -> String {
    if release.is_security_update() {
        settings_updates_security_available(&release.version.to_string())
    } else {
        settings_updates_version_available(&release.version.to_string())
    }
}

/// What the update fixes, from the (unsigned, so only advisory) severity in the feed.
fn security_line(security: &Security) -> String {
    match security.severity {
        Severity::Critical => text(SETTINGS_UPDATES_FIXES_CRITICAL),
        Severity::High => text(SETTINGS_UPDATES_FIXES_HIGH),
        Severity::Medium => text(SETTINGS_UPDATES_FIXES_MEDIUM),
        Severity::Low => text(SETTINGS_UPDATES_FIXES_LOW),
        Severity::Unknown => text(SETTINGS_UPDATES_FIXES_A_SECURITY_ISSUE),
    }
    .into()
}

/// Where GitHub shows an advisory. The id has passed the feed's strict format check, so it can't
/// carry a path or a query.
fn advisory_url(id: &str) -> String {
    format!("https://github.com/wormhole-terminal/wormhole/security/advisories/{id}")
}

/// Critical and high read as danger, the rest as a warning.
fn security_color(ui: UiColors, severity: Severity) -> Hsla {
    match severity {
        Severity::Critical | Severity::High => ui.danger_text,
        Severity::Medium | Severity::Low | Severity::Unknown => ui.warning_text,
    }
}

/// Where an update stands, with the button for the next step and the release notes.
fn draw_status(_: &SettingsWindow, cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let updates = cx.global::<Updates>();
    let (phase, busy, available, (done, total), last, restart_pending) = (
        updates.phase.clone(),
        updates.busy(),
        updates.available(),
        updates.progress(),
        updates.last_check(),
        updates.restart_pending(),
    );
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());

    let status: SharedString = match &phase {
        UpdatePhase::Idle if !available => text(SETTINGS_UPDATES_DEVELOPMENT_BUILD).into(),
        UpdatePhase::Idle => match last {
            Some(last) => checked_ago(last, now).into(),
            None => text(SETTINGS_UPDATES_NOT_CHECKED_YET).into(),
        },
        UpdatePhase::Checking => text(SETTINGS_UPDATES_CHECKING).into(),
        UpdatePhase::UpToDate => match last {
            Some(last) => settings_updates_up_to_date_checked(&checked_ago(last, now)).into(),
            None => text(SETTINGS_UPDATES_WORMHOLE_IS_UP_TO_DATE).into(),
        },
        UpdatePhase::Available(release) => available_text(release).into(),
        UpdatePhase::Unsupported(message) | UpdatePhase::Failed(message) => message.clone().into(),
        UpdatePhase::Downloading => text(SETTINGS_UPDATES_DOWNLOADING).into(),
        UpdatePhase::Installed(version) if restart_pending => {
            settings_updates_installed_waits(version).into()
        }
        UpdatePhase::Installed(version) => settings_updates_installed_restart(version).into(),
    };
    let status_color = match &phase {
        UpdatePhase::Failed(_) | UpdatePhase::Unsupported(_) => ui.warning_text,
        UpdatePhase::Installed(_) | UpdatePhase::UpToDate => ui.success_text,
        UpdatePhase::Available(release) => match &release.security {
            Some(security) => security_color(ui, security.severity),
            None => ui.text,
        },
        _ => ui.muted,
    };

    let mut actions = div().flex().items_center().gap_2();
    match &phase {
        UpdatePhase::Available(release) => {
            let (install, skip) = ((**release).clone(), (**release).clone());
            // A security update can't be skipped: checks would offer it again anyway.
            if !release.is_security_update() {
                actions = actions.child(
                    Button::new("update-skip")
                        .ghost()
                        .small()
                        .label(text(SETTINGS_UPDATES_SKIP_THIS_VERSION))
                        .on_click(cx.listener(move |_, _, _, cx| crate::update::skip_update(&skip, cx))),
                );
            }
            actions = actions.child(
                Button::new("update-install")
                    .primary()
                    .small()
                    .label(text(SETTINGS_UPDATES_INSTALL_UPDATE))
                    .on_click(
                        cx.listener(move |_, _, _, cx| crate::update::install_update(install.clone(), cx)),
                    ),
            );
        }
        UpdatePhase::Installed(_) => {
            // Restarting asks first while something runs; waiting for the terminals to be idle
            // can be called off, or cut short by restarting now (no second question: it was asked).
            if restart_pending {
                actions = actions.child(
                    Button::new("update-restart-cancel")
                        .ghost()
                        .small()
                        .label(text(SETTINGS_UPDATES_DONT_RESTART))
                        .on_click(cx.listener(|_, _, _, cx| crate::update::cancel_restart_when_idle(cx))),
                );
            }
            actions = actions.child(
                Button::new("update-restart")
                    .primary()
                    .small()
                    .label(text(SETTINGS_UPDATES_RESTART_NOW))
                    .on_click(cx.listener(move |_, _, window, cx| {
                        if restart_pending {
                            crate::update::restart_to_update(cx);
                        } else {
                            crate::update::restart::request_restart(window, cx);
                        }
                    })),
            );
        }
        UpdatePhase::Checking => {
            actions = actions.child(Button::new("update-checking").ghost().small().loading(true).label(""));
        }
        UpdatePhase::Downloading => {
            let percent = (done * 100).checked_div(total).unwrap_or(0);
            actions = actions
                .child(div().w(px(METER_W)).child(Progress::new("update-meter").value(percent as f32)))
                .child(div().text_caption().tabular_nums().text_color(ui.muted).child(format!("{percent}%")));
        }
        UpdatePhase::Idle if !available => {}
        _ => {
            actions = actions.child(
                Button::new("update-check").small().label(text(SETTINGS_UPDATES_CHECK_NOW)).on_click(
                    cx.listener(move |_, _, _, cx| {
                        if !busy {
                            crate::update::check_for_updates(true, cx);
                        }
                    }),
                ),
            );
        }
    }

    let security = match &phase {
        UpdatePhase::Available(release) => release.security.clone(),
        _ => None,
    };
    let notes = match &phase {
        UpdatePhase::Available(release) if !release.notes.trim().is_empty() => {
            Some((release.version.to_string(), release.notes.clone()))
        }
        _ => None,
    };

    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_4()
                .min_h(px(style::SETTING_ROW_MIN_H))
                .py_2()
                .rule_top(ui)
                .child(div().text_body().text_color(status_color).child(status))
                .child(actions),
        )
        .children(security.map(|security| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .py_3()
                .rule_top(ui)
                .child(
                    div()
                        .text_body()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(security_color(ui, security.severity))
                        .child(security_line(&security)),
                )
                .when(!security.advisories.is_empty(), |d| {
                    d.child(div().flex().flex_wrap().items_center().gap_1().children(
                        security.advisories.iter().enumerate().map(|(ix, id)| {
                            let url = advisory_url(id);
                            style::link_button(("update-advisory", ix), ui, id.to_uppercase(), true)
                                .on_click(move |_, _, cx| cx.open_url(&url))
                        }),
                    ))
                })
        }))
        .children(notes.map(|(version, notes)| {
            div()
                .flex()
                .flex_col()
                .gap_1()
                .py_3()
                .rule_top(ui)
                .child(
                    div()
                        .text_caption()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(ui.muted)
                        .child(settings_updates_whats_new(&version)),
                )
                .children(
                    notes
                        .lines()
                        .map(|line| line.trim().trim_start_matches(['-', '*']).trim().to_string())
                        .filter(|line| !line.is_empty())
                        .map(|line| {
                            div()
                                .flex()
                                .gap_2()
                                .text_body()
                                .text_color(ui.muted)
                                .child(div().flex_none().text_color(ui.faint).child("–"))
                                .child(div().min_w_0().child(line))
                        }),
                )
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use wormhole_core::update::{Asset, Release, Security, Severity};

    use super::{advisory_url, available_text, checked_ago, security_line};

    #[test]
    fn the_last_check_reads_as_an_age() {
        assert_eq!(checked_ago(1000, 1010), "Checked just now.");
        assert_eq!(checked_ago(0, 4 * 60), "Checked 4 minutes ago.");
        assert_eq!(checked_ago(0, 3 * 3600), "Checked 3 hours ago.");
        assert_eq!(checked_ago(0, 3 * 86400), "Checked 3 days ago.");
        assert_eq!(checked_ago(100, 0), "Checked just now.", "a clock that went back");
    }

    fn release(security: Option<Security>) -> Release {
        Release {
            version: semver::Version::new(0, 3, 1),
            notes: String::new(),
            pub_date: None,
            minimum_system_version: None,
            asset: Asset { url: String::new(), sha256: String::new(), signature: String::new(), size: 0 },
            security,
        }
    }

    #[test]
    fn a_security_update_says_so() {
        assert_eq!(available_text(&release(None)), "Version 0.3.1 is available.");
        let high = Security { severity: Severity::High, advisories: Vec::new() };
        assert_eq!(
            available_text(&release(Some(high.clone()))),
            "Security update available: Wormhole 0.3.1."
        );
        assert_eq!(security_line(&high), "Fixes a high-severity security issue.");
        let unknown = Security { severity: Severity::Unknown, advisories: Vec::new() };
        assert_eq!(security_line(&unknown), "Fixes a security issue.");
    }

    #[test]
    fn an_advisory_links_to_its_page() {
        assert_eq!(
            advisory_url("ghsa-cfgh-2345-rvwx"),
            "https://github.com/wormhole-terminal/wormhole/security/advisories/ghsa-cfgh-2345-rvwx"
        );
    }
}
