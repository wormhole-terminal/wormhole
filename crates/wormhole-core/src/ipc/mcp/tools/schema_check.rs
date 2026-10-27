//! A small JSON Schema checker for the tests: the keywords the tools' schemas use, and a failure for any
//! other, so a typo in a schema (`maxLenght`) is found instead of ignored.

use serde_json::{Value, json};

pub(super) const KEYWORDS: &[&str] = &[
    "type",
    "properties",
    "required",
    "additionalProperties",
    "description",
    "enum",
    "minimum",
    "maximum",
    "minLength",
    "maxLength",
    "minItems",
    "maxItems",
    "uniqueItems",
    "items",
    "oneOf",
];

pub(in crate::ipc::mcp) fn valid(schema: &Value, value: &Value) -> Result<(), String> {
    let Value::Object(schema) = schema else { return Err(format!("a schema is an object, not {schema}")) };
    if let Some(unknown) = schema.keys().find(|key| !KEYWORDS.contains(&key.as_str())) {
        return Err(format!("the checker does not know the keyword {unknown}"));
    }
    if let Some(kind) = schema.get("type").and_then(Value::as_str) {
        let ok = match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            other => return Err(format!("type {other}")),
        };
        if !ok {
            return Err(format!("{value} is not a {kind}"));
        }
    }
    if let Some(words) = schema.get("enum").and_then(Value::as_array)
        && !words.contains(value)
    {
        return Err(format!("{value} is not one of {words:?}"));
    }
    if let Some(number) = value.as_i64().or_else(|| value.as_u64().and_then(|n| i64::try_from(n).ok())) {
        if schema.get("minimum").and_then(Value::as_i64).is_some_and(|min| number < min) {
            return Err(format!("{number} is below the minimum"));
        }
        if schema.get("maximum").and_then(Value::as_i64).is_some_and(|max| number > max) {
            return Err(format!("{number} is above the maximum"));
        }
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        if schema.get("minLength").and_then(Value::as_u64).is_some_and(|min| length < min) {
            return Err("too short".into());
        }
        if schema.get("maxLength").and_then(Value::as_u64).is_some_and(|max| length > max) {
            return Err("too long".into());
        }
    }
    if let Value::Array(items) = value {
        let count = items.len() as u64;
        if schema.get("minItems").and_then(Value::as_u64).is_some_and(|min| count < min) {
            return Err("too few items".into());
        }
        if schema.get("maxItems").and_then(Value::as_u64).is_some_and(|max| count > max) {
            return Err("too many items".into());
        }
        if let Some(item_schema) = schema.get("items") {
            items.iter().try_for_each(|item| valid(item_schema, item))?;
        }
    }
    if let Value::Object(object) = value {
        for name in schema.get("required").and_then(Value::as_array).into_iter().flatten() {
            let name = name.as_str().expect("names");
            if !object.contains_key(name) {
                return Err(format!("{name} is required"));
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (name, member) in object {
            match properties.and_then(|properties| properties.get(name)) {
                Some(property) => valid(property, member).map_err(|why| format!("{name}: {why}"))?,
                None if schema.get("additionalProperties") == Some(&json!(false)) => {
                    return Err(format!("{name} is not a property"));
                }
                None => {}
            }
        }
    }
    if let Some(options) = schema.get("oneOf").and_then(Value::as_array) {
        let matching = options.iter().filter(|option| valid(option, value).is_ok()).count();
        if matching != 1 {
            return Err(format!("{value} matches {matching} of the options"));
        }
    }
    Ok(())
}

/// Every schema node, to see that its keywords are known (a typo such as `maxLenght` would be ignored by a
/// client, and the bound would be gone).
pub(in crate::ipc::mcp) fn keywords_known(schema: &Value) -> Result<(), String> {
    let Value::Object(schema) = schema else { return Ok(()) };
    if let Some(unknown) = schema.keys().find(|key| !KEYWORDS.contains(&key.as_str())) {
        return Err(unknown.clone());
    }
    for properties in schema.get("properties").and_then(Value::as_object).into_iter() {
        properties.values().try_for_each(keywords_known)?;
    }
    for sub in schema
        .get("items")
        .into_iter()
        .chain(schema.get("oneOf").and_then(Value::as_array).into_iter().flatten())
    {
        keywords_known(sub)?;
    }
    Ok(())
}
