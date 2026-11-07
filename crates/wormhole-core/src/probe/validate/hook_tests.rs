//! `[[contributes.hooks]]`: each rule of `validate/hooks.rs` by the code of the finding that says it.

use super::tests::{HEAD, report};
use super::*;
use crate::probe::manifest::Manifest;

/// What a hook needs around it: permissions, a background command that takes a capture, and a status
/// item that only hooks feed.
const BASE: &str = r#"
[permissions]
process = ["git"]
notify = true
fold = true
terminal = ["read"]
[[contributes.commands]]
id = "show"
title = "Show"
run = ["git", "show", "$1"]
in = "background"
[[contributes.commands]]
id = "plain"
title = "Plain"
run = ["git", "status"]
in = "background"
[[contributes.status]]
id = "build"
run = ["git", "log", "-1"]
"#;

fn errors(hook: &str) -> Vec<&'static str> {
    let found = report(&format!("{BASE}{hook}"));
    found.errors().map(|f| f.code).collect()
}

/// No error: the permissions BASE grants and a hook leaves unused are warnings, tested on their own.
fn clean(hook: &str) {
    let found = report(&format!("{BASE}{hook}"));
    assert_eq!(found.errors().collect::<Vec<_>>(), Vec::<&Finding>::new(), "{hook}");
}

fn without_read(hook: &str) -> Vec<&'static str> {
    let text = format!("{BASE}{hook}").replace("terminal = [\"read\"]\n", "");
    report(&text).errors().map(|f| f.code).collect()
}

fn hook(on: &str, matcher: &str, action: &str) -> String {
    format!(
        "[[contributes.hooks]]\nid = \"h\"\non = \"{on}\"\nmatch = {{ {matcher} }}\ndo = [{{ {action} }}]\n"
    )
}

const NOTIFY: &str = "type = \"notify\", title = \"t\"";

#[test]
fn a_hook_of_every_kind_is_fine() {
    clean(&hook("command.finished", "exit = \"!=0\", took_ms = \">=60000\"", NOTIFY));
    clean(&hook("bell", "", NOTIFY));
    clean(&hook("agent.status", "status = \"needs_you,failed\", agent = \"claude\"", NOTIFY));
    clean(&hook("git.changed", "branch = \"main,release/*\", dirty = \">0\"", NOTIFY));
    clean(&hook("folder.changed", "folder = \"/Users/*/src/*\"", NOTIFY));
    clean(&hook("title", "pattern = 'build [0-9]+'", NOTIFY));
    clean(&hook("terminal.opened", "when = \"front\", kind = \"shell\"", NOTIFY));
}

// ---- events and matchers --------------------------------------------------------------------

#[test]
fn an_event_name_that_is_not_one_is_a_parse_error_naming_the_choices() {
    let text = format!("{HEAD}[[contributes.hooks]]\nid = \"h\"\non = \"command.done\"\ndo = []\n");
    let err = Manifest::parse(&text).expect_err("not an event");
    assert!(err.message.contains("command.finished"), "{err}");
}

#[test]
fn a_matcher_field_is_one_of_the_events_own() {
    let found = report(&format!("{BASE}{}", hook("bell", "exit = \"0\"", NOTIFY)));
    let finding = found.find("hook-match-field").expect("found");
    assert_eq!(finding.path, "contributes.hooks[0].match.exit");
    assert!(finding.message.contains("when, kind"), "{}", finding.message);
    assert_eq!(errors(&hook("agent.status", "exit = \"0\"", NOTIFY)), ["hook-match-field"]);
    assert_eq!(
        errors(&hook("command.started", "exit = \"0\"", NOTIFY)),
        ["hook-match-field"],
        "no exit before it finishes"
    );
}

#[test]
fn a_matcher_value_has_the_shape_of_its_field() {
    for (on, matcher) in [
        ("command.finished", "exit = \"nonzero\""),
        ("command.finished", "took_ms = \">=\""),
        ("agent.status", "status = \"sleeping\""),
        ("agent.status", "status = \"working,\""),
        ("agent.status", "agent = \"Claude\""),
        ("git.changed", "branch = \"a\\u0007b\""),
        ("git.changed", "branch = \"\""),
        ("bell", "when = \"back\""),
        ("bell", "kind = \"robot\""),
    ] {
        assert_eq!(errors(&hook(on, matcher, NOTIFY)), ["hook-match-value"], "{on} {matcher}");
    }
}

#[test]
fn a_toml_number_is_not_a_matcher_value() {
    // Every value is text, so the operator has somewhere to be.
    let text = format!(
        "{HEAD}[[contributes.hooks]]\nid = \"h\"\non = \"command.finished\"\nmatch = {{ exit = 0 }}\ndo = [{{ {NOTIFY} }}]\n"
    );
    assert!(Manifest::parse(&text).is_err());
}

#[test]
fn a_pattern_has_a_rules_limits_and_a_line_event_needs_one() {
    let bad = |pattern: &str| errors(&hook("output.match", &format!("pattern = '{pattern}'"), NOTIFY));
    assert_eq!(bad("(?m)x"), ["pattern-flag"]);
    assert_eq!(bad("x*"), ["pattern-empty-match"]);
    assert_eq!(bad("("), ["pattern-syntax"]);
    assert_eq!(errors(&hook("output.match", "", NOTIFY)), ["hook-pattern-missing"]);
    assert_eq!(errors(&hook("output.match", "when = \"front\"", NOTIFY)), ["hook-pattern-missing"]);
    clean(&hook("output.match", "pattern = '(?-u:\\b)FAILED'", NOTIFY));
}

// ---- actions and the caps ---------------------------------------------------------------------

#[test]
fn a_hook_does_something_and_not_too_much() {
    assert_eq!(errors("[[contributes.hooks]]\nid = \"h\"\non = \"bell\"\ndo = []\n"), ["hook-no-actions"]);
    let five = format!(
        "[[contributes.hooks]]\nid = \"h\"\non = \"bell\"\ndo = [{}]\n",
        vec![format!("{{ {NOTIFY} }}"); 5].join(", ")
    );
    assert_eq!(errors(&five), ["hook-too-many-actions"]);
    let many: String = (0..33)
        .map(|i| format!("[[contributes.hooks]]\nid = \"h{i}\"\non = \"bell\"\ndo = [{{ {NOTIFY} }}]\n"))
        .collect();
    assert_eq!(errors(&many), ["too-many-hooks"]);
}

#[test]
fn a_hook_id_names_one_hook() {
    let twice = format!("{}{}", hook("bell", "", NOTIFY), hook("bell", "", NOTIFY));
    assert_eq!(errors(&twice), ["contribution-id-duplicate"]);
    assert_eq!(errors(&hook("bell", "", NOTIFY).replace("\"h\"", "\"-h\"")), ["contribution-id"]);
}

#[test]
fn a_field_an_action_does_not_read_is_named() {
    let found =
        report(&format!("{BASE}{}", hook("bell", "", "type = \"notify\", title = \"t\", tone = \"danger\"")));
    assert_eq!(
        found.errors().map(|f| (f.code, f.path.as_str())).collect::<Vec<_>>(),
        [("hook-field-unused", "contributes.hooks[0].do[0].tone")]
    );
    // Typos too: an unknown key is an error here, a warning when the Probe loads.
    assert_eq!(errors(&hook("bell", "", "type = \"notify\", title = \"t\", bodi = \"x\"")), ["unknown-key"]);
}

#[test]
fn notify_needs_its_permission_and_a_title() {
    let text = format!("{BASE}{}", hook("bell", "", NOTIFY)).replace("notify = true\n", "");
    assert_eq!(report(&text).errors().map(|f| f.code).collect::<Vec<_>>(), ["hook-notify-not-permitted"]);
    assert_eq!(errors(&hook("bell", "", "type = \"notify\"")), ["hook-notify-title"]);
}

#[test]
fn highlight_takes_a_tone_of_the_theme_on_matched_text_or_a_finished_block() {
    clean(&hook("command.finished", "", "type = \"highlight\", tone = \"danger\""));
    assert_eq!(
        errors(&hook("command.finished", "", "type = \"highlight\", tone = \"#ff0000\"")),
        ["hook-tone"]
    );
    assert_eq!(errors(&hook("command.finished", "", "type = \"highlight\"")), ["hook-tone"]);
    assert_eq!(
        errors(&hook("command.started", "", "type = \"highlight\", tone = \"danger\"")),
        ["hook-highlight-event"]
    );
}

// ---- templates and the text policy ----------------------------------------------------------------

#[test]
fn a_template_names_the_fields_of_its_event_and_nothing_evaluates() {
    clean(&hook(
        "command.finished",
        "",
        "type = \"notify\", title = \"{took_s} s\", body = \"{exit} {output_lines} {folder_name}\"",
    ));
    for bad in ["{exit} on a bell", "{took_s}", "{1}", "{x"] {
        assert_eq!(
            errors(&hook("bell", "", &format!("type = \"notify\", title = \"{bad}\""))).len(),
            1,
            "{bad}"
        );
    }
    assert_eq!(errors(&hook("bell", "", "type = \"notify\", title = \"{x\"")), ["hook-template"]);
    assert_eq!(errors(&hook("bell", "", "type = \"notify\", title = \"{nope}\"")), ["hook-template-field"]);
    assert_eq!(
        errors(&hook("bell", "", "type = \"notify\", title = \"a\\u001b[2Jb\"")),
        ["hook-text-control"]
    );
}

#[test]
fn text_from_a_terminal_leaves_a_hook_only_with_terminal_read() {
    // Ids, numbers and names are free; a command line, a branch, a line of output are not.
    let free =
        hook("agent.status", "", "type = \"notify\", title = \"{agent} {status} {terminal} {folder_name}\"");
    assert_eq!(without_read(&free), Vec::<&str>::new());
    for (on, matcher, field) in [
        ("command.finished", "", "command"),
        ("agent.status", "", "message"),
        ("git.changed", "", "branch"),
        ("folder.changed", "", "folder"),
        ("title", "", "title"),
    ] {
        let text = hook(on, matcher, &format!("type = \"notify\", title = \"t\", body = \"{{{field}}}\""));
        assert_eq!(without_read(&text), ["hook-needs-terminal-read"], "{field}");
        assert_eq!(errors(&text), Vec::<&str>::new(), "{field} with the permission");
    }
    // A capture is text from the terminal; matching on it is not sending it anywhere.
    let capture = hook(
        "output.match",
        "pattern = '(?-u:\\b)FAILED ([a-z]{1,20})'",
        "type = \"notify\", title = \"{1}\"",
    );
    assert_eq!(without_read(&capture), ["hook-needs-terminal-read"]);
    let matching_only =
        hook("output.match", "pattern = '(?-u:\\b)FAILED'", "type = \"highlight\", tone = \"danger\"");
    assert_eq!(without_read(&matching_only), Vec::<&str>::new());
}

#[test]
fn a_capture_names_a_group_the_pattern_has() {
    let with = |template: &str| {
        hook(
            "output.match",
            "pattern = '(?-u:\\b)FAILED ([a-z]{1,20})'",
            &format!("type = \"notify\", title = \"{template}\""),
        )
    };
    clean(&with("{0} {1}"));
    assert_eq!(errors(&with("{2}")), ["hook-template-group"]);
    assert_eq!(errors(&hook("bell", "", "type = \"notify\", title = \"{1}\"")), ["hook-template-group"]);
    clean(&hook("title", "pattern = '(build) [0-9]+'", "type = \"notify\", title = \"{1}\""));
}

#[test]
fn the_permissions_a_hook_never_uses_are_a_warning() {
    let found = report(&format!("{BASE}{}", hook("bell", "", NOTIFY)));
    let codes: Vec<_> = found.warnings().map(|f| f.code).collect();
    assert!(
        codes.contains(&"permission-terminal-read-unused") && codes.contains(&"permission-fold-unused"),
        "{codes:?}"
    );
    assert!(found.is_ok());
}

// ---- fold ---------------------------------------------------------------------------------------

const FOLD: &str = "type = \"fold\", target = \"block-output\", label = \"build output\"";

#[test]
fn fold_needs_its_permission_and_says_which_rows_and_in_what_words() {
    clean(&hook("command.finished", "", &format!("{FOLD}, keep_head = 1, keep_tail = 3, min_lines = 10")));
    let denied = format!("{BASE}{}", hook("command.finished", "", FOLD)).replace("fold = true\n", "");
    assert_eq!(report(&denied).errors().map(|f| f.code).collect::<Vec<_>>(), ["hook-fold-not-permitted"]);
    assert_eq!(errors(&hook("command.finished", "", "type = \"fold\", label = \"x\"")), ["hook-fold-target"]);
    assert_eq!(
        errors(&hook("command.finished", "", "type = \"fold\", target = \"block-output\"")),
        ["hook-fold-label"]
    );
    assert_eq!(errors(&hook("bell", "", FOLD)), ["hook-fold-event"]);
    assert_eq!(errors(&hook("output.match", "pattern = 'x'", FOLD)), ["hook-fold-event"]);
    assert_eq!(
        errors(&hook("command.finished", "", "type = \"fold\", target = \"match-run\", label = \"x\"")),
        ["hook-fold-event"]
    );
    clean(&hook("output.match", "pattern = 'x'", "type = \"fold\", target = \"match-run\", label = \"x\""));
}

#[test]
fn a_marker_label_is_fixed_plain_text() {
    for label in ["", "   ", &"x".repeat(41), "{1} lines", "a}", "a\\u001b[2Jb", "\\u202ecba"] {
        let action = format!("type = \"fold\", target = \"block-output\", label = \"{label}\"");
        assert_eq!(errors(&hook("command.finished", "", &action)), ["hook-fold-label"], "{label:?}");
    }
    clean(&hook(
        "command.finished",
        "",
        &format!("type = \"fold\", target = \"block-output\", label = \"{}\"", "x".repeat(40)),
    ));
}

#[test]
fn fold_numbers_are_in_range() {
    for (field, code) in [
        ("keep_head = 21", "hook-fold-keep"),
        ("keep_tail = 21", "hook-fold-keep"),
        ("min_lines = 0", "hook-fold-min-lines"),
        ("min_lines = 100001", "hook-fold-min-lines"),
    ] {
        assert_eq!(errors(&hook("command.finished", "", &format!("{FOLD}, {field}"))), [code], "{field}");
    }
    let found = report(&format!(
        "{BASE}{}",
        hook("command.finished", "", &format!("{FOLD}, keep_head = 2, keep_tail = 2, min_lines = 5"))
    ));
    assert!(found.find("hook-fold-min-lines-low").is_some() && found.is_ok());
}

#[test]
fn fold_is_a_touch_id_permission_in_the_summary() {
    let found = report(&format!("{BASE}{}", hook("command.finished", "", FOLD)));
    let fold: Vec<_> = found
        .summary
        .permissions
        .iter()
        .filter(|p| p.key == "fold")
        .map(|p| (p.detail.as_str(), p.confirmation))
        .collect();
    assert_eq!(fold, [("hide parts of your terminal output", Confirmation::TouchId)]);
    assert_eq!(found.summary.hooks.len(), 1);
    assert_eq!(
        found.summary.hooks[0].actions[0],
        "fold block-output \"build output\", keeping 0 and 0 rows (hides output on screen; copy still gives it)"
    );
}

// ---- open-path, run and status ----------------------------------------------------------------------

const LOC: &str = "pattern = '(?-u:\\b)([a-z/]{1,100}\\.rs):([0-9]{1,5})(?-u:\\b)'";

#[test]
fn open_path_offers_a_file_and_never_a_link() {
    clean(&hook("output.match", LOC, "type = \"open-path\", path = \"{1}\", line = \"{2}\""));
    clean(&hook("output.match", LOC, "type = \"open-path\", path = \"/tmp/x.log\", line = \"12\""));
    assert_eq!(errors(&hook("output.match", LOC, "type = \"open-path\"")), ["hook-open-path"]);
    for path in ["https://x.dev/a", "../etc/passwd", "a/../b"] {
        assert_eq!(
            errors(&hook("output.match", LOC, &format!("type = \"open-path\", path = \"{path}\""))),
            ["hook-open-path"],
            "{path}"
        );
    }
    for line in ["x", "{1}{2}", "1.5", "123456789"] {
        let errs = errors(&hook(
            "output.match",
            LOC,
            &format!("type = \"open-path\", path = \"{{1}}\", line = \"{line}\""),
        ));
        assert_eq!(errs, ["hook-open-path-line"], "{line}");
    }
    let unbounded =
        hook("output.match", "pattern = '(?-u:\\b)([a-z/]+\\.rs)'", "type = \"open-path\", path = \"{1}\"");
    assert_eq!(errors(&unbounded), ["capture-unanchored"]);
    assert_eq!(
        without_read(&hook("output.match", LOC, "type = \"open-path\", path = \"{1}\"")),
        ["hook-needs-terminal-read"]
    );
}

#[test]
fn run_runs_a_background_command_of_the_probe_and_nothing_else() {
    let anchored = "pattern = '(?-u:\\b)FAILED ([A-Za-z0-9_][A-Za-z0-9_.:-]{0,79})'";
    clean(&hook(
        "output.match",
        anchored,
        "type = \"run\", command = \"show\", item = \"build\", timeout = 30",
    ));
    clean(&hook("bell", "", "type = \"run\", command = \"plain\""));
    assert_eq!(errors(&hook("bell", "", "type = \"run\"")), ["hook-run-command"]);
    assert_eq!(errors(&hook("bell", "", "type = \"run\", command = \"nope\"")), ["hook-run-command"]);
    assert_eq!(
        errors(&hook("bell", "", "type = \"run\", command = \"plain\", timeout = 61")),
        ["hook-run-timeout"]
    );
    assert_eq!(
        errors(&hook("bell", "", "type = \"run\", command = \"plain\", timeout = 0")),
        ["hook-run-timeout"]
    );
    assert_eq!(
        errors(&hook("bell", "", "type = \"run\", command = \"plain\", item = \"nope\"")),
        ["hook-status-item"]
    );
    // The same capture rules as a rule's command: anchored, bounded, never an option.
    let loose = "pattern = '(?-u:\\b)FAILED (\\S{1,80})'";
    assert_eq!(
        errors(&hook("output.match", loose, "type = \"run\", command = \"show\"")),
        ["capture-unanchored"]
    );
    assert_eq!(
        errors(&hook("bell", "", "type = \"run\", command = \"show\"")),
        ["hook-run-capture-without-pattern"]
    );
    assert_eq!(
        without_read(&hook("output.match", anchored, "type = \"run\", command = \"show\"")),
        ["hook-needs-terminal-read"]
    );
}

#[test]
fn run_refuses_a_command_that_opens_a_terminal_or_has_no_argv() {
    let text = format!("{BASE}{}", hook("bell", "", "type = \"run\", command = \"plain\"")).replace(
        "in = \"background\"\n[[contributes.commands]]\nid = \"plain\"",
        "[[contributes.commands]]\nid = \"plain\"",
    );
    assert!(report(&text).find("hook-run-in-terminal").is_none(), "{:?}", report(&text).findings);
    let in_terminal = format!("{BASE}{}", hook("bell", "", "type = \"run\", command = \"plain\"")).replace(
        "run = [\"git\", \"status\"]\nin = \"background\"",
        "run = [\"git\", \"status\"]\nin = \"new-terminal\"",
    );
    assert!(report(&in_terminal).find("hook-run-in-terminal").is_some());
    let no_argv = format!("{BASE}{}", hook("bell", "", "type = \"run\", command = \"plain\""))
        .replace("run = [\"git\", \"status\"]\nin = \"background\"", "");
    let found = report(&no_argv);
    assert!(found.find("hook-run-no-argv").is_some(), "{:?}", found.findings);
}

#[test]
fn status_sets_a_declared_item() {
    clean(&hook(
        "bell",
        "",
        "type = \"status\", item = \"build\", text = \"rang\", tone = \"warning\", icon = \"bell\", tooltip = \"rang\"",
    ));
    assert_eq!(errors(&hook("bell", "", "type = \"status\", text = \"x\"")), ["hook-status-item"]);
    assert_eq!(
        errors(&hook("bell", "", "type = \"status\", item = \"nope\", text = \"x\"")),
        ["hook-status-item"]
    );
    assert_eq!(errors(&hook("bell", "", "type = \"status\", item = \"build\"")), ["hook-status-text"]);
    assert_eq!(
        errors(&hook("bell", "", "type = \"status\", item = \"build\", text = \"x\", tone = \"red\"")),
        ["hook-tone"]
    );
    assert_eq!(
        errors(&hook("bell", "", "type = \"status\", item = \"build\", text = \"x\", icon = \"../x\"")),
        ["hook-status-icon"]
    );
}

#[test]
fn a_status_item_with_no_run_needs_a_hook_that_feeds_it() {
    let unfed = "[[contributes.status]]\nid = \"lonely\"\n";
    assert_eq!(report(unfed).errors().map(|f| f.code).collect::<Vec<_>>(), ["status-no-run"]);
    let fed = format!("{unfed}{}", hook("bell", "", "type = \"status\", item = \"lonely\", text = \"x\""));
    assert_eq!(report(&fed).findings, []);
    let by_run = format!(
        "[permissions]\nprocess = [\"git\"]\n[[contributes.commands]]\nid = \"c\"\ntitle = \"C\"\nrun = [\"git\", \"status\"]\nin = \"background\"\n{unfed}{}",
        hook("bell", "", "type = \"run\", command = \"c\", item = \"lonely\"")
    );
    assert_eq!(report(&by_run).errors().map(|f| f.code).collect::<Vec<_>>(), Vec::<&str>::new());
}

// ---- permissions around hooks -------------------------------------------------------------------------

#[test]
fn reading_a_terminal_is_not_code_s_alone_once_a_probe_has_hooks() {
    let read_only = "[permissions]\nterminal = [\"read\"]\n";
    assert_eq!(report(read_only).errors().map(|f| f.code).collect::<Vec<_>>(), ["needs-code"]);
    let with_hook = format!(
        "{read_only}{}",
        hook("command.finished", "", "type = \"notify\", title = \"t\", body = \"{command}\"")
    )
    .replace("[permissions]\nterminal", "[permissions]\nnotify = true\nterminal");
    assert_eq!(report(&with_hook).findings, []);
    let write = "[permissions]\nnotify = true\nterminal = [\"write\"]\n";
    let typed = format!("{write}{}", hook("bell", "", NOTIFY));
    assert_eq!(
        report(&typed).errors().map(|f| f.code).collect::<Vec<_>>(),
        ["needs-code"],
        "typing into a terminal is code's"
    );
}

#[test]
fn reading_text_and_running_a_program_is_the_dangerous_pair_a_hook_makes() {
    let found = report(&format!(
        "{BASE}{}",
        hook(
            "output.match",
            "pattern = '(?-u:\\b)FAILED ([a-z]{1,20})'",
            "type = \"run\", command = \"show\""
        )
    ));
    assert!(
        found
            .summary
            .combinations
            .iter()
            .any(|c| c.description == "can send what is on your terminals to anything, through git"),
        "{:?}",
        found.summary.combinations
    );
}

#[test]
fn the_summary_says_when_a_hook_fires_in_words() {
    let found = report(&format!(
        "{BASE}{}",
        hook("command.finished", "exit = \"!=0\", took_ms = \">=60000\"", NOTIFY)
    ));
    let note = &found.summary.hooks[0];
    assert_eq!(
        (note.id.as_str(), note.on, note.when.as_str()),
        ("h", "command.finished", "exit != 0, took_ms >= 60000")
    );
    assert_eq!(note.actions, ["notify \"t\""]);
}
