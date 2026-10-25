//! `validate` end to end on small in-memory Probes: each test names what the design says and
//! checks the code of the finding that says it. The fixture folders in `tests/probes/` run the same
//! checks from real folders.

use std::path::PathBuf;

use super::*;
use crate::probe::manifest::Manifest;
use crate::probe::read::{MANIFEST_FILE, ProbeFile};

const HEAD: &str = "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
                    version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n";

/// A Probe of `HEAD`, then `rest` (more of `[probe]`, or any other table), and `files`.
fn probe(rest: &str, files: &[(&str, &str)]) -> ProbeFolder {
    let text = format!("{HEAD}{rest}");
    let parsed = Manifest::parse(&text).unwrap_or_else(|err| panic!("{err}\n{text}"));
    let mut all = vec![ProbeFile::new(MANIFEST_FILE, text.as_bytes())];
    all.extend(files.iter().map(|(path, body)| ProbeFile::new(*path, body.as_bytes())));
    ProbeFolder { root: PathBuf::new(), manifest: parsed.manifest, warnings: parsed.warnings, files: all }
}

fn report(rest: &str) -> Report {
    validate(&probe(rest, &[]))
}

fn has(rest: &str, code: &str) -> bool {
    let found = report(rest);
    found.find(code).is_some() || panic!("no {code} in {:#?}", found.findings)
}

fn clean(rest: &str) {
    let found = report(rest);
    assert_eq!(found.findings, [], "{rest}");
}

#[test]
fn a_probe_that_is_only_its_identity_is_fine() {
    clean("");
    assert!(report("").is_ok());
}

// ---- identity -------------------------------------------------------------------------------

#[test]
fn identity_fields_are_checked() {
    for (bad, code) in [
        (HEAD.replace("tiny", "Tiny!"), "probe-id"),
        (HEAD.replace("\"d\"", "\"\""), "probe-text-empty"),
        (HEAD.replace("\"a\"", "\"a\\u001b[2J\""), "probe-text-control"),
        (HEAD.replace("0.1.0", "1.0"), "probe-version"),
        (HEAD.replace(">=1.3", "soon"), "probe-wormhole-range"),
        (HEAD.replace("api = 1", "api = 2"), "probe-api"),
    ] {
        let parsed = Manifest::parse(&bad).unwrap_or_else(|err| panic!("{err}"));
        let folder = ProbeFolder { manifest: parsed.manifest, ..probe("", &[]) };
        assert!(validate(&folder).find(code).is_some(), "{code}");
    }
}

#[test]
fn a_repository_that_is_not_https_is_a_warning() {
    let found = report("repository = \"git@github.com:a/b\"\n");
    assert_eq!(found.findings.len(), 1);
    assert_eq!(found.findings[0].severity, Severity::Warning);
    assert!(found.is_ok());
}

#[test]
fn unknown_keys_are_errors_here() {
    let found = report("flavour = \"x\"\n");
    assert_eq!(found.findings[0].code, "unknown-key");
    assert_eq!(found.findings[0].path, "probe.flavour");
    assert!(!found.is_ok());
}

#[test]
fn main_and_code_api_go_together() {
    assert!(has("code_api = 0\n", "code-api-without-main"));
    assert!(has("main = \"src/index.ts\"\n", "main-without-code-api"));
    assert!(has("main = \"src/index.ts\"\ncode_api = 3\n", "code-api-unsupported"));
    let folder = probe("main = \"src/index.ts\"\ncode_api = 0\n", &[("src/index.ts", "export {}")]);
    assert_eq!(validate(&folder).findings, []);
    assert!(validate(&folder).summary.has_code);
    let folder = probe("main = \"src/index.py\"\ncode_api = 0\n", &[("src/index.py", "")]);
    assert!(validate(&folder).find("main-extension").is_some());
    let folder = probe("main = \"../x.ts\"\ncode_api = 0\n", &[]);
    assert!(validate(&folder).find("main-path").is_some());
    let folder = probe("main = \"src/index.ts\"\ncode_api = 0\n", &[]);
    assert!(validate(&folder).find("main-missing").is_some());
}

#[test]
fn counts_are_capped() {
    let many = |kind: &str, n: usize, body: &str| {
        (0..n).map(|i| format!("[[contributes.{kind}]]\nid = \"x{i}\"\n{body}")).collect::<String>()
    };
    let rules = many("rules", 65, "pattern = \"a+\"\ntone = \"link\"\n");
    assert!(validate(&probe(&rules, &[])).find("too-many-rules").is_some());
    let rules = many("rules", 64, "pattern = \"a+\"\ntone = \"link\"\n");
    assert_eq!(validate(&probe(&rules, &[])).findings, []);
    let commands = many("commands", 17, "title = \"t\"\nrun = [\"say\"]\nin = \"background\"\n");
    assert!(
        validate(&probe(&format!("[permissions]\nprocess = [\"say\"]\n{commands}"), &[]))
            .find("too-many-commands")
            .is_some()
    );
    let status = many("status", 9, "run = [\"say\"]\n");
    assert!(
        validate(&probe(&format!("[permissions]\nprocess = [\"say\"]\n{status}"), &[]))
            .find("too-many-status")
            .is_some()
    );
    let agents =
        many("agents", 5, "name = \"A\"\ncommand = \"zork\"\nexecutables = [\"zork\"]\nmonogram = \"ZK\"\n");
    assert!(validate(&probe(&agents, &[])).find("too-many-agents").is_some());
}

#[test]
fn ids_are_local_and_unique_within_a_kind() {
    let rule =
        |id: &str| format!("[[contributes.rules]]\nid = \"{id}\"\npattern = \"a+\"\ntone = \"link\"\n");
    assert!(has(&rule("Bad Id"), "contribution-id"));
    let found = report(&(rule("a") + &rule("a")));
    let dup = found.find("contribution-id-duplicate").expect("duplicate");
    assert_eq!(dup.path, "contributes.rules[1].id");
    assert!(dup.message.contains("tiny/a"), "{}", dup.message);
    // The same local id in two kinds is two different contributions.
    let both = rule("same")
        + "[[contributes.commands]]\nid = \"same\"\ntitle = \"t\"\nrun = [\"say\"]\nin = \"background\"\n[permissions]\nprocess = [\"say\"]\n";
    assert_eq!(report(&both).findings, []);
}

#[test]
fn what_needs_code_is_said_so_without_a_main() {
    assert!(has("[permissions]\nterminal = [\"read\"]\n", "needs-code"));
    assert!(has("[permissions]\nnet = [\"api.acme.dev\"]\n", "needs-code"));
    assert!(has("[permissions]\nfs = [\"read:/tmp\"]\n", "needs-code"));
    assert!(has("[[contributes.panels]]\nid = \"p\"\n", "needs-code"));
    assert!(has("[[contributes.hovers]]\nid = \"p\"\n", "needs-code"));
    let message = &report("[permissions]\nterminal = [\"read\"]\n").findings[0].message;
    assert!(message.contains("E2") && message.contains("not supported yet"), "{message}");
    assert!(has("[[contributes.commands]]\nid = \"c\"\ntitle = \"t\"\n", "command-no-run"));
    assert!(has("[[contributes.status]]\nid = \"s\"\n", "status-no-run"));
    // With code they are the code's to handle.
    let folder = probe(
        "main = \"i.ts\"\ncode_api = 0\n[permissions]\nterminal = [\"read\", \"write\"]\n[[contributes.status]]\nid = \"s\"\n[[contributes.commands]]\nid = \"c\"\ntitle = \"t\"\n",
        &[("i.ts", "")],
    );
    assert_eq!(validate(&folder).findings, []);
}

// ---- rules ------------------------------------------------------------------------------------

fn rule(body: &str) -> String {
    format!("[[contributes.rules]]\nid = \"r\"\n{body}")
}

#[test]
fn a_rule_pattern_is_analysed_at_its_path() {
    let found = report(&rule("pattern = 'a*'\ntone = \"link\"\n"));
    let finding = found.find("pattern-empty-match").expect("empty");
    assert_eq!(finding.path, "contributes.rules[0].pattern");
}

#[test]
fn look_keep_and_reveal_must_agree() {
    assert!(has(&rule("pattern = 'a+'\nlook = \"redact\"\nkeep = \"start:1\"\n"), "rule-keep-needs-shorten"));
    assert!(has(&rule("pattern = 'a+'\ntone = \"link\"\nkeep = \"start:1\"\n"), "rule-keep-needs-shorten"));
    assert!(has(&rule("pattern = 'a+'\nlook = \"shorten\"\nkeep = \"start:0\"\n"), "rule-keep"));
    assert!(has(&rule("pattern = 'a+'\nlook = \"shorten\"\nkeep = \"domain,domain\"\n"), "rule-keep"));
    assert!(has(&rule("pattern = 'a+'\nlook = \"shorten\"\n"), "rule-shorten-keeps-nothing"));
    assert!(has(&rule("pattern = 'a+'\ntone = \"link\"\nreveal = \"hover\"\n"), "rule-reveal-needs-look"));
    assert!(has(&rule("pattern = 'a+'\n"), "rule-does-nothing"));
    clean(&rule("pattern = 'a+'\nlook = \"shorten\"\nkeep = \"start:1,end:2,domain\"\nreveal = \"never\"\n"));
    clean(&rule("pattern = 'a+'\nlook = \"redact\"\nreveal = \"deliberate\"\n"));
}

#[test]
fn a_tone_is_a_token_name_and_a_colour_is_an_error() {
    for tone in ["link", "danger", "warning", "success", "muted"] {
        clean(&rule(&format!("pattern = 'a+'\ntone = \"{tone}\"\n")));
    }
    for colour in ["#ff0000", "#f00", "rgb(1,2,3)", "ff0000"] {
        assert!(has(&rule(&format!("pattern = 'a+'\ntone = \"{colour}\"\n")), "tone-colour"), "{colour}");
    }
    assert!(has(&rule("pattern = 'a+'\ntone = \"pink\"\n"), "tone-unknown"));
}

#[test]
fn a_hover_is_short_plain_and_names_groups_that_exist() {
    clean(&rule("pattern = '(a)+'\ntone = \"link\"\nhover = \"Open {1} for {0}\"\n"));
    assert!(has(
        &rule(&format!("pattern = 'a+'\ntone = \"link\"\nhover = \"{}\"\n", "x".repeat(121))),
        "hover-too-long"
    ));
    clean(&rule(&format!("pattern = 'a+'\ntone = \"link\"\nhover = \"{}\"\n", "x".repeat(120))));
    assert!(has(&rule("pattern = '(a)+'\ntone = \"link\"\nhover = \"{2}\"\n"), "hover-group"));
    assert!(has(&rule("pattern = 'a+'\ntone = \"link\"\nhover = \"{1}\"\n"), "hover-group"));
    assert!(has(&rule("pattern = 'a+'\ntone = \"link\"\nhover = \"{oops}\"\n"), "hover-template"));
}

fn open_url(url: &str, open: &str) -> String {
    format!(
        "[permissions]\nopen = [{open}]\n{}",
        rule(&format!("pattern = '(a+)(b+)'\naction = {{ type = \"open-url\", url = \"{url}\" }}\n"))
    )
}

#[test]
fn open_url_has_a_fixed_https_host_the_probe_may_open() {
    clean(&open_url("https://acme.atlassian.net/browse/{1}", "\"acme.atlassian.net\""));
    assert!(has(&open_url("https://acme.atlassian.net/browse/{1}", ""), "action-url-host-not-permitted"));
    assert!(has(
        &open_url("https://acme.atlassian.net/browse/{1}", "\"other.net\""),
        "action-url-host-not-permitted"
    ));
    assert!(has(&open_url("https://{1}.atlassian.net/", "\"x.atlassian.net\""), "action-url-host-capture"));
    assert!(has(&open_url("http://acme.net/{1}", "\"acme.net\""), "action-url"));
    assert!(has(&open_url("https://acme.net@evil.example/{1}", "\"acme.net\""), "action-url"));
    assert!(has(&open_url("https://acme.net/{3}", "\"acme.net\""), "action-group"));
    assert!(has(&open_url("https://acme.net/ {1}", "\"acme.net\""), "action-url"));
    assert!(has(&rule("pattern = 'a+'\naction = { type = \"open-url\" }\n"), "action-url-missing"));
}

#[test]
fn an_action_field_the_type_does_not_read_is_an_error() {
    assert!(has(
        &rule("pattern = 'a+'\naction = { type = \"copy\", url = \"https://x.net\" }\n"),
        "action-field-unused"
    ));
    assert!(has(
        &rule("pattern = 'a+'\naction = { type = \"none\", command = \"x\" }\n"),
        "action-field-unused"
    ));
    clean(&rule("pattern = '(a)+'\naction = { type = \"copy\", text = \"{1}\" }\n"));
    assert!(has(&rule("pattern = 'a+'\naction = { type = \"copy\", text = \"{1}\" }\n"), "action-group"));
}

fn command_rule(pattern: &str, run: &str) -> String {
    format!(
        "[permissions]\nprocess = [\"git\"]\n{}[[contributes.commands]]\nid = \"show\"\ntitle = \"Show\"\nrun = {run}\nin = \"background\"\n",
        rule(&format!("pattern = '{pattern}'\naction = {{ type = \"command\", command = \"show\" }}\n"))
    )
}

#[test]
fn a_command_action_needs_an_anchored_pattern() {
    let run = "[\"git\", \"show\", \"$1\"]";
    clean(&command_rule(r"(?-u:\b)([0-9a-f][0-9a-f]{6,39})(?-u:\b)", run));
    for bad in ["(.+)", r"(\S{1,40})", "([a-z-]{3,9})", "(-?[0-9]{3,9})", "([a-z]+)"] {
        let found = report(&command_rule(bad, run));
        let finding =
            found.find("capture-unanchored").unwrap_or_else(|| panic!("{bad}: {:?}", found.findings));
        assert_eq!(finding.path, "contributes.rules[0].action.command", "{bad}");
        assert!(!found.is_ok());
    }
}

#[test]
fn a_command_action_names_a_command_and_captures_that_exist() {
    let run = "[\"git\", \"show\", \"$2\"]";
    assert!(has(&command_rule("([0-9a-f]{7,9})", run), "action-capture-missing"));
    let no_command = rule("pattern = '(a)+'\naction = { type = \"command\", command = \"nope\" }\n");
    assert!(has(&no_command, "action-command-unknown"));
    assert!(has(&rule("pattern = 'a+'\naction = { type = \"command\" }\n"), "action-command-missing"));
    // The whole match is `$0`.
    clean(&command_rule("[0-9a-f]{7,9}", "[\"git\", \"show\", \"$0\"]"));
    assert!(has(&command_rule("[0-9a-f-]{7,9}", "[\"git\", \"show\", \"$0\"]"), "capture-unanchored"));
}

#[test]
fn a_command_that_takes_no_capture_asks_nothing_of_the_pattern() {
    clean(&command_rule("(.+)", "[\"git\", \"status\"]"));
}

// ---- commands and status ------------------------------------------------------------------

fn command(body: &str, process: &str) -> String {
    format!(
        "[permissions]\nprocess = [{process}]\n[[contributes.commands]]\nid = \"c\"\ntitle = \"T\"\n{body}"
    )
}

#[test]
fn a_program_must_be_listed_and_a_script_must_be_inside_the_folder() {
    let ok = probe(
        &command("run = [\"./scripts/go.sh\", \"-x\"]\nin = \"background\"\n", "\"./scripts/go.sh\""),
        &[("scripts/go.sh", "")],
    );
    assert_eq!(validate(&ok).findings, []);
    assert!(has(&command("run = [\"say\"]\n", ""), "run-program-not-permitted"));
    assert!(has(&command("run = [\"say\"]\n", "\"afplay\""), "run-program-not-permitted"));
    assert!(has(&command("run = []\n", ""), "run-empty"));
    assert!(has(&command("run = [\"/bin/ls\"]\n", "\"/bin/ls\""), "run-program-path"));
    assert!(has(&command("run = [\"bin/ls\"]\n", "\"bin/ls\""), "run-program-path"));
    // A script is judged where it is listed: it must be plain and be a file of the folder.
    assert!(has(&command("run = [\"./../x.sh\"]\n", "\"./../x.sh\""), "permission-process"));
    assert!(has(&command("run = [\"./a/../../x.sh\"]\n", "\"./a/../../x.sh\""), "permission-process"));
    assert!(has(
        &command("run = [\"./scripts/gone.sh\"]\n", "\"./scripts/gone.sh\""),
        "permission-process-missing"
    ));
    assert!(has(&command("run = [\"./scripts/go.sh\"]\n", "\"say\""), "run-program-not-permitted"));
    assert!(has(&command("run = [\"$1\"]\n", "\"$1\""), "run-program-capture"));
    assert!(has(&command("run = [\"say\", \"a\\u0000b\"]\n", "\"say\""), "run-nul"));
}

#[test]
fn a_capture_is_only_ever_a_whole_argument() {
    assert!(has(
        &command("run = [\"say\", \"--to=$1\"]\nin = \"background\"\n", "\"say\""),
        "run-capture-embedded"
    ));
    assert!(has(
        &command("run = [\"say\", \"a$1\"]\nin = \"background\"\n", "\"say\""),
        "run-capture-embedded"
    ));
}

#[test]
fn a_capture_in_a_new_terminal_command_is_an_error_and_an_unbound_one_a_warning() {
    let found = report(&command("run = [\"say\", \"$1\"]\nin = \"new-terminal\"\n", "\"say\""));
    assert!(found.errors().any(|f| f.code == "command-capture-new-terminal"), "{:?}", found.findings);
    assert!(found.warnings().any(|f| f.code == "command-capture-unbound"));
    assert!(!found.is_ok());
    let background = report(&command("run = [\"say\", \"$1\"]\nin = \"background\"\n", "\"say\""));
    assert!(background.find("command-capture-new-terminal").is_none());
}

#[test]
fn git_runs_only_read_only_and_never_in_a_new_terminal() {
    clean(&command("run = [\"git\", \"log\", \"-1\"]\nin = \"background\"\n", "\"git\""));
    assert!(has(&command("run = [\"git\", \"push\"]\nin = \"background\"\n", "\"git\""), "git-subcommand"));
    assert!(has(
        &command("run = [\"git\", \"-c\", \"x=y\", \"log\"]\nin = \"background\"\n", "\"git\""),
        "git-subcommand"
    ));
    assert!(has(
        &command("run = [\"git\", \"log\"]\nin = \"new-terminal\"\n", "\"git\""),
        "git-in-new-terminal"
    ));
    assert!(
        has(&command("run = [\"git\", \"log\"]\n", "\"git\""), "git-in-new-terminal"),
        "new-terminal is the default"
    );
    assert!(has(
        &command("run = [\"git\", \"branch\", \"topic\"]\nin = \"background\"\n", "\"git\""),
        "git-branch-writes"
    ));
    assert!(has(
        &command("run = [\"git\", \"diff\", \"--output=/tmp/x\"]\nin = \"background\"\n", "\"git\""),
        "git-flag"
    ));
}

#[test]
fn a_key_must_parse_and_be_free() {
    let with_key =
        |key: &str| command(&format!("run = [\"say\"]\nin = \"background\"\nkey = \"{key}\"\n"), "\"say\"");
    clean(&with_key("cmd-alt-shift-9"));
    clean(&with_key("f9"));
    assert!(has(&with_key("cmd-"), "command-key"));
    assert!(has(&with_key("cmd-banana"), "command-key"));
    assert!(has(&with_key("a"), "command-key-taken"), "the shell needs a bare letter");
    assert!(has(&with_key("cmd-t"), "command-key-taken"), "the app's New Terminal");
}

#[test]
fn two_commands_cannot_share_a_key() {
    let two = "[permissions]\nprocess = [\"say\"]\n[[contributes.commands]]\nid = \"a\"\ntitle = \"A\"\nrun = [\"say\"]\nin = \"background\"\nkey = \"cmd-alt-shift-9\"\n[[contributes.commands]]\nid = \"b\"\ntitle = \"B\"\nrun = [\"say\"]\nin = \"background\"\nkey = \"alt-cmd-shift-9\"\n";
    assert!(has(two, "command-key-duplicate"));
}

fn status(body: &str) -> String {
    format!(
        "[permissions]\nprocess = [\"git\"]\n[[contributes.status]]\nid = \"s\"\nrun = [\"git\", \"log\", \"-1\"]\n{body}"
    )
}

#[test]
fn a_status_item_has_an_interval_and_a_timeout_in_range() {
    clean(&status(""));
    clean(&status("interval = 5\ntimeout = 10\n"));
    assert!(has(&status("interval = 4\n"), "status-interval"));
    assert!(has(&status("timeout = 11\n"), "status-timeout"));
    assert!(has(&status("timeout = 0\n"), "status-timeout"));
}

#[test]
fn a_status_item_takes_no_captures_and_runs_read_only_git() {
    let found = report(
        "[permissions]\nprocess = [\"git\"]\n[[contributes.status]]\nid = \"s\"\nrun = [\"git\", \"log\", \"$1\"]\n",
    );
    assert!(found.find("run-capture-not-allowed").is_some());
    let found = report(
        "[permissions]\nprocess = [\"git\"]\n[[contributes.status]]\nid = \"s\"\nrun = [\"git\", \"gc\"]\n",
    );
    assert!(found.find("git-subcommand").is_some());
}

// ---- agents -------------------------------------------------------------------------------------

const ICON: &str = "<svg viewBox='0 0 8 8'><path d='M0 0h8v8z'/></svg>";

fn agent(body: &str) -> String {
    format!(
        "[[contributes.agents]]\nid = \"zork\"\nname = \"Zork\"\ncommand = \"zork\"\nexecutables = [\"zork\"]\nmonogram = \"ZK\"\n{body}"
    )
}

#[test]
fn an_agent_is_recognised_by_names_and_drawn_by_a_monogram_and_an_icon() {
    clean(&agent(""));
    let with_icon = probe(&agent("icon = \"assets/z.svg\"\n"), &[("assets/z.svg", ICON)]);
    assert_eq!(validate(&with_icon).findings, []);
    assert!(has(&agent("").replace("ZK", "Zk"), "agent-monogram"));
    assert!(has(&agent("").replace("ZK", "ZKX"), "agent-monogram"));
    assert!(has(&agent("").replace("ZK", "Z1"), "agent-monogram"));
    assert!(has(&agent("").replace("executables = [\"zork\"]", "executables = []"), "agent-executables"));
    assert!(has(&agent("").replace("[\"zork\"]", "[\"/usr/bin/zork\"]"), "agent-executables"));
    assert!(has(&agent("").replace("command = \"zork\"", "command = \"zork now\""), "agent-command"));
}

#[test]
fn a_built_in_agent_s_names_cannot_be_taken() {
    let found = report(&agent("").replace("[\"zork\"]", "[\"claude\"]"));
    assert!(found.find("agent-builtin-name").is_some(), "{:?}", found.findings);
    let found = report(&agent("").replace("command = \"zork\"", "command = \"codex\""));
    assert!(found.find("agent-builtin-name").is_some());
}

#[test]
fn an_agent_icon_is_a_small_inert_svg_of_the_folder() {
    let with =
        |icon: &str, files: &[(&str, &str)]| validate(&probe(&agent(&format!("icon = \"{icon}\"\n")), files));
    assert!(with("assets/z.svg", &[]).find("agent-icon-missing").is_some());
    assert!(with("../z.svg", &[]).find("agent-icon-path").is_some());
    assert!(with("/etc/z.svg", &[]).find("agent-icon-path").is_some());
    let big = format!("<svg>{}</svg>", "<path d='M0 0'/>".repeat(600));
    assert!(big.len() > 8 * 1024);
    assert!(with("z.svg", &[("z.svg", &big)]).find("agent-icon-size").is_some());
    let exact = format!("<svg>{}</svg>", " ".repeat(8 * 1024 - 11));
    assert_eq!(exact.len(), 8 * 1024);
    assert_eq!(with("z.svg", &[("z.svg", &exact)]).findings, []);
    assert!(
        with("z.svg", &[("z.svg", "<svg><script>1</script></svg>")]).find("agent-icon-forbidden").is_some()
    );
}

#[test]
fn hooks_need_their_permission_and_are_not_installable_yet() {
    let hooks = agent("status = \"hooks\"\n");
    assert!(has(&hooks, "agent-hooks-permission"));
    let granted = format!("[permissions]\nagents = [\"install-hooks:zork\"]\n{hooks}");
    let found = report(&granted);
    assert!(found.find("agent-hooks-permission").is_none());
    assert!(found.find("agent-hooks-unsupported").is_some());
    assert!(found.is_ok());
    assert!(has("[permissions]\nagents = [\"install-hooks:nobody\"]\n", "permission-agents"));
    assert!(
        has(&format!("[permissions]\nagents = [\"install-hooks:zork\"]\n{}", agent("")), "permission-agents"),
        "the agent must report through hooks"
    );
}

// ---- themes -------------------------------------------------------------------------------------

#[test]
fn contributed_theme_files_must_be_in_the_folder() {
    let body = "[[contributes.themes]]\nid = \"t\"\npath = \"themes/t.json\"\n";
    assert!(has(body, "contributed-file-missing"));
    assert!(has(&body.replace("themes/t.json", "../t.json"), "contributed-path"));
    let folder = probe(body, &[("themes/t.json", "{}")]);
    assert_eq!(validate(&folder).findings, []);
    assert!(has("[[contributes.icon_themes]]\nid = \"t\"\npath = \"nope\"\n", "contributed-file-missing"));
}

// ---- permissions and the summary ---------------------------------------------------------------

#[test]
fn permission_entries_are_checked() {
    assert!(has("[permissions]\nprocess = [\"a b\"]\n", "permission-process"));
    assert!(has("[permissions]\nprocess = [\"/bin/ls\"]\n", "permission-process"));
    assert!(has("[permissions]\nprocess = [\"./x.sh\"]\n", "permission-process-missing"));
    assert!(has("[permissions]\nprocess = [\"./../x.sh\"]\n", "permission-process"));
    assert!(has("[permissions]\nenv = [\"A-B\"]\n", "permission-env"));
    assert!(has("[permissions]\nenv = [\"WORMHOLE_SOCKET\"]\n", "permission-env-reserved"));
    assert!(has("[permissions]\nopen = [\"https://acme.net\"]\n", "permission-open"));
    assert!(has("[permissions]\nopen = [\"Acme.net\"]\n", "permission-open"));
}

#[test]
fn a_listed_program_nothing_runs_is_a_warning() {
    let found = report("[permissions]\nprocess = [\"say\"]\n");
    assert_eq!(found.findings.len(), 1);
    assert_eq!(found.findings[0].code, "permission-process-unused");
    assert_eq!(found.findings[0].severity, Severity::Warning);
    let found = report("[permissions]\nenv = [\"TOKEN\"]\n");
    assert_eq!(found.findings[0].code, "permission-env-unused");
}

#[test]
fn the_summary_lists_each_permission_with_how_it_is_confirmed() {
    let folder = probe(
        "[permissions]\nprocess = [\"git\", \"./go.sh\", \"bash\"]\nenv = [\"TOKEN\"]\nopen = [\"acme.net\"]\nnotify = true\n",
        &[("go.sh", "")],
    );
    let summary = validate(&folder).summary;
    let listed: Vec<_> =
        summary.permissions.iter().map(|p| (p.key, p.detail.as_str(), p.confirmation)).collect();
    assert_eq!(
        listed,
        [
            ("process", "git", Confirmation::TouchId),
            ("process", "./go.sh", Confirmation::TouchId),
            ("process", "bash", Confirmation::TouchId),
            ("env", "TOKEN", Confirmation::TouchId),
            ("open", "acme.net", Confirmation::Click),
            ("notify", "show notifications", Confirmation::Click),
        ]
    );
    let classes: Vec<_> = summary.programs.iter().map(|p| p.class).collect();
    assert_eq!(classes, [ProgramClass::ReadOnlyGit, ProgramClass::OwnScript, ProgramClass::RunsAnything]);
    assert_eq!(summary.combinations.len(), 1, "env with process");
    assert!(summary.combinations[0].description.contains("TOKEN"));
    assert!(!summary.has_code);
}

#[test]
fn reading_the_screen_with_a_way_out_is_a_named_combination() {
    let folder = probe(
        "main = \"i.ts\"\ncode_api = 0\n[permissions]\nterminal = [\"read\"]\nnet = [\"api.acme.dev\"]\nopen = [\"acme.net\"]\nprocess = [\"curl\"]\n",
        &[("i.ts", "")],
    );
    let found = validate(&folder);
    let said: Vec<_> = found.summary.combinations.iter().map(|c| c.description.as_str()).collect();
    assert_eq!(said.len(), 3, "{said:?}");
    assert!(said[0].contains("what is on your terminals") && said[0].contains("api.acme.dev"), "{said:?}");
    assert!(said[2].contains("anything"), "{said:?}");
    assert_eq!(found.summary.permissions[0].key, "main");
    assert!(found.summary.has_code);
    let alone = probe(
        "main = \"i.ts\"\ncode_api = 0\n[permissions]\nterminal = [\"write\"]\nnet = [\"api.acme.dev\"]\n",
        &[("i.ts", "")],
    );
    assert_eq!(validate(&alone).summary.combinations, [], "writing is not reading");
}

#[test]
fn findings_print_as_one_line_with_their_place() {
    let found = report(&rule("pattern = 'a*'\ntone = \"link\"\n"));
    let line = found.findings[0].to_string();
    assert!(line.starts_with("error[pattern-empty-match] contributes.rules[0].pattern: "), "{line}");
}

// ---- one spelling of a program ------------------------------------------------------------------

#[test]
fn git_is_git_whatever_the_case_and_its_other_programs_are_refused() {
    for name in ["GIT", "Git", "gIT"] {
        let found = report(&command(
            &format!("run = [\"{name}\", \"push\"]\nin = \"background\"\n"),
            &format!("\"{name}\""),
        ));
        assert!(found.find("run-program-name").is_some(), "{name}: {:?}", found.findings);
        assert!(found.find("permission-process").is_some(), "{name}: {:?}", found.findings);
        assert!(!found.is_ok());
        // Not even the git checks are skipped: the name is judged as `git` too.
        assert!(found.find("git-subcommand").is_some(), "{name}");
    }
    for name in ["git-upload-pack", "git-receive-pack", "git-shell", "git-upload-archive", "scalar"] {
        let found =
            report(&command(&format!("run = [\"{name}\"]\nin = \"background\"\n"), &format!("\"{name}\"")));
        let said = found.find("permission-process").unwrap_or_else(|| panic!("{name}: {:?}", found.findings));
        assert!(said.message.contains("`git`"), "{}", said.message);
        assert!(found.find("run-program-name").is_some(), "{name}");
    }
    // A bare name alone in permissions is refused too.
    assert!(has("[permissions]\nprocess = [\"GIT\"]\n", "permission-process"));
    // The same program is `git` for the summary.
    let summary = report("[permissions]\nprocess = [\"GIT\"]\n").summary;
    assert_eq!(summary.programs[0].class, ProgramClass::ReadOnlyGit);
}

// ---- text that hides itself -----------------------------------------------------------------------

#[test]
fn text_a_person_reads_cannot_hide_in_bidi_or_invisible_characters() {
    for bad in ["\u{202e}", "\u{200b}", "\u{2066}", "\u{feff}", "\u{2028}"] {
        let in_head = |field: &str| {
            let value =
                |name: &str, plain: &str| if name == field { format!("x{bad}y") } else { plain.to_string() };
            let text = HEAD
                .replace("name = \"Tiny\"", &format!("name = \"{}\"", value("name", "Tiny")))
                .replace("description = \"d\"", &format!("description = \"{}\"", value("description", "d")))
                .replace("author = \"a\"", &format!("author = \"{}\"", value("author", "a")));
            let parsed = Manifest::parse(&text).expect("parses");
            validate(&ProbeFolder { manifest: parsed.manifest, ..probe("", &[]) })
        };
        for field in ["name", "description", "author"] {
            assert!(in_head(field).find("probe-text-control").is_some(), "{field} {bad:?}");
        }
        let title = report(&format!(
            "[permissions]\nprocess = [\"say\"]\n[[contributes.commands]]\nid = \"c\"\ntitle = \"x{bad}y\"\nrun = [\"say\"]\nin = \"background\"\n"
        ));
        assert!(title.find("command-title").is_some(), "title {bad:?}");
        let description = report(&format!(
            "[permissions]\nprocess = [\"say\"]\n[[contributes.commands]]\nid = \"c\"\ntitle = \"t\"\ndescription = \"x{bad}y\"\nrun = [\"say\"]\nin = \"background\"\n"
        ));
        assert!(description.find("command-title").is_some(), "description {bad:?}");
        assert!(
            has(&rule(&format!("pattern = 'a+'\ntone = \"link\"\nhover = \"x{bad}y\"\n")), "hover-control"),
            "hover {bad:?}"
        );
        assert!(
            has(&agent("").replace("name = \"Zork\"", &format!("name = \"Zo{bad}rk\"")), "agent-name"),
            "agent {bad:?}"
        );
        assert!(
            has(&format!("[permissions]\nprocess = [\"sa{bad}y\"]\n"), "permission-process"),
            "process {bad:?}"
        );
        assert!(has(&format!("[permissions]\nenv = [\"TO{bad}KEN\"]\n"), "permission-env"), "env {bad:?}");
        assert!(
            has(&format!("[permissions]\nopen = [\"acme{bad}.net\"]\n"), "permission-open"),
            "open {bad:?}"
        );
        assert!(
            has(&agent("").replace("[\"zork\"]", &format!("[\"zo{bad}rk\"]")), "agent-executables"),
            "executable {bad:?}"
        );
    }
}

// ---- captures that reach an argument ------------------------------------------------------------

#[test]
fn a_capture_that_can_be_empty_or_absent_is_not_an_argument() {
    let run = "[\"git\", \"show\", \"$1\"]";
    for bad in ["([0-9a-f]{0,9})x", "(?:([0-9a-f]{7,9})|zz)", "(?:([0-9a-f]{7,9}))?x", "x(a*)"] {
        let found = report(&command_rule(bad, run));
        let said = found.find("capture-unanchored").unwrap_or_else(|| panic!("{bad}: {:?}", found.findings));
        assert!(said.message.contains("empty"), "{bad}: {}", said.message);
    }
    clean(&command_rule("x([0-9a-f]{7,9})", run));
}

// ---- agent packages ---------------------------------------------------------------------------------

#[test]
fn an_agent_package_is_long_and_overlaps_no_built_in_s() {
    let with = |packages: &str| agent(&format!("packages = [{packages}]\n"));
    clean(&with("\"@acme/zork-cli\""));
    for short in ["\"npm\"", "\"/a/\"", "\"1234\"", "\"\""] {
        assert!(has(&with(short), "agent-packages"), "{short}");
    }
    // A piece of Claude Code's package, a package that holds it, and one that is the same.
    for overlapping in ["\"anthropic-ai\"", "\"/.local/share/claude/versions/foo\"", "\"@openai/codex\""] {
        assert!(has(&with(overlapping), "agent-packages-builtin"), "{overlapping}");
    }
    assert!(has(&with("\"zork\u{202e}pkg\""), "agent-packages"));
}
