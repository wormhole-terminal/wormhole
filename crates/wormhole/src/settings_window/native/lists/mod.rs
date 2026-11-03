//! The rows the native window cannot have in its catalog because they are things found on this Mac: the agents,
//! the projects, the Probes, the terminals to import from, the profile. A list is a group's rows after its
//! catalog ones (`"<page>:<group>"`), each an entry with an `id` (the row's key is the group's plus the id) and
//! what the window draws. A button on one comes back as an action `{"k": row key, "v": the button's id}`,
//! which the module of the list carries out through the code the GPUI window uses.

mod agents;
mod import;
mod import_sheets;
mod info;
mod probes;
mod profile;
mod projects;

use std::path::PathBuf;

use gpui_kit::App;
use serde_json::{Map, Value, json};

use crate::settings_window::sections::AgentsSnapshot;

/// What the lists read from disk, kept between snapshots: reading it is slow, and nothing changes it but the
/// window's own buttons, which read it again.
#[derive(Default)]
pub(crate) struct Cache {
    agents: Option<AgentsSnapshot>,
    /// Projects worth offering (`project::known_projects`: file checks).
    projects: Vec<PathBuf>,
    /// The Probes folder as last read.
    probes: Option<probes::Probes>,
    /// The project "For" shows on the Project folders page.
    selected: projects::Selected,
    /// The terminals found to import from; `None` until the scan finishes.
    import: Option<Vec<wormhole_core::import::Found>>,
}

fn cache(cx: &App) -> Option<&Cache> {
    cx.try_global::<super::Native>().map(|native| &native.lists)
}

fn cache_mut(cx: &mut App) -> &mut Cache {
    &mut cx.global_mut::<super::Native>().lists
}

/// One entry of a list, built up field by field.
pub(super) struct Entry(Map<String, Value>);

impl Entry {
    pub(super) fn new(id: &str, kind: &str, label: &str) -> Self {
        let mut map = Map::new();
        map.insert("id".into(), id.into());
        map.insert("kind".into(), kind.into());
        map.insert("label".into(), label.into());
        Self(map)
    }

    pub(super) fn with(mut self, field: &str, value: impl Into<Value>) -> Self {
        self.0.insert(field.into(), value.into());
        self
    }

    pub(super) fn into_value(self) -> Value {
        Value::Object(self.0)
    }
}

/// The profile's name and avatar letter, for the window's sidebar row.
pub(super) fn profile(cx: &App) -> Value {
    profile::identity(cx)
}

/// Every list and sheet, for the snapshot: `({"<page>:<group>": [entry…]}, {"<id>": sheet})`.
pub(super) fn build(cx: &mut App) -> (Value, Value) {
    let mut lists = Map::new();
    let mut sheets = Map::new();
    agents::lists(&mut lists, cx);
    projects::lists(&mut lists, cx);
    import::lists(&mut lists, &mut sheets, cx);
    profile::lists(&mut lists, cx);
    probes::lists(&mut lists, &mut sheets, cx);
    info::sheets(&mut sheets, cx);
    (json!(lists), json!(sheets))
}

/// Carry out a button of a listed row. Whether the row was one of the lists'.
pub(super) fn perform(key: &str, id: &str, cx: &mut App) -> bool {
    agents::perform(key, id, cx)
        || projects::perform(key, id, cx)
        || probes::perform(key, id, cx)
        || import::perform(key, id, cx)
        || profile::perform(key, id, cx)
}

/// Read what the lists need from disk or the database, in the background where it is slow; the window is
/// pushed a snapshot when it arrives.
pub(super) fn load(cx: &mut App) {
    agents::load(cx);
    projects::load(cx);
    probes::load(cx);
    import::load(cx);
}

/// The hubs changed: the chosen project's files are read again.
pub(super) fn projects_changed(cx: &mut App) {
    projects::refresh(cx);
}

/// The Extensions page's footer buttons.
pub(super) fn extensions_footer(id: &str, cx: &mut App) {
    match id {
        "show_in_finder" => probes::show_folder(cx),
        "scan_again" => probes::load(cx),
        _ => {}
    }
}
