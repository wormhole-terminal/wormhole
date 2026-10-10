//! What each agent's hook events mean for the session's status.

use super::AgentId;

/// What a hook event means for the session's status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Signal {
    /// The agent is working on a prompt.
    Busy,
    /// It's waiting for the user (permission prompt, question).
    Attention,
    /// It finished its turn.
    Done,
    /// Back at its prompt with nothing to report: you interrupted it, or `/clear` started over.
    Quiet,
    /// The session ended; back to the output heuristic.
    Ended,
}

/// Map an agent's hook event onto a status change. Notification events
/// arrive as `event:notification_type` when the payload names a type.
pub fn signal(agent: AgentId, event: &str) -> Option<Signal> {
    use AgentId::*;
    use Signal::*;
    let (event, kind) = event.split_once(':').unwrap_or((event, ""));
    match (agent, event) {
        // Its own dialogs (a question, the plan to approve) are tool calls that wait for you.
        (Claude | Qwen | Droid, "PreToolUse") if matches!(kind, "AskUserQuestion" | "ExitPlanMode") => {
            Some(Attention)
        }
        (Claude | Qwen | Droid, "UserPromptSubmit" | "PreToolUse" | "PostToolUse") => Some(Busy),
        // The moment it asks; its `permission_prompt` notification only comes ~6 s later.
        (Claude, "PermissionRequest") => Some(Attention),
        // Claude also notifies when it has sat idle for 60 s ("Claude is waiting for your
        // input"), after a login, when a background session finishes, …; only these are
        // questions. An untyped one (older versions, Droid) still counts.
        (Claude | Qwen | Droid, "Notification") => matches!(
            kind,
            "" | "permission_prompt"
                | "elicitation_dialog"
                | "elicitation_url_dialog"
                | "quota_auto_resume_stale"
        )
        .then_some(Attention),
        // A background agent wakes the session when it ends: still working.
        (Claude | Qwen | Droid, "Stop") if kind == "background" => Some(Busy),
        (Claude | Qwen | Droid, "Stop") => Some(Done),
        // The turn ended on an API error (rate limit, overload, …) instead of `Stop`.
        (Claude, "StopFailure") => Some(Done),
        (Codex, "UserPromptSubmit" | "PreToolUse" | "PostToolUse") => Some(Busy),
        (Codex, "PermissionRequest") => Some(Attention),
        (Codex, "Stop") => Some(Done),
        (Gemini, "BeforeAgent" | "BeforeTool" | "AfterTool") => Some(Busy),
        (Gemini, "Notification") => Some(Attention),
        (Gemini, "AfterAgent") => Some(Done),
        (Copilot, "userPromptSubmitted" | "preToolUse" | "postToolUse") => Some(Busy),
        // Copilot also notifies when idle or done; only questions need you.
        (Copilot, "notification") => {
            matches!(kind, "permission_prompt" | "elicitation_dialog").then_some(Attention)
        }
        (Copilot, "agentStop") => Some(Done),
        (Goose, "UserPromptSubmit" | "PreToolUse" | "PostToolUse") => Some(Busy),
        (Goose, "Stop") => Some(Done),
        (OpenCode, "session.busy" | "permission.replied") => Some(Busy),
        (OpenCode, "permission.asked") => Some(Attention),
        (OpenCode, "session.idle") => Some(Done),
        // `/clear` and `/resume` end the session, not the agent: it is back at its prompt.
        (_, "SessionEnd" | "sessionEnd") if matches!(kind, "clear" | "resume") => Some(Quiet),
        (_, "SessionEnd" | "sessionEnd") => Some(Ended),
        _ => None,
    }
}

/// A key typed into a terminal whose agent needs you: what answering its prompt means. Return, a
/// number or y/n picks an answer, so it works on; Esc or ⌃C turns the question down and stops
/// it. Anything else (the arrows, ⇥) only moves through the choices.
pub fn answer_key(key: &str, control: bool) -> Option<Signal> {
    match key {
        "c" if control => Some(Signal::Quiet),
        _ if control => None,
        "escape" => Some(Signal::Quiet),
        "enter" | "y" | "n" => Some(Signal::Busy),
        _ if key.len() == 1 && key.bytes().all(|b| b.is_ascii_digit()) => Some(Signal::Busy),
        _ => None,
    }
}
