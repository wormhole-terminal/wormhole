//! What a hook's JSON payload adds to its event name: the detail [`super::signal`] reads after a
//! `:` (`Notification:idle_prompt`, `SessionEnd:clear`, `PreToolUse:AskUserQuestion`), and the
//! words the sidebar shows while the agent needs input ("Approve cargo test?").

use serde_json::Value;

/// The longest message kept: the sidebar shows one line of it.
const MESSAGE_MAX_CHARS: usize = 200;

/// Background work that wakes the agent again when it ends. A background shell can run forever
/// (`tail -f`, a dev server), so it doesn't keep a finished turn working.
const WAKING_TASKS: &[&str] = &["subagent", "workflow", "teammate", "cloud session"];

/// `event` with the payload's detail appended (`Event:detail`), and the message to show.
pub fn describe(event: &str, payload: &Value) -> (String, Option<String>) {
    let text = |key: &str| payload.get(key).and_then(Value::as_str);
    let detail = match event {
        _ if text("notification_type").is_some() => text("notification_type").map(str::to_string),
        "SessionEnd" => text("reason").map(str::to_string),
        "StopFailure" => text("error").map(str::to_string),
        "PreToolUse" | "PermissionRequest" => text("tool_name").map(str::to_string),
        "Stop" => waits_for_background(payload).then(|| "background".to_string()),
        _ => None,
    };
    let message = match event {
        "PermissionRequest" => approval(payload),
        "PreToolUse" => question(payload),
        _ => text("message").map(str::to_string),
    };
    let event = match detail {
        Some(detail) => format!("{event}:{detail}"),
        None => event.to_string(),
    };
    (event, message.map(|m| clip(m.trim())).filter(|m| !m.is_empty()))
}

/// A turn ended while a background agent still runs: it wakes the session when it's done.
fn waits_for_background(payload: &Value) -> bool {
    let Some(tasks) = payload.get("background_tasks").and_then(Value::as_array) else { return false };
    tasks.iter().any(|task| {
        let field = |key: &str| task.get(key).and_then(Value::as_str).unwrap_or_default();
        WAKING_TASKS.contains(&field("type")) && matches!(field("status"), "running" | "pending")
    })
}

/// What a permission prompt asks, in the sidebar's words: "Approve cargo test?".
fn approval(payload: &Value) -> Option<String> {
    let tool = payload.get("tool_name").and_then(Value::as_str)?;
    let input = |key: &str| payload.get("tool_input")?.get(key)?.as_str();
    let what = match tool {
        "Bash" => input("command").and_then(|c| c.lines().find(|l| !l.trim().is_empty())).map(str::to_string),
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => input("file_path")
            .or_else(|| input("notebook_path"))
            .and_then(|path| path.rsplit('/').next())
            .map(|name| format!("editing {name}")),
        "WebFetch" => input("url").map(|url| format!("fetching {}", host(url))),
        _ => None,
    };
    // An MCP tool is `mcp__<server>__<tool>`: the tool's own name says more.
    let tool = tool.rsplit("__").next().unwrap_or(tool);
    Some(format!("Approve {}?", what.unwrap_or_else(|| tool.to_string())))
}

/// The question an agent asks with its own dialog (AskUserQuestion), or the plan it wants
/// approved (ExitPlanMode). Other tools ask nothing.
fn question(payload: &Value) -> Option<String> {
    match payload.get("tool_name").and_then(Value::as_str)? {
        "AskUserQuestion" => {
            payload.get("tool_input")?.get("questions")?.get(0)?.get("question")?.as_str().map(str::to_string)
        }
        "ExitPlanMode" => Some("Approve the plan?".to_string()),
        _ => None,
    }
}

fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
}

fn clip(text: &str) -> String {
    match text.char_indices().nth(MESSAGE_MAX_CHARS) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::describe;

    #[test]
    fn notifications_keep_their_type_and_message() {
        let payload =
            json!({ "notification_type": "idle_prompt", "message": "Claude is waiting for your input" });
        assert_eq!(
            describe("Notification", &payload),
            ("Notification:idle_prompt".into(), Some("Claude is waiting for your input".into()))
        );
        assert_eq!(describe("Notification", &json!({})), ("Notification".into(), None));
    }

    #[test]
    fn a_permission_request_says_what_to_approve() {
        let bash = json!({ "tool_name": "Bash", "tool_input": { "command": "\ncargo test -p core\necho" } });
        assert_eq!(
            describe("PermissionRequest", &bash),
            ("PermissionRequest:Bash".into(), Some("Approve cargo test -p core?".into()))
        );
        let edit = json!({ "tool_name": "Edit", "tool_input": { "file_path": "/repo/src/main.rs" } });
        assert_eq!(describe("PermissionRequest", &edit).1.as_deref(), Some("Approve editing main.rs?"));
        let fetch = json!({ "tool_name": "WebFetch", "tool_input": { "url": "https://docs.rs/serde?x=1" } });
        assert_eq!(describe("PermissionRequest", &fetch).1.as_deref(), Some("Approve fetching docs.rs?"));
        let mcp = json!({ "tool_name": "mcp__github__create_issue", "tool_input": {} });
        assert_eq!(describe("PermissionRequest", &mcp).1.as_deref(), Some("Approve create_issue?"));
    }

    #[test]
    fn a_question_or_a_plan_is_the_message() {
        let ask = json!({
            "tool_name": "AskUserQuestion",
            "tool_input": { "questions": [{ "question": "Which database?" }] }
        });
        assert_eq!(
            describe("PreToolUse", &ask),
            ("PreToolUse:AskUserQuestion".into(), Some("Which database?".into()))
        );
        let plan = json!({ "tool_name": "ExitPlanMode", "tool_input": { "plan": "…" } });
        assert_eq!(describe("PreToolUse", &plan).1.as_deref(), Some("Approve the plan?"));
        assert_eq!(describe("PreToolUse", &json!({ "tool_name": "Read" })), ("PreToolUse:Read".into(), None));
    }

    #[test]
    fn only_a_background_agent_keeps_a_turn_going() {
        let tasks = |kind: &str, status: &str| json!({ "background_tasks": [{ "id": "1", "type": kind, "status": status }] });
        assert_eq!(describe("Stop", &tasks("subagent", "running")).0, "Stop:background");
        assert_eq!(describe("Stop", &tasks("shell", "running")).0, "Stop");
        assert_eq!(describe("Stop", &tasks("subagent", "completed")).0, "Stop");
        assert_eq!(describe("Stop", &json!({ "background_tasks": [] })).0, "Stop");
    }

    #[test]
    fn session_ends_and_failures_carry_their_reason() {
        assert_eq!(describe("SessionEnd", &json!({ "reason": "clear" })).0, "SessionEnd:clear");
        assert_eq!(describe("StopFailure", &json!({ "error": "rate_limit" })).0, "StopFailure:rate_limit");
    }

    #[test]
    fn long_messages_are_clipped() {
        let long = "x".repeat(300);
        let (_, message) = describe("Notification", &json!({ "message": long }));
        assert_eq!(message.map(|m| m.chars().count()), Some(201));
    }
}
