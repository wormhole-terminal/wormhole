//! The Usage tab's reports: from a status line fixture and the hooks, kept per agent and across restarts.

use std::fs;
use std::sync::Arc;

use super::write;
use crate::files_panel::listener::testing::Counting;
use crate::files_panel::{PlanLimitRecord, UsageReportRecord, UsageService};

const NOW: i64 = 1_800_000_000;
const STATUS_LINE: &str = include_str!("../../../tests/fixtures/claude-status-line.json");

fn service(dir: &std::path::Path) -> (Arc<UsageService>, Arc<Counting>) {
    let counting = Counting::new();
    (UsageService::in_dir(dir, counting.clone()), counting)
}

#[test]
fn claude_codes_status_line_becomes_the_plan_limits_and_is_kept_across_restarts() {
    let dir = tempfile::tempdir().expect("data dir");
    let (usage, woken) = service(dir.path());
    assert!(usage.reports().is_empty());
    assert!(usage.record_status_line(STATUS_LINE.into(), NOW));
    assert_eq!(woken.count(), 1);
    assert_eq!(
        usage.reports(),
        vec![UsageReportRecord {
            agent: "claude".into(),
            name: "Claude".into(),
            monogram: "CC".into(),
            logo: usage.reports()[0].logo.clone(),
            limits: vec![
                PlanLimitRecord { label: "5-hour limit".into(), used: 25.0, resets_at: Some(1_800_002_040) },
                PlanLimitRecord {
                    label: "Weekly \u{b7} all models".into(),
                    used: 86.0,
                    resets_at: Some(1_800_176_400)
                },
            ],
            response_tokens: None,
            updated: NOW,
        }]
    );
    let (again, _) = service(dir.path());
    assert_eq!(again.reports(), usage.reports(), "the tab is not empty after a restart");
}

#[test]
fn the_latest_report_of_an_agent_wins_and_wakes_once_until_drained() {
    let dir = tempfile::tempdir().expect("data dir");
    let (usage, woken) = service(dir.path());
    usage.record_status_line(STATUS_LINE.into(), NOW);
    usage.record_status_line(STATUS_LINE.replace("25.0", "30.0"), NOW + 60);
    usage.record_status_line(STATUS_LINE.replace("25.0", "5.0"), NOW - 600);
    let reports = usage.reports();
    assert_eq!(reports.len(), 1, "one report per agent");
    assert_eq!(
        (reports[0].limits[0].used, reports[0].updated),
        (30.0, NOW + 60),
        "an older one does not replace it"
    );
    assert_eq!(woken.count(), 1, "three reports, one wake");
    usage.drain();
    usage.record_status_line(STATUS_LINE.into(), NOW + 120);
    assert_eq!(woken.count(), 2);
}

#[test]
fn what_is_not_a_report_is_not_kept() {
    let dir = tempfile::tempdir().expect("data dir");
    let (usage, woken) = service(dir.path());
    assert!(!usage.record_status_line("not json".into(), NOW));
    assert!(!usage.record_status_line(r#"{"model": "x"}"#.into(), NOW), "an API key has no limits");
    assert!(!usage.record_report("{}".into()));
    assert!(!usage.record_hook("nobody".into(), "Stop".into(), "{}".into(), "/".into(), NOW));
    assert!(usage.reports().is_empty());
    assert_eq!(woken.count(), 0);
}

#[test]
fn a_report_from_the_hook_socket_and_a_codex_transcript_under_its_home_are_kept() {
    let dir = tempfile::tempdir().expect("data dir");
    let (usage, _) = service(dir.path());
    let socket = r#"{"agent": "gemini", "five_hour": null, "seven_day": null, "response_tokens": 18412, "updated": 1800000000}"#;
    assert!(usage.record_report(socket.into()));
    assert_eq!(usage.reports()[0].response_tokens, Some(18_412));

    let home = tempfile::tempdir().expect("agent home");
    let line = r#"{"type":"event_msg","timestamp":"2027-01-15T08:00:00Z","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":8.0,"window_minutes":300,"resets_in_seconds":600}}}}"#;
    write(&home.path().join("sessions/rollout.jsonl"), &format!("{line}\n"));
    let input =
        format!(r#"{{"transcript_path": "{}"}}"#, home.path().join("sessions/rollout.jsonl").display());
    let home_path = home.path().canonicalize().expect("canonical").to_string_lossy().into_owned();
    let input = input.replace(&home.path().to_string_lossy().into_owned(), &home_path);
    assert!(
        usage.record_hook("codex".into(), "Stop".into(), input, home_path.clone(), NOW),
        "under its home"
    );
    assert!(usage.reports().iter().any(|r| r.agent == "codex"));

    // A transcript path outside the agent's home is never read.
    let elsewhere = tempfile::tempdir().expect("elsewhere");
    fs::write(elsewhere.path().join("rollout.jsonl"), format!("{line}\n")).expect("write");
    let input = format!(r#"{{"transcript_path": "{}"}}"#, elsewhere.path().join("rollout.jsonl").display());
    assert!(!usage.record_hook("codex".into(), "Stop".into(), input, home_path, NOW + 1));
}
