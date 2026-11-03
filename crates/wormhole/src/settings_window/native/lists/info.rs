//! Two explanations the window opens as a sheet: how an agent's status is known with hooks and without, and what an
//! update check sends. Both are the app's words and table, as data: `{title, done, intro, rows}`, each row a
//! `label` over `values` (shown as code when `code`) with a `why` under it.

use std::sync::OnceLock;

use gpui_kit::App;
use serde_json::{Map, Value, json};
use wormhole_core::update::{counted_user_agent, current_platform, macos_version, plain_user_agent};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use crate::settings::settings;
use crate::settings_window::native::rows::row_label;
use crate::settings_window::native::update_words::COUNT_HEADERS;
use crate::update::Updates;

/// The id of the status explanation's sheet and of the update check's.
pub(super) const STATUS: &str = "status-how";
pub(super) const UPDATE_SENDS: &str = "update-sends";

fn row(label: &str, values: Vec<String>, code: bool, why: Option<&str>) -> Value {
    json!({ "label": label, "values": values, "code": code, "why": why.unwrap_or_default() })
}

fn status() -> Value {
    json!({
        "title": super::super::words::word("How status works"), // text: a native word, translated by its id
        "done": text(SETTINGS_IMPORT_DONE),
        "intro": [],
        "rows": [
            row(text(SETTINGS_AGENTS_WITH_STATUS_HOOKS), vec![text(SETTINGS_AGENTS_THE_AGENT_TELLS_WORMHOLE_THE_MOMENT).into()], false, None),
            row(text(SETTINGS_AGENTS_WITHOUT), vec![text(SETTINGS_AGENTS_STATUS_IS_READ_FROM_THE_TERMINAL).into()], false, None),
        ],
        "note": text(SETTINGS_AGENTS_HOOKS_DO_NOTHING_OUTSIDE_WORMHOLE_ONLY),
    })
}

fn update_sends(cx: &App) -> Value {
    // The version and the Mac don't change while the app runs; the system's version is read once.
    static MAC: OnceLock<Option<(semver::Version, String, Option<semver::Version>)>> = OnceLock::new();
    let mac = MAC.get_or_init(|| {
        let version = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
        Some((version, current_platform(), macos_version()))
    });
    let (counting, channel) = (settings(cx).anonymous_count, settings(cx).update_channel);
    let agent = mac.as_ref().map(|(version, platform, system)| match counting {
        true => counted_user_agent(version, channel, platform, system.as_ref()),
        false => plain_user_agent(version),
    });
    let starts = counting
        .then(|| cx.global::<Updates>().count_starts_in())
        .flatten()
        .map(|left| settings_updates_count_starts_in(whole(left.as_secs().div_ceil(3600))));
    let mut rows = vec![row(
        text(SETTINGS_UPDATES_COUNT_EVERY_CHECK),
        agent.into_iter().collect(),
        true,
        Some(text(if counting {
            SETTINGS_UPDATES_COUNT_EVERY_CHECK_COUNTED
        } else {
            SETTINGS_UPDATES_COUNT_EVERY_CHECK_PLAIN
        })),
    )];
    if counting {
        rows.push(row(
            text(SETTINGS_UPDATES_COUNT_FIRST_CHECK),
            COUNT_HEADERS.iter().map(ToString::to_string).collect(),
            true,
            Some(text(SETTINGS_UPDATES_COUNT_FIRST_CHECK_WHY)),
        ));
    }
    rows.push(row(
        text(SETTINGS_UPDATES_COUNT_NEVER),
        vec![text(SETTINGS_UPDATES_COUNT_NEVER_WHAT).into()],
        false,
        None,
    ));
    json!({
        "title": row_label("count-details"), "done": text(SETTINGS_IMPORT_DONE),
        "intro": starts.into_iter().collect::<Vec<_>>(), "rows": rows, "note": "",
    })
}

pub(super) fn sheets(out: &mut Map<String, Value>, cx: &App) {
    out.insert(STATUS.into(), status());
    out.insert(UPDATE_SENDS.into(), update_sends(cx));
}
