//! A saved window given new ids, for the profile a window moves into when that profile still holds a window saved
//! with the same ids (written before the profiles shared one counter: each numbered its windows and terminals from 1).
//! The GPUI app has no such case to handle: its counters start past every profile's ids, which `Windows::new` does too,
//! so a window it moves keeps its ids. What is renumbered here is always the target's dormant window, never the moving
//! one: a running terminal cannot change its id (its notes, its hooks' `WORMHOLE_TAB_ID` and its session host name it).

use std::collections::HashMap;

use serde_json::Value;
use wormhole_core::storage::store::{ClaimRecord, WindowRecord};
use wormhole_core::view::sidebar::Item;
use wormhole_core::workspace::SavedLayout;

/// `record` as window `window`, its terminals and splits under ids from `fresh`, and what each terminal was called
/// before (the keys of its text and its claims).
pub(super) fn reissued(
    record: &WindowRecord,
    window: i64,
    mut fresh: impl FnMut() -> i64,
) -> (WindowRecord, HashMap<i64, i64>) {
    let mut renamed = HashMap::new();
    for id in record.sessions.iter().map(|s| s.id).chain(record.splits.iter().map(|s| s.id)) {
        renamed.insert(id, fresh());
    }
    let new = |id: i64| renamed.get(&id).copied().unwrap_or(id);
    let mut out = record.clone();
    out.id = window;
    for session in &mut out.sessions {
        session.id = new(session.id);
        session.split_id = session.split_id.map(new);
    }
    for split in &mut out.splits {
        split.id = new(split.id);
        split.layout = split.layout.take().map(|json| mapped_json(&json, &new));
    }
    out.layout = out.layout.take().map(|json| {
        let Ok(mut layout) = serde_json::from_str::<SavedLayout>(&json) else { return json };
        for group in &mut layout.sidebar.groups.0 {
            group.members.iter_mut().for_each(|member| *member = new(*member));
        }
        for item in &mut layout.sidebar.items {
            if let Item::Terminal(id) = item {
                *id = new(*id);
            }
        }
        serde_json::to_string(&layout).unwrap_or(json)
    });
    (out, renamed)
}

/// The claims of renumbered terminals, under their new ids.
pub(super) fn renamed_claims(claims: Vec<ClaimRecord>, renamed: &HashMap<i64, i64>) -> Vec<ClaimRecord> {
    claims
        .into_iter()
        .map(|claim| ClaimRecord {
            session_id: renamed.get(&claim.session_id).copied().unwrap_or(claim.session_id),
            ..claim
        })
        .collect()
}

/// A split's layout (`TabPanes::to_json`): the panes are `leaf`s of its tree, and `focused` and `zoomed` name one.
fn mapped_json(json: &str, new: &impl Fn(i64) -> i64) -> String {
    fn walk(value: &mut Value, new: &impl Fn(i64) -> i64) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map.iter_mut() {
                    match (key.as_str(), inner.as_i64()) {
                        ("leaf" | "focused" | "zoomed", Some(id)) => *inner = Value::from(new(id)),
                        _ => walk(inner, new),
                    }
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|item| walk(item, new)),
            _ => {}
        }
    }
    let Ok(mut value) = serde_json::from_str::<Value>(json) else { return json.to_owned() };
    walk(&mut value, new);
    value.to_string()
}
