//! What a tool call comes back with, as MCP's `CallToolResult`.
//!
//! A result that worked has a short text (what `wh` prints, so a client that only shows the model `content`
//! still gives it everything) and, where the revision has it, `structuredContent`: the reply's `result`,
//! the same JSON `wh --json` prints under that key. A request the app refused, or could not reach, is a
//! result with `isError: true`, not a JSON-RPC error: it is the model's to read and act on (turn a
//! switch on, wait, choose another terminal). The code, reason and message are in the text and, whole, in
//! `_meta`, where they do not have to fit the tool's `outputSchema`.

use std::path::Path;

use serde_json::{Value, json};

use super::version::Version;
use crate::ipc::control::{Action, PROTOCOL_VERSION, Reply, render_with};

/// The `_meta` key of the error a failed call carries.
pub(super) const ERROR_META: &str = "app.getwormhole/error";

/// A finished tool call.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Outcome {
    pub(super) text: String,
    pub(super) structured: Option<Value>,
    /// Why the call failed, when it did: `code`, `message`, and `reason`/`block` when the app gave them.
    pub(super) error: Option<Value>,
}

impl Outcome {
    pub(super) fn done(text: impl Into<String>, structured: Value) -> Self {
        Self { text: text.into(), structured: Some(structured), error: None }
    }

    /// A call that failed before or without the app's say (the arguments, no app, no answer).
    pub(super) fn failed(code: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            text: message.clone(),
            structured: None,
            error: Some(json!({"code": code, "message": message})),
        }
    }

    pub(super) fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// The app's reply to `action`, as a result.
    pub(super) fn of_reply(action: &Action, reply: &Reply, home: Option<&Path>) -> Self {
        if reply.version != PROTOCOL_VERSION && reply.error.is_none() {
            return Self::failed(
                "version",
                format!(
                    "the running Wormhole speaks protocol {} and this MCP server {PROTOCOL_VERSION}",
                    reply.version
                ),
            );
        }
        match render_with(action, reply, home, false) {
            // (`render_with` fails only for a reply that has an error; should it ever fail for another, the call
            // is still an error and says so, not a success with nothing in it.)
            Err(words) => Self {
                text: words.clone(),
                structured: None,
                error: Some(
                    reply
                        .error
                        .as_ref()
                        .and_then(|error| serde_json::to_value(error).ok())
                        .unwrap_or_else(|| json!({"code": "failed", "message": words})),
                ),
            },
            Ok(text) => Self {
                text: text.trim_end().to_string(),
                structured: Some(reply.result.clone().unwrap_or_else(|| json!({}))),
                error: None,
            },
        }
    }

    /// The `CallToolResult` for a client of `version`.
    pub(super) fn to_value(&self, version: Version) -> Value {
        let mut result = json!({
            "content": [{"type": "text", "text": self.text}],
            "isError": self.is_error(),
        });
        if version.has_structured_output()
            && let Some(structured) = &self.structured
        {
            result["structuredContent"] = structured.clone();
        }
        if let Some(error) = &self.error {
            result["_meta"] = json!({ ERROR_META: error });
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ERROR_META, Outcome};
    use crate::ipc::control::{Action, Error, ErrorCode, Reason, Reply};
    use crate::ipc::mcp::version::Version;

    #[test]
    fn a_reply_that_worked_has_its_text_and_its_result_as_structured_content() {
        let reply = Reply::ok(json!({"tab": 4}));
        let outcome = Outcome::of_reply(&Action::List, &reply, None);
        assert!(!outcome.is_error());
        let value = outcome.to_value(Version::V2025_11_25);
        assert_eq!(value["isError"], json!(false));
        assert_eq!(value["structuredContent"], json!({"tab": 4}));
        assert_eq!(value["content"][0]["type"], json!("text"));
        // An earlier revision has no structured content.
        assert!(outcome.to_value(Version::V2025_03_26).get("structuredContent").is_none());
    }

    #[test]
    fn a_refusal_is_an_error_result_with_the_code_and_reason_in_the_text_and_in_meta() {
        let reply: Reply = Error::new(ErrorCode::Refused, "the shell is not at its prompt")
            .because(Reason::NotAtPrompt)
            .into();
        let outcome = Outcome::of_reply(&Action::List, &reply, None);
        let value = outcome.to_value(Version::V2025_06_18);
        assert_eq!(value["isError"], json!(true));
        let text = value["content"][0]["text"].as_str().expect("text");
        assert!(
            text.contains("not at its prompt") && text.contains("refused") && text.contains("not_at_prompt"),
            "{text}"
        );
        assert_eq!(
            value["_meta"][ERROR_META],
            json!({"code": "refused", "message": "the shell is not at its prompt", "reason": "not_at_prompt"})
        );
        assert!(value.get("structuredContent").is_none());
    }

    #[test]
    fn a_reply_from_another_protocol_is_an_error_not_a_guess() {
        let mut reply = Reply::ok(json!({}));
        reply.version = 9;
        let outcome = Outcome::of_reply(&Action::List, &reply, None);
        assert!(outcome.is_error() && outcome.text.contains("protocol 9"));
    }
}
