//! Keys that reach into an object: `motion.insert` is the `insert` key of the `motion` object. The
//! settings index names what a row writes by these, so one row of an object's several has its own
//! changed mark and its own reset. Plain keys are paths of one step.

use serde_json::{Map, Value};

/// The value at `key` in a settings map, if there is one.
pub(super) fn get<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    match key.split_once('.') {
        None => map.get(key),
        Some((object, rest)) => match map.get(object)? {
            Value::Object(inner) => get(inner, rest),
            _ => None,
        },
    }
}

/// Put `value` at `key`, in the object it names. A path through something that is not an object
/// changes nothing: the settings read back wrong and the caller keeps what it had.
pub(super) fn set(map: &mut Map<String, Value>, key: &str, value: Value) {
    match key.split_once('.') {
        None => {
            map.insert(key.to_string(), value);
        }
        Some((object, rest)) => {
            if let Some(Value::Object(inner)) = map.get_mut(object) {
                set(inner, rest, value);
            }
        }
    }
}

/// Take the value at `key` out of the map, in the object it names: a setting that is stored only
/// while it differs from its default is put back to its default by removing it.
pub(super) fn remove(map: &mut Map<String, Value>, key: &str) {
    match key.split_once('.') {
        None => {
            map.remove(key);
        }
        Some((object, rest)) => {
            if let Some(Value::Object(inner)) = map.get_mut(object) {
                remove(inner, rest);
            }
        }
    }
}

/// The key of the setting itself: `motion` for `motion.insert`.
pub(super) fn top(key: &str) -> &str {
    key.split_once('.').map_or(key, |(object, _)| object)
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::{get, remove, set, top};

    fn map(value: Value) -> Map<String, Value> {
        match value {
            Value::Object(map) => map,
            other => panic!("not an object: {other}"),
        }
    }

    #[test]
    fn a_path_reads_and_writes_inside_an_object() {
        let mut settings = map(json!({ "blur": true, "motion": { "insert": "reveal", "fold": "tuck" } }));
        assert_eq!(get(&settings, "blur"), Some(&json!(true)));
        assert_eq!(get(&settings, "motion.insert"), Some(&json!("reveal")));
        assert_eq!(get(&settings, "motion.nope"), None);
        assert_eq!(get(&settings, "blur.insert"), None, "a switch has no keys inside");
        set(&mut settings, "motion.insert", json!("classic"));
        assert_eq!(settings["motion"], json!({ "insert": "classic", "fold": "tuck" }), "the sibling stays");
        set(&mut settings, "blur.insert", json!(1));
        assert_eq!(settings["blur"], json!(true), "a path through a switch changes nothing");
        remove(&mut settings, "motion.insert");
        assert_eq!(settings["motion"], json!({ "fold": "tuck" }), "the sibling stays");
        assert_eq!((top("motion.insert"), top("blur")), ("motion", "blur"));
    }
}
