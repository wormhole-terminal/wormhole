//! Best-effort adapters for agent-owned reports. No credentials, network requests, or history
//! scans: Codex supplies a transcript path in its hook; Gemini supplies model usage directly.
//! Codex's rollout format is not a stable API. Unknown or incomplete records are ignored.
use std::path::Path;

use serde_json::Value;

use super::{PlanLimit, Usage};
use crate::agents::AgentId;
use crate::storage::fsutil::{self, LinkPolicy};

const TRANSCRIPT_TAIL: u64 = 1 << 20;

/// Called by the installed hooks, outside the app's UI process. The caller supplies the agent's
/// home, both to respect CODEX_HOME and to keep tests away from real agent data.
pub fn from_hook(agent: AgentId, event: &str, input: &Value, codex_home: &Path, now: i64) -> Option<Usage> {
    match (agent, event) {
        (AgentId::Codex, "Stop" | "PostToolUse" | "SessionEnd") => {
            let path = Path::new(input.get("transcript_path")?.as_str()?);
            let bytes =
                fsutil::read_tail_capped(path, TRANSCRIPT_TAIL, LinkPolicy::Within(codex_home)).ok()?;
            codex_tail(&bytes, now)
        }
        (AgentId::Gemini, "AfterModel") => gemini(input, now),
        _ => None,
    }
}

fn codex_tail(bytes: &[u8], now: i64) -> Option<Usage> {
    for line in bytes.rsplit(|b| *b == b'\n') {
        let Ok(record) = serde_json::from_slice::<Value>(line) else { continue };
        if record.get("type").and_then(Value::as_str) != Some("event_msg") {
            continue;
        }
        let Some(payload) = record.get("payload") else { continue };
        if payload.get("type").and_then(Value::as_str) != Some("token_count") {
            continue;
        }
        let Some(limits) = payload.get("rate_limits").filter(|v| v.is_object()) else { continue };
        // Additional model-specific quota buckets must not overwrite the account's Codex limits.
        if limits.get("limit_id").and_then(Value::as_str).is_some_and(|id| id != "codex") {
            continue;
        }
        // The latest account snapshot wins, even if it contains no usable limits. Never fall
        // back to older percentages after an empty snapshot or a change in the report format.
        let Some(updated) = report_time(&record, now) else { continue };
        return codex(limits, updated);
    }
    None
}

fn codex(input: &Value, now: i64) -> Option<Usage> {
    let limits: Vec<_> = [("primary", "Primary limit"), ("secondary", "Secondary limit")]
        .into_iter()
        .filter_map(|(key, fallback)| {
            let window = input.get(key)?;
            let used = window.get("used_percent")?.as_f64()?.clamp(0.0, 100.0) as f32;
            let minutes = window.get("window_minutes").and_then(Value::as_i64);
            let label = match minutes {
                Some(300) => "5-hour limit".into(),
                Some(10_080) => "Weekly limit".into(),
                Some(m) if m > 0 => format!("{} limit", super::duration(m.saturating_mul(60))),
                _ => fallback.into(),
            };
            Some(PlanLimit { label, used, resets_at: window.get("resets_at").and_then(Value::as_i64) })
        })
        .collect();
    (!limits.is_empty()).then_some(Usage {
        agent: AgentId::Codex,
        five_hour: None,
        seven_day: None,
        limits,
        response_tokens: None,
        updated: now,
    })
}

fn report_time(input: &Value, now: i64) -> Option<i64> {
    let text = input.get("timestamp")?.as_str()?;
    let timestamp = chrono::DateTime::parse_from_rfc3339(text).ok()?.timestamp();
    // A wildly future-dated source must not pin the account to that snapshot indefinitely.
    (timestamp <= now.saturating_add(300)).then_some(timestamp.min(now))
}

fn gemini(input: &Value, now: i64) -> Option<Usage> {
    let response_tokens =
        input.get("llm_response")?.get("usageMetadata")?.get("totalTokenCount")?.as_u64()?;
    Some(Usage {
        agent: AgentId::Gemini,
        five_hour: None,
        seven_day: None,
        limits: Vec::new(),
        response_tokens: Some(response_tokens),
        updated: report_time(input, now).unwrap_or(now),
    })
}

#[cfg(test)]
mod tests {
    use super::{AgentId, TRANSCRIPT_TAIL, codex, codex_tail, from_hook, gemini};
    use serde_json::json;
    const NOW: i64 = 1_800_000_000;

    fn limits() -> serde_json::Value {
        json!({"primary": {"used_percent": 42, "window_minutes": 300, "resets_at": NOW+600},
            "secondary": {"used_percent": 18, "window_minutes": 10080, "resets_at": NOW+86400}})
    }
    fn event(limits: serde_json::Value) -> String {
        json!({"timestamp":chrono::DateTime::from_timestamp(NOW, 0).unwrap().to_rfc3339(),
            "type":"event_msg", "payload":{"type":"token_count", "rate_limits":limits}})
        .to_string()
    }
    #[test]
    fn codex_windows_use_reported_durations_and_allow_unknown_resets() {
        let usage = codex(&limits(), NOW).unwrap();
        assert_eq!(usage.agent, AgentId::Codex);
        assert_eq!(usage.limits[0].label, "5-hour limit");
        assert_eq!(usage.limits[1].label, "Weekly limit");
        assert_eq!(usage.limits[0].shown(NOW), (0.42, "42% · resets 10m".into()));
        assert_eq!(usage.limits[0].shown(NOW + 601).1, "Awaiting update");
        let odd = codex(&json!({"primary":{"used_percent":150,"window_minutes":60}}), NOW).unwrap();
        assert_eq!(odd.limits[0].label, "1h limit");
        assert_eq!(odd.limits[0].shown(NOW), (1.0, "100%".into()));
    }
    #[test]
    fn only_latest_complete_account_report_is_used() {
        let first = event(limits());
        let second = event(json!({"limit_id":"codex","primary":{"used_percent":55}}));
        let other = event(json!({"limit_id":"codex_other","primary":{"used_percent":99}}));
        let text = format!("broken first line\n{first}\n{second}\n{other}\n{{partial");
        assert_eq!(codex_tail(text.as_bytes(), NOW).unwrap().limits[0].used, 55.0);
        assert!(codex_tail(format!("{first}\n{}", event(json!({}))).as_bytes(), NOW).is_none());
        assert!(codex_tail(br#"{"type":"response_item","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":99}}}}"#,NOW).is_none());
    }
    #[test]
    fn older_transcripts_do_not_look_fresh_when_another_terminal_reports() {
        let mut data: serde_json::Value = serde_json::from_str(&event(limits())).unwrap();
        data["timestamp"] = json!(chrono::DateTime::from_timestamp(NOW - 600, 0).unwrap().to_rfc3339());
        let report = codex_tail(data.to_string().as_bytes(), NOW).unwrap();
        assert_eq!(report.updated, NOW - 600);
        data["timestamp"] = json!("bad timestamp");
        assert!(codex_tail(data.to_string().as_bytes(), NOW).is_none());
        data["timestamp"] = json!(chrono::DateTime::from_timestamp(NOW + 600, 0).unwrap().to_rfc3339());
        assert!(codex_tail(data.to_string().as_bytes(), NOW).is_none());
    }

    #[test]
    fn gemini_reports_latest_response_tokens_not_a_quota_or_running_total() {
        let input = json!({"llm_response":{"usageMetadata":{"totalTokenCount":12345}}});
        let report = gemini(&input, NOW).unwrap();
        assert_eq!(report.response_tokens, Some(12345));
        assert!(report.plan_limits().is_empty());
        assert!(gemini(&json!({"llm_response":{}}), NOW).is_none());
        assert!(gemini(&json!({"llm_response":{"usageMetadata":{"totalTokenCount":-1}}}), NOW).is_none());
    }
    #[test]
    fn hook_reads_only_a_bounded_tail_inside_the_agent_home() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("rollout.jsonl");
        let mut content = vec![b'x'; TRANSCRIPT_TAIL as usize + 100];
        content.extend_from_slice(format!("\n{}\n", event(limits())).as_bytes());
        std::fs::write(&path, content).unwrap();
        let input = json!({"transcript_path":path});
        assert!(from_hook(AgentId::Codex, "Stop", &input, home.path(), NOW).is_some());
        assert!(from_hook(AgentId::Codex, "UserPromptSubmit", &input, home.path(), NOW).is_none());
        assert!(from_hook(AgentId::Claude, "Stop", &input, home.path(), NOW).is_none());
        let outside = tempfile::tempdir().unwrap();
        assert!(from_hook(AgentId::Codex, "Stop", &input, outside.path(), NOW).is_none());
        let directory = json!({"transcript_path":home.path()});
        assert!(from_hook(AgentId::Codex, "Stop", &directory, home.path(), NOW).is_none());
    }
}
