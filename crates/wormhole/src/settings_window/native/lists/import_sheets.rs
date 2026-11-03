//! Import's two sheets: what a terminal's settings would bring (each with its tick, the person unticks what they do
//! not want) and how each terminal compares with Wormhole. Both are data for the window to draw; the review's
//! ticks are the window's until it sends the ones left on with the Import button.

use serde_json::{Value, json};
use wormhole_core::import::{
    CHECKED, Found, Setting, SettingValue, WORMHOLE, command_line, comparison, visible,
};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::import_words::{Subject, gap_text, reason_text, setting_label, strength_text, value_text};
use crate::settings::import::terminal_name;

/// The id of the compare sheet and of a terminal's review sheet.
pub(super) const COMPARE: &str = "import-compare";

pub(super) fn review_id(found: &Found) -> String {
    format!("import-review-{}", found.source.key())
}

fn pair(found: &Found) -> bool {
    let has = |wanted: Setting| found.proposal.values.iter().any(|p| p.setting == wanted);
    has(Setting::Theme) && has(Setting::ThemeLight)
}

/// What the terminal's proposal would set, one row per setting with its tick as the proposal starts it.
pub(super) fn review(found: &Found) -> Value {
    let pair = pair(found);
    let rows: Vec<Value> = found
        .proposal
        .values
        .iter()
        .enumerate()
        .map(|(at, proposed)| {
            let (value, warn) = match &proposed.value {
                SettingValue::Profile(profile) => {
                    (command_line(profile).to_string(), text(SETTINGS_IMPORT_RUNS_EVERYWHERE))
                }
                other => (value_text(other), ""),
            };
            json!({
                "id": at.to_string(), "label": setting_label(proposed.setting, pair),
                "from": visible(&proposed.from), "value": value, "warn": warn, "on": proposed.default_on,
            })
        })
        .collect();
    let skipped: Vec<Value> = found
        .proposal
        .skipped
        .iter()
        .map(|skipped| json!([visible(&skipped.key), reason_text(&skipped.reason)]))
        .collect();
    json!({
        "kind": "review", "title": terminal_name(found.source), "rows": rows, "skipped": skipped,
        "take": text(SETTINGS_IMPORT_TAKE), "leave": text(SETTINGS_IMPORT_LEAVE),
        "nothing": text(SETTINGS_IMPORT_NOTHING_TO_TAKE), "apply": text(SETTINGS_IMPORT_APPLY),
    })
}

fn part(subject: Subject, comparison: &wormhole_core::import::Comparison, label: String) -> Value {
    let gaps: Vec<Value> = comparison
        .drawbacks
        .iter()
        .map(|drawback| {
            let several = drawback.sources.len() > 1;
            let sources: Vec<Value> = drawback
                .sources
                .iter()
                .enumerate()
                .map(|(n, url)| {
                    let words = if several {
                        settings_import_source_numbered(&(n + 1).to_string())
                    } else {
                        text(SETTINGS_IMPORT_SOURCE).to_string()
                    };
                    json!([words, url])
                })
                .collect();
            let as_of = if drawback.date != CHECKED || drawback.recheck {
                settings_import_as_of(drawback.date)
            } else {
                String::new()
            };
            json!({ "text": gap_text(subject, drawback.id).unwrap_or_default(), "asOf": as_of, "sources": sources })
        })
        .collect();
    json!({
        "title": subject.name(), "label": label, "gaps": gaps,
        "strength": strength_text(subject, comparison).unwrap_or_default(),
        "asOf": if subject == Subject::Wormhole { String::new() } else { settings_import_as_of(CHECKED) },
    })
}

/// Each terminal found against Wormhole, then Wormhole's own gaps.
pub(super) fn compare(all: &[Found]) -> Value {
    let mut sections: Vec<Value> = all
        .iter()
        .map(|found| {
            let subject = Subject::Terminal(found.source);
            part(subject, comparison(found.source), settings_import_compare_gaps(subject.name()))
        })
        .collect();
    if !all.is_empty() {
        sections.push(part(Subject::Wormhole, &WORMHOLE, text(SETTINGS_IMPORT_WORMHOLE_GAPS).to_string()));
    }
    json!({
        "kind": "compare", "title": text(SETTINGS_IMPORT_GROUP_COMPARE), "done": text(SETTINGS_IMPORT_DONE),
        "none": text(SETTINGS_IMPORT_COMPARE_NONE), "sections": sections,
    })
}
