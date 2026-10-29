//! Reading JSON that rejects a key given twice.
//!
//! `serde_json::Value` keeps the last of two equal keys and says nothing, so a theme with two
//! `"primary"` entries (a paste gone wrong) would silently use one of them. Version 1 files
//! are read through this instead; version 0 files keep being read as they always were.

use std::fmt;

use serde::Deserialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

use crate::storage::jsonc;

/// Parse JSONC text into a value; a key given twice in one object is an error that names it.
pub(super) fn parse_unique(text: &str) -> Result<Value, String> {
    jsonc::parse::<Unique>(text).map(|u| u.0)
}

struct Unique(Value);

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any JSON value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
                Ok(Value::from(v))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
                Ok(Value::from(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
                Ok(Number::from_f64(v).map_or(Value::Null, Value::Number))
            }

            fn visit_str<E>(self, v: &str) -> Result<Value, E> {
                Ok(Value::String(v.to_string()))
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(Unique(item)) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Value::Array(items))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut out = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if out.contains_key(&key) {
                        return Err(de::Error::custom(format!("{key:?} is given twice")));
                    }
                    let Unique(value) = map.next_value()?;
                    out.insert(key, value);
                }
                Ok(Value::Object(out))
            }
        }
        deserializer.deserialize_any(V).map(Unique)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_unique;

    #[test]
    fn a_key_given_twice_is_an_error_at_any_depth() {
        let e = parse_unique(r##"{"ui": {"primary": "#f00", "primary": "#0f0"}}"##).unwrap_err();
        assert!(e.contains("\"primary\" is given twice") && e.starts_with("line 1"), "{e}");
        assert!(parse_unique(r#"{"a": [{"b": 1, "b": 2}]}"#).is_err());
        assert!(parse_unique(r#"{"a": 1, "b": {"a": 2}, "c": [1, 2.5, null, true]}"#).is_ok());
    }

    #[test]
    fn comments_and_trailing_commas_are_still_allowed() {
        let v = parse_unique("{ // c\n \"a\": 1, }").unwrap();
        assert_eq!(v["a"], 1);
    }
}
