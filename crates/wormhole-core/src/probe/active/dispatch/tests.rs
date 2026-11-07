use super::*;
use crate::probe::active::{Grants, load::ActiveProbe};
use crate::probe::events::Value;
use crate::probe::hooks::CompiledHooks;
use crate::probe::manifest::Manifest;
use crate::probe::rules::CompiledRules;

const HOOKS: &str = r#"
[permissions]
notify = true

[[contributes.status]]
id = "build"

[[contributes.hooks]]
id = "failed"
on = "command.finished"
match = { exit = "!=0" }
do = [
  { type = "notify", title = "Build failed", body = "{took_s} s in {folder_name}" },
  { type = "status", item = "build", text = "failed", tone = "danger" },
  { type = "highlight", tone = "danger" },
]

[[contributes.hooks]]
id = "bell"
on = "bell"
do = [{ type = "status", item = "build", text = "rang" }]

[[contributes.hooks]]
id = "strangers"
on = "bell"
do = [{ type = "status", item = "other", text = "x" }]
"#;

fn probe(extra: &str, grants: Grants) -> ActiveProbe {
    let text = format!(
        "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
             version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n{extra}"
    );
    let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}"));
    let manifest = parsed.manifest;
    ActiveProbe {
        id: "tiny".into(),
        name: "Tiny".into(),
        rules: CompiledRules::compile(&manifest).expect("rules"),
        hooks: CompiledHooks::compile(&manifest).expect("hooks"),
        grants,
        status_items: manifest.contributes.status.iter().map(|s| s.id.clone()).collect(),
    }
}

fn finished(exit: i64, took_ms: i64) -> EventData {
    EventData::new([
        ("terminal".to_string(), Value::Number(3)),
        ("block".to_string(), Value::Number(9)),
        ("exit".to_string(), Value::Number(exit)),
        ("took_ms".to_string(), Value::Number(took_ms)),
        ("folder_name".to_string(), Value::Text("acme".into())),
    ])
}

fn notify() -> Grants {
    Grants { notify: true, ..Grants::default() }
}

#[test]
fn a_failed_command_notifies_sets_the_item_and_highlights_its_block() {
    let mut dispatcher = Dispatcher::new(vec![probe(HOOKS, notify())]);
    let effects = dispatcher.deliver(HookEvent::CommandFinished, &finished(2, 61_000), Instant::now());
    assert_eq!(effects.len(), 3, "{effects:?}");
    assert!(
        matches!(&effects[0], Effect::Notify { probe, probe_name, title, body, terminal: Some(3) }
            if probe == "tiny" && probe_name == "Tiny" && title == "Build failed" && body == "61 s in acme"),
        "{:?}",
        effects[0]
    );
    assert!(matches!(&effects[1], Effect::Status { item, text, tone: Some(Tone::Danger), .. }
            if item == "build" && text == "failed"));
    assert_eq!(
        effects[2],
        Effect::Highlight { probe: "tiny".into(), tone: Tone::Danger, terminal: 3, block: 9 }
    );
    assert!(
        dispatcher.deliver(HookEvent::CommandFinished, &finished(0, 5), Instant::now()).is_empty(),
        "exit 0 does not match"
    );
}

#[test]
fn a_notification_needs_the_grant_and_the_rest_still_acts() {
    let mut dispatcher = Dispatcher::new(vec![probe(HOOKS, Grants::default())]);
    let effects = dispatcher.deliver(HookEvent::CommandFinished, &finished(1, 1), Instant::now());
    assert!(effects.iter().all(|e| !matches!(e, Effect::Notify { .. })), "{effects:?}");
    assert_eq!(effects.len(), 2);
}

#[test]
fn a_status_item_the_probe_did_not_declare_is_not_set() {
    let mut dispatcher = Dispatcher::new(vec![probe(HOOKS, notify())]);
    let effects = dispatcher.deliver(
        HookEvent::Bell,
        &EventData::new([("terminal".to_string(), Value::Number(1))]),
        Instant::now(),
    );
    assert_eq!(effects.len(), 1, "`rang` only: `other` is nobody's item");
    assert!(matches!(&effects[0], Effect::Status { text, .. } if text == "rang"));
}

#[test]
fn a_burst_is_one_run_within_the_cooldown_and_a_minute_has_a_cap() {
    let limits = Limits {
        cooldown: Duration::from_millis(100),
        per_minute: 3,
        notifications_per_minute: 2,
        ..Limits::default()
    };
    let mut dispatcher = Dispatcher::with_limits(vec![probe(HOOKS, notify())], limits);
    let start = Instant::now();
    let bell = EventData::new([("terminal".to_string(), Value::Number(1))]);
    assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, start).len(), 1);
    assert!(dispatcher.deliver(HookEvent::Bell, &bell, start + Duration::from_millis(10)).is_empty());
    assert_eq!(dispatcher.refused(), 2, "counted for each of the two hooks on the bell");
    let later = |ms: u64| start + Duration::from_millis(ms);
    assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, later(200)).len(), 1);
    assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, later(400)).len(), 1);
    assert!(dispatcher.deliver(HookEvent::Bell, &bell, later(600)).is_empty(), "the fourth in a minute");
    assert_eq!(dispatcher.deliver(HookEvent::Bell, &bell, later(61_000)).len(), 1, "a new minute");
    // Notifications have a cap of their own.
    let mut dispatcher = Dispatcher::with_limits(vec![probe(HOOKS, notify())], limits);
    let notices: usize = (0..3u64)
        .map(|n| {
            dispatcher
                .deliver(HookEvent::CommandFinished, &finished(1, 1), start + Duration::from_millis(200 * n))
                .iter()
                .filter(|e| matches!(e, Effect::Notify { .. }))
                .count()
        })
        .sum();
    assert_eq!(notices, 2);
}

#[test]
fn text_fields_reach_a_hook_only_with_the_read_grant() {
    let text_hook = r#"
[[contributes.hooks]]
id = "wherever"
on = "folder.changed"
match = { folder = "/work/*" }
do = [{ type = "highlight", tone = "link" }]
"#;
    let folder = EventData::new([
        ("terminal".to_string(), Value::Number(1)),
        ("block".to_string(), Value::Number(1)),
        ("folder".to_string(), Value::Text("/work/a".into())),
    ]);
    let mut without = Dispatcher::new(vec![probe(text_hook, Grants::default())]);
    assert!(without.deliver(HookEvent::FolderChanged, &folder, Instant::now()).is_empty());
    let read = Grants { terminal_read: true, ..Grants::default() };
    let mut with = Dispatcher::new(vec![probe(text_hook, read)]);
    // The highlight is refused by `validate` on this event, but the dispatcher only acts on what matched.
    assert_eq!(with.deliver(HookEvent::FolderChanged, &folder, Instant::now()).len(), 1);
}

const RUNNER: &str = r#"
[permissions]
process = ["tool"]

[[contributes.status]]
id = "build"

[[contributes.commands]]
id = "c"
title = "C"
run = ["tool", "x"]
in = "background"

[[contributes.hooks]]
id = "r"
on = "bell"
do = [{ type = "run", command = "c", item = "build" }]
"#;

fn bell() -> EventData {
    EventData::new([("terminal".to_string(), Value::Number(1))])
}

fn granted() -> Grants {
    Grants { process: vec!["tool".into()], ..Grants::default() }
}

fn started(effects: &[Effect]) -> Option<RunTicket> {
    effects.iter().find_map(|effect| match effect {
        Effect::Run { ticket, .. } => Some(*ticket),
        _ => None,
    })
}

#[test]
fn a_program_runs_only_if_granted_and_one_at_a_time_per_hook() {
    let start = Instant::now();
    let later = |ms: u64| start + Duration::from_millis(ms);
    let mut ungranted = Dispatcher::new(vec![probe(RUNNER, Grants::default())]);
    assert!(started(&ungranted.deliver(HookEvent::Bell, &bell(), start)).is_none(), "not in `process`");

    let mut dispatcher = Dispatcher::new(vec![probe(RUNNER, granted())]);
    let effects = dispatcher.deliver(HookEvent::Bell, &bell(), start);
    let ticket = started(&effects).expect("a run");
    assert!(matches!(&effects[0], Effect::Run { argv, terminal: Some(1), .. } if argv == &["tool", "x"]));
    assert!(started(&dispatcher.deliver(HookEvent::Bell, &bell(), later(500))).is_none(), "still going");
    assert_eq!(dispatcher.refused(), 1);

    let done = dispatcher.finished(ticket, &RunEnd::Exited { code: 0, stdout: "ok\n".into() }, later(600));
    assert!(matches!(&done[..], [Effect::Status { item, text, .. }] if item == "build" && text == "ok"));
    assert!(started(&dispatcher.deliver(HookEvent::Bell, &bell(), later(700))).is_some(), "free again");
}

#[test]
fn a_hook_that_fails_three_times_in_a_minute_is_paused() {
    let start = Instant::now();
    let mut dispatcher = Dispatcher::new(vec![probe(RUNNER, granted())]);
    let mut paused = Vec::new();
    for n in 0..3u64 {
        let at = start + Duration::from_secs(n * 5);
        let ticket = started(&dispatcher.deliver(HookEvent::Bell, &bell(), at)).expect("a run");
        let end = if n == 0 { RunEnd::TimedOut } else { RunEnd::Exited { code: 2, stdout: String::new() } };
        paused = dispatcher.finished(ticket, &end, at + Duration::from_secs(1));
    }
    assert!(paused.iter().any(|e| matches!(e, Effect::Paused { hook, .. } if hook == "r")), "{paused:?}");
    let after = start + Duration::from_secs(30);
    assert!(dispatcher.deliver(HookEvent::Bell, &bell(), after).is_empty(), "paused");
}

#[test]
fn only_so_many_programs_run_at_once() {
    let limits = Limits { runs_at_once: 1, ..Limits::default() };
    let two = format!(
        "{RUNNER}\n[[contributes.hooks]]\nid = \"s\"\non = \"bell\"\ndo = [{{ type = \"run\", command = \"c\" }}]\n"
    );
    let mut dispatcher = Dispatcher::with_limits(vec![probe(&two, granted())], limits);
    let effects = dispatcher.deliver(HookEvent::Bell, &bell(), Instant::now());
    assert_eq!(effects.iter().filter(|e| matches!(e, Effect::Run { .. })).count(), 1);
}

#[test]
fn a_path_is_offered_to_its_terminal_and_never_opened_here() {
    let text = r#"
[[contributes.hooks]]
id = "p"
on = "output.match"
match = { pattern = 'at (\S+) line ([0-9]+)' }
do = [{ type = "open-path", path = "{1}", line = "{2}" }]
"#;
    let read = Grants { terminal_read: true, ..Grants::default() };
    let mut dispatcher = Dispatcher::new(vec![probe(text, read)]);
    assert_eq!(dispatcher.events(), [HookEvent::OutputMatch]);
    let line = EventData::new([
        ("terminal".to_string(), Value::Number(2)),
        ("line".to_string(), Value::Text("at src/a.rs line 7".into())),
    ]);
    let effects = dispatcher.deliver(HookEvent::OutputMatch, &line, Instant::now());
    assert!(
        matches!(&effects[..], [Effect::OpenPath { terminal: 2, path, line: Some(7), .. }] if path == "src/a.rs")
    );
    let blind = Dispatcher::new(vec![probe(text, Grants::default())]);
    assert!(blind.events().is_empty(), "without the read grant nothing is heard for it");
}
