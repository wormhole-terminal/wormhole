//! Coding agents' plan usage limits (the right panel's Usage tab): how much of a subscription's
//! limit windows is used and when each resets, as the agent reports it. Gemini reports the
//! latest model-response token count instead. Codex and Gemini arrive through `usage::from_hook`.
//!
//! Claude Code hands its status line command `rate_limits` (since 2.1.80, Pro and Max plans,
//! after the first reply of a session); Wormhole's status line entry forwards that JSON here
//! (`ipc::run_status_line_cli`). It is the account's, not a terminal's: the latest report wins.
//! The same JSON also says which model runs and how full its context window is, which is the
//! terminal's own (`Gauge`, for the title row). Nothing here reads a clock; callers pass `now`
//! in Unix seconds.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::agents::AgentId;

mod providers;
pub use providers::from_hook;

/// One rate-limit window: how much of it is used, and when it starts over.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Window {
    /// 0 to 100.
    pub used: f32,
    /// Unix seconds.
    pub resets_at: i64,
}

/// What an agent last reported about its plan's limits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub agent: AgentId,
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    /// Provider-defined windows, where the duration can differ from Claude's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<PlanLimit>,
    /// Gemini reports tokens for the latest model response, not account quota.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_tokens: Option<u64>,
    /// When the report came, Unix seconds.
    pub updated: i64,
}

/// A limit whose label and reset time come from the provider's actual window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanLimit {
    pub label: String,
    pub used: f32,
    pub resets_at: Option<i64>,
}

impl PlanLimit {
    /// Expiry does not prove there has been no usage since the last snapshot.
    pub fn shown(&self, now: i64) -> (f32, String) {
        if self.resets_at.is_some_and(|reset| reset <= now) {
            return (0.0, "Awaiting update".into());
        }
        let used = self.used.clamp(0.0, 100.0);
        let mut words = format!("{}%", used.round() as u32);
        if let Some(reset) = self.resets_at {
            words.push_str(&format!(" · resets {}", duration(reset.saturating_sub(now))));
        }
        (used / 100.0, words)
    }
}

/// The windows a panel shows, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    FiveHour,
    Weekly,
}

impl Limit {
    pub const ALL: [Limit; 2] = [Limit::FiveHour, Limit::Weekly];

    /// The row's name, as Claude's own usage menu words it.
    pub fn label(self) -> &'static str {
        match self {
            Limit::FiveHour => "5-hour limit",
            Limit::Weekly => "Weekly \u{b7} all models",
        }
    }
}

impl Usage {
    /// Claude Code's status line input, if it carries any limit (it doesn't for an API key, or
    /// before the session's first reply).
    pub fn from_claude_status(input: &Value, now: i64) -> Option<Usage> {
        let limits = input.get("rate_limits")?;
        let window = |key: &str| {
            let window = limits.get(key)?;
            let used = window.get("used_percentage")?.as_f64()?;
            let resets_at = window.get("resets_at")?.as_i64()?;
            Some(Window { used: used.clamp(0.0, 100.0) as f32, resets_at })
        };
        let usage = Usage {
            agent: AgentId::Claude,
            five_hour: window("five_hour"),
            seven_day: window("seven_day"),
            limits: Vec::new(),
            response_tokens: None,
            updated: now,
        };
        (usage.five_hour.is_some() || usage.seven_day.is_some()).then_some(usage)
    }

    /// Only reported limits get a meter; missing data is not an unused allowance.
    pub fn plan_limits(&self) -> Vec<PlanLimit> {
        let mut limits: Vec<_> = Limit::ALL
            .into_iter()
            .filter_map(|limit| {
                let window = self.window(limit)?;
                Some(PlanLimit {
                    label: limit.label().into(),
                    used: window.used,
                    resets_at: Some(window.resets_at),
                })
            })
            .collect();
        limits.extend(self.limits.iter().cloned());
        limits
    }

    pub fn window(&self, limit: Limit) -> Option<Window> {
        match limit {
            Limit::FiveHour => self.five_hour,
            Limit::Weekly => self.seven_day,
        }
    }

    /// A window as the panel draws it at `now`: the share used (0 to 1) and its words, "25% ·
    /// resets 34m". A window that has reset since, or that the agent left out (Claude drops one
    /// once it resets), starts over at 0%.
    pub fn shown(&self, limit: Limit, now: i64) -> (f32, String) {
        match self.window(limit).filter(|w| w.resets_at > now) {
            Some(w) => (
                w.used / 100.0,
                format!("{}% \u{b7} resets {}", w.used.round() as u32, duration(w.resets_at - now)),
            ),
            None => (0.0, "0%".to_string()),
        }
    }
}

/// What an agent's status line says about one terminal's session: the model it runs and how full
/// its context window is. Unlike the plan limits this is the terminal's own (it travels with the
/// terminal's id), and only an agent that reports it has one: nothing is guessed for the others.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gauge {
    pub agent: AgentId,
    /// The model as the agent names it ("Opus 4.5").
    pub model: Option<String>,
    /// The share of the context window used, 0 to 100.
    pub context: Option<f32>,
}

/// From this share used on, the meter turns to the warning colour: early enough to compact.
pub const CONTEXT_WARN_AT: f32 = 80.0;

/// The longest model name kept; a status line is not trusted to be short.
const MODEL_MAX: usize = 40;

impl Gauge {
    /// Claude Code's status line input: `model.display_name` (or `model.id`) and
    /// `context_window.used_percentage` (worked out from the last call's tokens when an older
    /// version only gives those). Nothing in it, no gauge.
    pub fn from_claude_status(input: &Value) -> Option<Gauge> {
        let model = match input.get("model") {
            Some(Value::String(name)) => Some(name.as_str()),
            Some(model) => ["display_name", "id"].iter().find_map(|key| model.get(key)?.as_str()),
            None => None,
        }
        .map(clean_model)
        .filter(|name| !name.is_empty());
        let context = input.get("context_window").and_then(context_used).map(|used| used.clamp(0.0, 100.0));
        (model.is_some() || context.is_some()).then_some(Gauge { agent: AgentId::Claude, model, context })
    }

    /// The share used in whole percent, as the meter says it.
    pub fn percent(&self) -> Option<u32> {
        self.context.map(|used| used.round() as u32)
    }

    /// Whether the window is nearly full.
    pub fn warns(&self) -> bool {
        self.context.is_some_and(|used| used >= CONTEXT_WARN_AT)
    }

    /// Both parts in words, for the tooltip and VoiceOver: "Opus 4.5, 41% of the context window used".
    pub fn words(&self) -> String {
        let used = self.percent().map(|percent| format!("{percent}% of the context window used"));
        match (&self.model, used) {
            (Some(model), Some(used)) => format!("{model}, {used}"),
            (Some(model), None) => model.clone(),
            (None, Some(used)) => used,
            (None, None) => String::new(),
        }
    }
}

/// A model name as the row shows it: one line, without the "Claude " every name of Anthropic's
/// could start with, cut where it is unreasonably long.
fn clean_model(name: &str) -> String {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name = name.strip_prefix("Claude ").unwrap_or(&name);
    name.chars().take(MODEL_MAX).collect()
}

/// `used_percentage`, else the tokens of the last call over the window's size.
fn context_used(window: &Value) -> Option<f32> {
    if let Some(used) = window.get("used_percentage").and_then(Value::as_f64) {
        return Some(used as f32);
    }
    let size = window.get("context_window_size")?.as_f64().filter(|size| *size > 0.0)?;
    let current = window.get("current_usage")?;
    let counts: Vec<f64> = ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens"]
        .iter()
        .filter_map(|key| current.get(key)?.as_f64())
        .collect();
    // Nothing counted is not the same as an empty window.
    (!counts.is_empty()).then(|| (counts.iter().sum::<f64>() / size * 100.0) as f32)
}

/// The reports kept, one per agent, in the order the agents first reported.
pub fn record(all: &mut Vec<Usage>, usage: Usage) {
    match all.iter_mut().find(|u| u.agent == usage.agent) {
        Some(slot) if usage.updated >= slot.updated => *slot = usage,
        Some(_) => {}
        None => all.push(usage),
    }
}

/// A time to come, as short as the usage menu has it: "45s", "34m", "2h 5m", "6d".
pub fn duration(secs: i64) -> String {
    let secs = secs.max(0);
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86_400 => match (secs % 3600) / 60 {
            0 => format!("{}h", secs / 3600),
            m => format!("{}h {m}m", secs / 3600),
        },
        // To the nearest day: three days less a minute still reads "3d".
        _ => format!("{}d", secs.saturating_add(43_200) / 86_400),
    }
}

/// When the numbers came: "Updated just now", "Updated 3m ago".
pub fn updated(updated: i64, now: i64) -> String {
    match now - updated {
        ..60 => "Updated just now".to_string(),
        ago => format!("Updated {} ago", duration(ago)),
    }
}

/// The store's key for the reports, kept so the tab isn't empty after a restart.
pub const STORE_KEY: &str = "usage";

pub fn to_json(all: &[Usage]) -> String {
    serde_json::to_string(all).unwrap_or_default()
}

/// Saved reports; anything unreadable is dropped (the next report replaces it anyway).
pub fn from_json(json: &str) -> Vec<Usage> {
    serde_json::from_str(json).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AgentId, Gauge, Limit, Usage, Window, duration, from_json, record, to_json, updated};

    const NOW: i64 = 1_800_000_000;

    fn status() -> serde_json::Value {
        json!({
            "session_id": "abc",
            "rate_limits": {
                "five_hour": { "used_percentage": 25.4, "resets_at": NOW + 34 * 60 },
                "seven_day": { "used_percentage": 0, "resets_at": NOW + 6 * 86_400 + 100 }
            }
        })
    }

    #[test]
    fn claudes_status_line_input_gives_both_windows() {
        let usage = Usage::from_claude_status(&status(), NOW).expect("limits");
        assert_eq!(usage.agent, AgentId::Claude);
        assert_eq!(usage.five_hour, Some(Window { used: 25.4, resets_at: NOW + 34 * 60 }));
        assert_eq!(usage.shown(Limit::FiveHour, NOW), (0.254, "25% \u{b7} resets 34m".to_string()));
        assert_eq!(usage.shown(Limit::Weekly, NOW).1, "0% \u{b7} resets 6d");
    }

    #[test]
    fn no_limits_no_report() {
        assert_eq!(Usage::from_claude_status(&json!({ "session_id": "abc" }), NOW), None);
        assert_eq!(Usage::from_claude_status(&json!({ "rate_limits": {} }), NOW), None);
        // One window alone is a report: Claude drops the other once it resets.
        let one = json!({ "rate_limits": { "seven_day": { "used_percentage": 50, "resets_at": NOW + 60 } } });
        let usage = Usage::from_claude_status(&one, NOW).expect("a report");
        assert_eq!(usage.shown(Limit::FiveHour, NOW), (0.0, "0%".to_string()));
    }

    #[test]
    fn a_window_that_reset_starts_over() {
        let usage = Usage::from_claude_status(&status(), NOW).expect("limits");
        assert_eq!(usage.shown(Limit::FiveHour, NOW + 3600), (0.0, "0%".to_string()));
    }

    #[test]
    fn out_of_range_percentages_are_clamped() {
        let over =
            json!({ "rate_limits": { "five_hour": { "used_percentage": 140, "resets_at": NOW + 60 } } });
        let usage = Usage::from_claude_status(&over, NOW).expect("a report");
        assert_eq!(usage.shown(Limit::FiveHour, NOW).0, 1.0);
    }

    #[test]
    fn the_latest_report_per_agent_wins() {
        let mut all = Vec::new();
        let first = Usage::from_claude_status(&status(), NOW).expect("limits");
        record(&mut all, first.clone());
        record(&mut all, Usage { updated: NOW + 5, ..first });
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].updated, NOW + 5);
        assert_eq!(from_json(&to_json(&all)), all, "kept across a restart");
        assert!(from_json("not json").is_empty());
    }

    #[test]
    fn old_saved_reports_and_new_providers_coexist() {
        let legacy = r#"[{"agent":"claude","five_hour":null,"seven_day":null,"updated":12}]"#;
        let mut all = from_json(legacy);
        assert_eq!(all.len(), 1);
        assert!(all[0].limits.is_empty());
        assert_eq!(all[0].response_tokens, None);
        let gemini = Usage {
            agent: AgentId::Gemini,
            five_hour: None,
            seven_day: None,
            limits: Vec::new(),
            response_tokens: Some(100),
            updated: NOW,
        };
        record(&mut all, gemini.clone());
        record(&mut all, Usage { response_tokens: Some(9), updated: NOW - 1, ..gemini.clone() });
        assert_eq!(all[1].response_tokens, Some(100), "late hooks cannot replace a newer report");
        record(&mut all, Usage { response_tokens: Some(150), updated: NOW + 1, ..gemini });
        assert_eq!(all[1].response_tokens, Some(150), "streaming snapshots are replaced, not summed");
        assert_eq!(from_json(&to_json(&all)), all);
    }

    #[test]
    fn the_panel_omits_unreported_limits_and_marks_expired_snapshots() {
        let report = Usage::from_claude_status(
            &json!({"rate_limits": {
                "five_hour": {"used_percentage": 70, "resets_at": NOW+1}
            }}),
            NOW,
        )
        .unwrap();
        let limits = report.plan_limits();
        assert_eq!(limits.len(), 1);
        assert_eq!(limits[0].shown(NOW + 2).1, "Awaiting update");
        assert!(!duration(i64::MAX).is_empty());
    }

    #[test]
    fn times_read_like_the_usage_menu() {
        assert_eq!(duration(-5), "0s");
        assert_eq!(duration(45), "45s");
        assert_eq!(duration(34 * 60 + 10), "34m");
        assert_eq!(duration(2 * 3600), "2h");
        assert_eq!(duration(2 * 3600 + 5 * 60), "2h 5m");
        assert_eq!(duration(6 * 86_400 + 3600), "6d");
        assert_eq!(duration(3 * 86_400 - 60), "3d");
        assert_eq!(updated(NOW, NOW + 20), "Updated just now");
        assert_eq!(updated(NOW, NOW + 3 * 60), "Updated 3m ago");
    }

    fn gauge(input: serde_json::Value) -> Option<Gauge> {
        Gauge::from_claude_status(&input)
    }

    #[test]
    fn claudes_status_line_gives_the_model_and_the_context_used() {
        let got = gauge(json!({
            "model": { "id": "claude-opus-4-5", "display_name": "Opus 4.5" },
            "context_window": { "used_percentage": 41.4, "context_window_size": 200_000 }
        }))
        .expect("a gauge");
        assert_eq!(got.agent, AgentId::Claude);
        assert_eq!((got.model.as_deref(), got.percent(), got.warns()), (Some("Opus 4.5"), Some(41), false));
        assert_eq!(got.words(), "Opus 4.5, 41% of the context window used");
    }

    #[test]
    fn the_meter_warns_from_80_percent() {
        let at = |used: f64| gauge(json!({ "context_window": { "used_percentage": used } })).expect("gauge");
        assert!(!at(79.4).warns() && at(80.0).warns() && at(100.0).warns());
        assert_eq!(at(140.0).percent(), Some(100), "clamped");
        assert_eq!(at(-3.0).percent(), Some(0));
    }

    /// Parts the agent leaves out are left out, and a status line without either is no gauge.
    #[test]
    fn a_gauge_shows_only_what_the_agent_reported() {
        let model_only = gauge(json!({ "model": { "id": "claude-sonnet-4" } })).expect("gauge");
        assert_eq!((model_only.model.as_deref(), model_only.context), (Some("claude-sonnet-4"), None));
        assert_eq!(model_only.words(), "claude-sonnet-4");
        let context_only = gauge(json!({ "context_window": { "used_percentage": 12 } })).expect("gauge");
        assert_eq!(context_only.model, None);
        assert_eq!(context_only.words(), "12% of the context window used");
        assert_eq!(gauge(json!({ "session_id": "abc" })), None);
        // Before the first reply the percentage is null, and there is nothing to count either.
        let early = json!({ "context_window": { "used_percentage": null, "current_usage": null } });
        assert_eq!(gauge(early), None);
        assert_eq!(gauge(json!({ "model": { "display_name": "  " } })), None);
    }

    #[test]
    fn older_versions_give_tokens_and_a_window_size() {
        let got = gauge(json!({
            "context_window": {
                "context_window_size": 200_000,
                "current_usage": { "input_tokens": 20_000, "cache_read_input_tokens": 30_000, "output_tokens": 9 }
            }
        }))
        .expect("gauge");
        assert_eq!(got.percent(), Some(25));
        let empty = json!({ "context_window": { "context_window_size": 200_000, "current_usage": {} } });
        assert_eq!(gauge(empty), None, "nothing counted is not an empty window");
    }

    #[test]
    fn a_model_name_is_one_short_line() {
        let name = |text: &str| gauge(json!({ "model": text })).and_then(|g| g.model);
        assert_eq!(
            name("Claude Opus 4.5").as_deref(),
            Some("Opus 4.5"),
            "a bare string, as older versions had"
        );
        assert_eq!(name("Opus\n 4.5").as_deref(), Some("Opus 4.5"));
        assert_eq!(name(&"x".repeat(200)).map(|n| n.chars().count()), Some(40));
    }

    #[test]
    fn the_rows_are_named_as_claude_names_them() {
        let labels: Vec<&str> = Limit::ALL.iter().map(|l| l.label()).collect();
        assert_eq!(labels, ["5-hour limit", "Weekly \u{b7} all models"]);
    }
}
