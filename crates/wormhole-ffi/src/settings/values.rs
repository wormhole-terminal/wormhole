//! The settings as the window holds them: one flat map of JSON values keyed by the `config.json` keys
//! the settings index names (a path for a key inside an object: `motion.insert`). No Swift struct
//! mirrors `Settings`; a row reads its keys from this map and writes them back by key.

use std::sync::LazyLock;

use serde_json::{Map, Value};
use wormhole_core::config::Settings;
use wormhole_core::config::settings_index::entries;

use crate::settings_dump::{get, tidy};

/// Every key some row writes, once, in the pages' order.
pub(crate) fn schema_keys() -> &'static [String] {
    static KEYS: LazyLock<Vec<String>> = LazyLock::new(|| {
        let mut keys: Vec<String> = Vec::new();
        for key in entries().flat_map(|(_, _, entry)| entry.keys.iter().copied()) {
            if !keys.iter().any(|known| known == key) {
                keys.push(key.to_string());
            }
        }
        keys
    });
    &KEYS
}

/// The value of every schema key in `settings`. A key that holds nothing (a motion choice left at its
/// default is not stored) is absent: the schema's own default stands for it.
pub(crate) fn schema_values(settings: &Settings) -> Map<String, Value> {
    let root = serde_json::to_value(settings).unwrap_or(Value::Null);
    schema_keys().iter().filter_map(|key| get(&root, key).map(|value| (key.clone(), tidy(value)))).collect()
}

pub(crate) fn to_text(map: Map<String, Value>) -> String {
    serde_json::to_string(&Value::Object(map)).unwrap_or_else(|_| "{}".into())
}
