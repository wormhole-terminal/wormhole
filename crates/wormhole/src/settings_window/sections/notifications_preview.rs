//! Notifications' preview: the banners that are turned on, as macOS draws them at the top right, compiled
//! from the design's `BannerMock` (`ui/generated/banner_mock.rs`). Which banners show follows the page's
//! switches; with three or more and "Group bursts" on they become one.

use gpui_kit::*;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::card::{Words, card};
use crate::settings::settings;
use crate::settings_window::{SettingsWindow, ui};
use crate::ui::{Banner, BannerMockProps, Look};

/// The most banners the picture holds.
const BANNERS: usize = 2;

/// The banners the settings would show, as (title, body).
fn banners(cx: &App) -> Vec<(String, String)> {
    let s = settings(cx);
    // text: sample (made-up notifications of the preview)
    let kinds = [
        (s.notify_kind_on(s.notify_needs_you), "api needs input", "Allow edit of src/limits.rs?"),
        (s.notify_kind_on(s.notify_agent_done), "Release notes finished", "Claude Code · 12m"),
        (s.notify_kind_on(s.notify_command_done), "cargo build finished", "2m 14s · ~/Projects/api"),
        (s.notify_kind_on(s.notify_command_failed), "cargo test failed", "Exit code 101 · ~/Projects/api"),
        (s.notify_kind_on(s.notify_bell), "scratch rang the bell", "At most once a minute per terminal"),
    ];
    let mut on: Vec<(String, String)> = kinds
        .into_iter()
        .filter(|(on, ..)| *on)
        .map(|(_, title, body)| (title.to_owned(), body.to_owned()))
        .collect();
    if s.notify_group_bursts && on.len() >= 3 {
        let titles = on.iter().map(|(title, _)| title.as_str()).collect::<Vec<_>>().join(" · ");
        on = vec![(settings_notifications_preview_grouped(&whole(on.len()).to_string()), titles)];
    }
    on.truncate(BANNERS);
    on
}

/// The banners.
pub(in crate::settings_window) fn draw_notifications_preview(cx: &mut Context<SettingsWindow>) -> AnyElement {
    let ui = ui(cx);
    let banners = banners(cx);
    let mock = BannerMockProps {
        id: "banner-mock".into(),
        spoken: text(SETTINGS_WINDOW_PREVIEW_NOTIFICATIONS_LABEL).into(),
        when: text(SETTINGS_NOTIFICATIONS_PREVIEW_NOW).into(),
        none: banners.is_empty(),
        empty: text(SETTINGS_NOTIFICATIONS_PREVIEW_EMPTY).into(),
        banners: banners
            .into_iter()
            .map(|(title, body)| Banner { title: title.into(), body: body.into() })
            .collect(),
    };
    card(
        ui,
        Words {
            id: "notifications-preview-card",
            what: SETTINGS_WINDOW_NOTIFICATIONS,
            spoken: SETTINGS_WINDOW_PREVIEW_NOTIFICATIONS_LABEL,
            caption: SETTINGS_NOTIFICATIONS_PREVIEW_CAPTION,
        },
        div().child(mock.render(&Look::of(cx))).overflow_hidden(),
    )
}
