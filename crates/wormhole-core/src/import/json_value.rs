//! Reading JSON settings (Zed's and VS Code's, once their comments are gone) by dotted path.

use serde_json::Value;

/// The value at a dotted path of nested objects (`terminal.font_size`).
pub(crate) fn at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(root, |value, part| value.get(part))
}

/// `path = value`, for the review list.
pub(crate) fn shown(path: &str, value: &Value) -> String {
    format!("{path} = {value}")
}
