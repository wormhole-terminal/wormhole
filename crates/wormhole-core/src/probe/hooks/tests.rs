//! Hooks planned over events, from small in-memory manifests. These plan hooks that `validate` may
//! have refused, because the fixed rules of a `run` must hold for a Probe that skipped it too.

use super::*;
use crate::probe::events::Value;
use crate::probe::manifest::{Cwd, FoldTarget, Manifest};

const HEAD: &str = "[probe]\nid = \"tiny\"\nname = \"T\"\ndescription = \"d\"\nauthor = \"a\"\n\
                    version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n";

fn hooks(rest: &str) -> CompiledHooks {
    let parsed = Manifest::parse(&format!("{HEAD}{rest}")).unwrap_or_else(|err| panic!("{err}"));
    CompiledHooks::compile(&parsed.manifest).unwrap_or_else(|err| panic!("{err}"))
}

fn event(fields: &[(&str, Value)]) -> EventData {
    EventData::new(fields.iter().map(|(name, value)| (name.to_string(), value.clone())))
}

fn n(value: i64) -> Value {
    Value::Number(value)
}

fn t(value: &str) -> Value {
    Value::Text(value.into())
}

fn actions(plan: Plan) -> Vec<Planned> {
    match plan {
        Plan::Actions(actions) => actions,
        Plan::Skipped(why) => panic!("skipped: {why}"),
    }
}

fn skipped(plan: Plan) -> String {
    match plan {
        Plan::Skipped(why) => why,
        Plan::Actions(actions) => panic!("fired: {actions:?}"),
    }
}

// ---- matching -------------------------------------------------------------------------------

const SLOW: &str = r#"
[[contributes.hooks]]
id = "slow"
on = "command.finished"
match = { exit = "!=0", took_ms = ">=60000" }
do = [{ type = "highlight", tone = "danger" }]
"#;

#[test]
fn every_field_of_a_match_must_hold_and_the_first_that_does_not_is_named() {
    let slow = hooks(SLOW);
    let hook = slow.get("slow").expect("hook");
    assert_eq!(actions(hook.plan(&event(&[("exit", n(2)), ("took_ms", n(61000))]))).len(), 1);
    assert_eq!(
        skipped(hook.plan(&event(&[("exit", n(0)), ("took_ms", n(61000))]))),
        "`exit != 0` did not hold (`exit` was 0)"
    );
    assert_eq!(
        skipped(hook.plan(&event(&[("exit", n(2)), ("took_ms", n(10))]))),
        "`took_ms >= 60000` did not hold (`took_ms` was 10)"
    );
    // A field the event does not have cannot satisfy a test.
    assert_eq!(
        skipped(hook.plan(&event(&[("took_ms", n(61000))]))),
        "`exit != 0` did not hold (the event has no `exit`)"
    );
}

#[test]
fn states_names_globs_and_facts_match_what_they_say() {
    let all = hooks(
        r#"
[[contributes.hooks]]
id = "agent"
on = "agent.status"
match = { status = "needs_you,failed", agent = "claude,codex", kind = "agent", when = "front" }
do = [{ type = "highlight", tone = "danger" }]
[[contributes.hooks]]
id = "branch"
on = "git.changed"
match = { branch = "main,release/*" }
do = [{ type = "highlight", tone = "danger" }]
"#,
    );
    let agent = all.get("agent").expect("hook");
    let base = |status: &str, agent: &str, kind: &str, front: i64| {
        event(&[("status", t(status)), ("agent", t(agent)), ("kind", t(kind)), ("front", n(front))])
    };
    assert_eq!(actions(agent.plan(&base("needs_you", "claude", "agent", 1))).len(), 1);
    assert_eq!(actions(agent.plan(&base("failed", "codex", "agent", 1))).len(), 1);
    assert!(
        skipped(agent.plan(&base("working", "claude", "agent", 1))).starts_with("`status needs_you,failed`")
    );
    assert!(skipped(agent.plan(&base("needs_you", "aider", "agent", 1))).starts_with("`agent claude,codex`"));
    assert!(skipped(agent.plan(&base("needs_you", "claude", "shell", 1))).starts_with("`kind agent`"));
    assert!(skipped(agent.plan(&base("needs_you", "claude", "agent", 0))).starts_with("`when front`"));
    let branch = all.get("branch").expect("hook");
    for (name, fires) in
        [("main", true), ("release/1.2", true), ("mainline", false), ("feature/release/x", false)]
    {
        let plan = branch.plan(&event(&[("branch", t(name))]));
        assert_eq!(matches!(plan, Plan::Actions(_)), fires, "{name}");
    }
}

#[test]
fn a_pattern_runs_over_the_text_field_and_gives_its_groups_as_captures() {
    let compiled = hooks(
        r#"
[[contributes.hooks]]
id = "t"
on = "title"
match = { pattern = '(build) ([0-9]+)' }
do = [{ type = "status", item = "s", text = "{2}:{1}:{0}" }]
"#,
    );
    let hook = compiled.get("t").expect("hook");
    let plan = actions(hook.plan(&event(&[("title", t("ci build 17 ok"))])));
    assert!(matches!(&plan[0], Planned::Status { text, .. } if text == "17:build:build 17"), "{plan:?}");
    assert!(
        skipped(hook.plan(&event(&[("title", t("nothing"))]))).starts_with("`pattern /(build) ([0-9]+)/`")
    );
}

#[test]
fn a_pattern_sees_only_the_first_two_kib_of_a_line() {
    let compiled = hooks(
        "[[contributes.hooks]]\nid = \"t\"\non = \"output.match\"\nmatch = { pattern = 'NEEDLE' }\ndo = [{ type = \"highlight\", tone = \"danger\" }]\n",
    );
    let hook = compiled.get("t").expect("hook");
    let far = format!("{}NEEDLE", "x".repeat(crate::probe::rules::MAX_SCAN_BYTES));
    assert!(matches!(hook.plan(&event(&[("line", t(&far))])), Plan::Skipped(_)));
    assert!(matches!(hook.plan(&event(&[("line", t("a NEEDLE"))])), Plan::Actions(_)));
}

#[test]
fn a_hook_that_does_not_compile_names_itself() {
    let parsed = Manifest::parse(&format!(
        "{HEAD}[[contributes.hooks]]\nid = \"a\"\non = \"bell\"\ndo = [{{ type = \"highlight\", tone = \"danger\" }}]\n\
         [[contributes.hooks]]\nid = \"b\"\non = \"command.finished\"\nmatch = {{ exit = \"x\" }}\ndo = [{{ type = \"highlight\", tone = \"danger\" }}]\n"
    ))
    .expect("parses");
    let err = CompiledHooks::compile(&parsed.manifest).expect_err("b does not");
    assert_eq!(err.index, 1);
    assert!(err.to_string().starts_with("contributes.hooks[1]: `match.exit`"), "{err}");
}

// ---- actions ----------------------------------------------------------------------------------------

#[test]
fn text_of_an_event_is_cleaned_and_cut_before_it_reaches_a_notification() {
    let compiled = hooks(
        r#"
[[contributes.hooks]]
id = "n"
on = "folder.changed"
do = [{ type = "notify", title = "Now in {folder}", body = "{folder}" }]
"#,
    );
    let folder = format!("/tmp/\u{1b}]0;pwned\u{7}\u{202e}/{}\nsecond", "d".repeat(300));
    let plan = actions(compiled.get("n").expect("hook").plan(&event(&[("folder", t(&folder))])));
    let Planned::Notify { title, body } = &plan[0] else { panic!("{plan:?}") };
    for text in [title, body] {
        assert!(!text.chars().any(crate::probe::hidden::is_hidden), "{text:?}");
    }
    assert_eq!(title.chars().count(), 80);
    assert_eq!(body.chars().count(), 240);
    assert!(title.starts_with("Now in /tmp/]0;pwned/") && title.ends_with('…'));
}

#[test]
fn the_seconds_a_command_took_are_worked_out_for_a_template() {
    let compiled = hooks(
        "[[contributes.hooks]]\nid = \"n\"\non = \"command.finished\"\ndo = [{ type = \"notify\", title = \"{took_s} s\" }]\n",
    );
    for (ms, title) in [(61000, "61 s"), (3500, "3.5 s")] {
        let plan = actions(compiled.get("n").expect("hook").plan(&event(&[("took_ms", n(ms))])));
        assert_eq!(plan, [Planned::Notify { title: title.into(), body: String::new() }]);
    }
}

fn fold(action: &str) -> CompiledHooks {
    hooks(&format!(
        "[[contributes.hooks]]\nid = \"f\"\non = \"command.finished\"\ndo = [{{ type = \"fold\", target = \"block-output\", label = \"out\"{action} }}]\n"
    ))
}

fn folded(compiled: &CompiledHooks, lines: i64) -> Planned {
    actions(compiled.get("f").expect("hook").plan(&event(&[("output_lines", n(lines))]))).remove(0)
}

#[test]
fn a_fold_hides_the_lines_of_a_block_less_what_it_keeps() {
    let compiled = fold(", keep_head = 1, keep_tail = 3, min_lines = 10");
    assert_eq!(
        folded(&compiled, 120),
        Planned::Fold { target: FoldTarget::BlockOutput, hidden_rows: Some(116), label: "out".into() }
    );
    // Exactly the fewest rows a fold may hide: three beyond head and tail.
    assert!(matches!(folded(&compiled, 10), Planned::Fold { hidden_rows: Some(6), .. }));
    let tight = fold(", keep_head = 1, keep_tail = 3");
    assert!(matches!(folded(&tight, 7), Planned::Fold { hidden_rows: Some(3), .. }));
    assert!(
        matches!(folded(&tight, 6), Planned::Dropped { action: "fold", .. }),
        "two rows are not worth a marker"
    );
}

#[test]
fn a_fold_leaves_a_short_block_and_a_huge_one_alone() {
    let compiled = fold(", min_lines = 50");
    assert!(matches!(folded(&compiled, 49), Planned::Dropped { action: "fold", .. }));
    assert!(matches!(folded(&compiled, 50), Planned::Fold { hidden_rows: Some(50), .. }));
    assert!(matches!(folded(&compiled, 100_000), Planned::Fold { .. }));
    assert!(matches!(folded(&compiled, 100_001), Planned::Dropped { action: "fold", .. }));
    let none = actions(compiled.get("f").expect("hook").plan(&event(&[])));
    assert!(matches!(&none[0], Planned::Dropped { why, .. } if why.contains("output_lines")), "{none:?}");
}

#[test]
fn a_fold_of_a_run_of_matches_leaves_the_count_to_the_terminal() {
    let compiled = hooks(
        "[[contributes.hooks]]\nid = \"m\"\non = \"output.match\"\nmatch = { pattern = 'x' }\ndo = [{ type = \"fold\", target = \"match-run\", label = \"noise\" }]\n",
    );
    let plan = actions(compiled.get("m").expect("hook").plan(&event(&[("line", t("x"))])));
    assert_eq!(
        plan,
        [Planned::Fold { target: FoldTarget::MatchRun, hidden_rows: None, label: "noise".into() }]
    );
}

fn runner(command: &str, pattern: &str) -> CompiledHooks {
    hooks(&format!(
        "[permissions]\nprocess = [\"git\", \"say\"]\n\
         [[contributes.commands]]\nid = \"c\"\ntitle = \"C\"\nrun = {command}\nin = \"background\"\n\
         [[contributes.hooks]]\nid = \"r\"\non = \"output.match\"\nmatch = {{ pattern = '{pattern}' }}\n\
         do = [{{ type = \"run\", command = \"c\" }}]\n"
    ))
}

fn ran(compiled: &CompiledHooks, line: &str) -> Planned {
    actions(compiled.get("r").expect("hook").plan(&event(&[("line", t(line))]))).remove(0)
}

#[test]
fn a_capture_is_a_whole_argument_and_git_gets_its_marker_before_it() {
    let git = runner("[\"git\", \"show\", \"--stat\", \"$1\"]", "FAILED (.+)");
    assert_eq!(
        ran(&git, "FAILED abc1234"),
        Planned::Run {
            command: "c".into(),
            argv: ["git", "show", "--stat", "--end-of-options", "abc1234"].map(String::from).to_vec(),
            cwd: Cwd::Terminal,
            timeout_s: 10,
            item: None
        }
    );
    // A capture is never split, spliced or interpreted: spaces and metacharacters stay in one argument.
    let Planned::Run { argv, .. } = ran(&git, "FAILED a b;c $(x) `y`") else { panic!() };
    assert_eq!(argv.last().map(String::as_str), Some("a b;c $(x) `y`"));
    assert_eq!(argv.len(), 5);
    let say = runner("[\"say\", \"$1\"]", "FAILED (.+)");
    let Planned::Run { argv, .. } = ran(&say, "FAILED done") else { panic!() };
    assert_eq!(argv, ["say", "done"], "the marker is git's: other programs rely on the dash refusal");
}

#[test]
fn a_capture_a_program_could_read_as_an_option_is_refused() {
    // `validate` refuses a pattern that can produce these; a Probe that skipped it must not be trusted.
    let git = runner("[\"git\", \"show\", \"$1\"]", "FAILED (\\S*)");
    for line in ["FAILED --output=/tmp/x", "FAILED -p", "FAILED ", &format!("FAILED {}", "a".repeat(1025))] {
        assert!(matches!(ran(&git, line), Planned::Dropped { action: "run", .. }), "{line:.30}");
    }
    let nth = runner("[\"git\", \"show\", \"$2\"]", "FAILED (.+)");
    assert!(matches!(ran(&nth, "FAILED x"), Planned::Dropped { action: "run", .. }), "no capture 2");
    let whole = runner("[\"git\", \"show\", \"$0\"]", "FAILED .+");
    assert!(matches!(ran(&whole, "FAILED x"), Planned::Run { .. }));
}

#[test]
fn a_timeout_defaults_to_ten_seconds() {
    let compiled = hooks(
        "[permissions]\nprocess = [\"say\"]\n[[contributes.commands]]\nid = \"c\"\ntitle = \"C\"\nrun = [\"say\", \"x\"]\nin = \"background\"\n\
         [[contributes.hooks]]\nid = \"r\"\non = \"bell\"\ndo = [{ type = \"run\", command = \"c\", timeout = 60, item = \"s\" }, { type = \"run\", command = \"c\" }]\n",
    );
    let plan = actions(compiled.get("r").expect("hook").plan(&event(&[])));
    assert!(matches!(&plan[0], Planned::Run { timeout_s: 60, item: Some(item), .. } if item == "s"));
    assert!(matches!(&plan[1], Planned::Run { timeout_s: 10, item: None, .. }));
}

#[test]
fn a_path_that_walks_up_is_not_offered() {
    let compiled = hooks(
        "[[contributes.hooks]]\nid = \"p\"\non = \"output.match\"\nmatch = { pattern = 'at (\\S+) line ([0-9]+)' }\ndo = [{ type = \"open-path\", path = \"{1}\", line = \"{2}\" }]\n",
    );
    let plan =
        |line: &str| actions(compiled.get("p").expect("hook").plan(&event(&[("line", t(line))]))).remove(0);
    assert_eq!(plan("at src/a.rs line 7"), Planned::OpenPath { path: "src/a.rs".into(), line: Some(7) });
    for bad in [
        "at ../../etc/passwd line 1",
        "at src/../../x line 1",
        "at a.rs line 0",
        "at https://evil.dev/x line 1",
    ] {
        assert!(matches!(plan(bad), Planned::Dropped { action: "open-path", .. }), "{bad}");
    }
}

// ---- what a run printed ---------------------------------------------------------------------------------

#[test]
fn a_runs_output_feeds_its_item_and_a_failure_shows_on_it() {
    let compiled = hooks("[permissions]\nnotify = true\n");
    let (planned, log) = compiled.run_effects(Some("build"), 0, "3 failing\n");
    assert_eq!(
        planned,
        [Planned::Status {
            item: "build".into(),
            text: "3 failing".into(),
            tone: None,
            icon: None,
            tooltip: None
        }]
    );
    assert_eq!(log, Vec::<String>::new());
    let (planned, log) = compiled.run_effects(Some("build"), 2, "{\"status\":{\"text\":\"x\"}}");
    assert!(
        matches!(&planned[..], [Planned::Status { text, tone: Some(tone), .. }] if text == "failed (exit 2)" && tone == "muted"),
        "{planned:?}"
    );
    assert_eq!(log.len(), 1);
    let (planned, _) = compiled.run_effects(None, 1, "");
    assert_eq!(planned, [], "no item, nothing to show it on");
}

#[test]
fn a_runs_notification_needs_the_probes_notify() {
    let json = r#"{"notify":{"title":"Tests","body":"2 failed"}}"#;
    let (planned, log) = hooks("[permissions]\nnotify = true\n").run_effects(None, 0, json);
    assert_eq!(planned, [Planned::Notify { title: "Tests".into(), body: "2 failed".into() }]);
    assert_eq!(log, Vec::<String>::new());
    let (planned, log) = hooks("").run_effects(None, 0, json);
    assert_eq!(planned, []);
    assert_eq!(log.len(), 1, "{log:?}");
}
