//! The report: facts in, problems with fixes out.

use serde::Serialize;

use super::{Caller, Facts};
use crate::agents::skill::State;
use crate::ipc::control::{COMMAND_NAME, PROTOCOL_VERSION};

/// How the app answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum App {
    Answered(Facts),
    /// It answered, but an error: this build does not know `doctor` (an older Wormhole).
    TooOld,
    /// It answered that it speaks another protocol.
    OtherProtocol(String),
    /// It did not answer; the words say why.
    Down(String),
}

/// One agent's skill, as the command line sees it in the files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillLine {
    /// The agent's word on the command line (`claude`).
    pub agent: String,
    pub name: String,
    /// Whether the agent's command is on this Mac's `PATH`.
    pub agent_found: bool,
    /// `not_installed`, `current`, `outdated` or `foreign`, or `error`.
    pub state: &'static str,
    pub path: String,
}

impl SkillLine {
    /// The word a state is reported as.
    pub fn state_word(state: &Result<State, String>) -> &'static str {
        match state {
            Ok(State::NotInstalled) => "not_installed",
            Ok(State::Current) => "current",
            Ok(State::Outdated) => "outdated",
            Ok(State::Foreign) => "foreign",
            Err(_) => "error",
        }
    }
}

/// How much a problem matters to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Something you want to do will not work until it is fixed.
    Problem,
    /// Worth knowing; nothing is broken.
    Hint,
}

/// Something to fix, and how.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// A stable word to act on.
    pub id: &'static str,
    pub severity: Severity,
    pub message: String,
    /// The exact step, and whose it is (usually the user's: a Settings switch).
    pub fix: String,
}

fn problem(id: &'static str, severity: Severity, message: &str, fix: &str) -> Problem {
    Problem { id, severity, message: message.to_string(), fix: fix.to_string() }
}

/// What `doctor` prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub app: AppLine,
    pub cli: CliLine,
    /// `None` when the app did not say.
    pub automation: Option<bool>,
    pub control_write: Option<bool>,
    pub control_read: Option<bool>,
    pub caller: Option<Caller>,
    /// `WORMHOLE_TAB_ID` as this process has it, when it is set.
    pub tab_id_in_environment: Option<String>,
    pub skills: Vec<SkillLine>,
    pub mcp: Mcp,
    pub problems: Vec<Problem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppLine {
    pub reachable: bool,
    pub version: Option<String>,
    pub protocol: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CliLine {
    pub version: &'static str,
    pub protocol: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mcp {
    /// The command that adds the MCP server to Claude Code, for the user to run.
    pub add_command: String,
}

/// The command that adds the MCP server to Claude Code. Nothing here edits another app's MCP file.
pub fn mcp_add_command() -> String {
    format!("claude mcp add wormhole -- {COMMAND_NAME} mcp")
}

const AUTOMATION_FIX: &str = "Ask the user to turn the Automation flag on: Settings → Feature flags → Automation (a development build or the Unstable update channel shows it; a Stable release needs `\"unlocked\": true` under feature_flags in config.json).";
const WRITE_FIX: &str =
    "Ask the user to turn on Settings → Coding agents → \"Let the command line type and open terminals\".";
const READ_FIX: &str = "Ask the user to turn on Settings → Coding agents → \"Let the command line read terminals\" (shown once the Automation flag is on).";

impl Report {
    /// Put what is known together. `tab` is `WORMHOLE_TAB_ID` as set in this process.
    pub fn build(app: App, tab: Option<String>, skills: Vec<SkillLine>) -> Self {
        let mut problems = Vec::new();
        let mut facts = None;
        let app_line = match &app {
            App::Answered(f) => {
                facts = Some(f);
                AppLine { reachable: true, version: Some(f.app_version.clone()), protocol: Some(f.protocol) }
            }
            App::TooOld | App::OtherProtocol(_) => AppLine { reachable: true, version: None, protocol: None },
            App::Down(_) => AppLine { reachable: false, version: None, protocol: None },
        };
        match &app {
            App::Down(why) => problems.push(problem(
                "app_unreachable",
                Severity::Problem,
                &format!("Wormhole does not answer: {why}."),
                "Ask the user to start Wormhole. Outside Wormhole's own terminals, `WORMHOLE_SOCKET` names the socket of a running instance; `wh list` and `wh status` still show terminals that keep running without the app.",
            )),
            App::TooOld => problems.push(problem(
                "app_too_old",
                Severity::Problem,
                "The running Wormhole is older than this command line and does not know `doctor`; most of the automation verbs will not work either.",
                "Ask the user to update Wormhole and restart it.",
            )),
            App::OtherProtocol(words) => problems.push(problem(
                "protocol_mismatch",
                Severity::Problem,
                words,
                "Ask the user to update Wormhole (or restart it, if it was updated while running).",
            )),
            App::Answered(f) if f.protocol != PROTOCOL_VERSION => problems.push(problem(
                "protocol_mismatch",
                Severity::Problem,
                &format!("The app speaks protocol {} and this command line {PROTOCOL_VERSION}.", f.protocol),
                "Ask the user to update Wormhole (or restart it, if it was updated while running).",
            )),
            App::Answered(_) => {}
        }
        if let Some(f) = facts {
            if !f.automation {
                problems.push(problem(
                    "automation_off",
                    Severity::Problem,
                    "The Automation flag is off: events, read, blocks, send-keys, mouse, windows and splits, wait and run answer `unsupported` (reason `flag_off`).",
                    AUTOMATION_FIX,
                ));
            }
            if !f.control_write {
                problems.push(problem(
                    "control_write_off",
                    Severity::Problem,
                    "Typing and opening are off: send-text, send-keys, mouse, run, new-terminal, new-window, split, focus, rename and close answer `not_allowed`.",
                    WRITE_FIX,
                ));
            }
            if !f.control_read {
                problems.push(problem(
                    "control_read_off",
                    Severity::Hint,
                    "Reading other terminals is off: read, blocks, wait --match/--gone, events --output and the output of run answer `not_allowed` or are left out. A terminal can always read itself.",
                    READ_FIX,
                ));
            }
            match &f.caller {
                None => problems.push(problem(
                    "not_in_a_terminal",
                    Severity::Hint,
                    "This command does not run in one of Wormhole's terminals, so there is no \"own terminal\": status and notify need --terminal, and nothing is refused as your own.",
                    "Name terminals with --terminal <id> from `wh list --json`.",
                )),
                Some(caller) => {
                    if !caller.verified {
                        problems.push(problem(
                            "own_terminal_unverified",
                            Severity::Hint,
                            &format!("WORMHOLE_TAB_ID names terminal {}, but this command does not run under that terminal's shell.", caller.terminal),
                            "Do not rely on being treated as that terminal's owner; name terminals with --terminal.",
                        ));
                    }
                    if caller.shell_integration == Some(false) {
                        problems.push(problem(
                            "no_shell_integration",
                            Severity::Hint,
                            "This terminal's shell has not reported command marks (OSC 133), so `run`, `blocks` and `wait --block-done` will answer `unsupported` (reason `no_integration`) here and in terminals started the same way.",
                            "Use send-text or send-keys with `wait --match` or `--stable` instead; the user can turn on the shell integration in Wormhole's terminal settings.",
                        ));
                    }
                }
            }
        }
        for line in &skills {
            let fix = format!(
                "Ask the user whether to install it: `{COMMAND_NAME} skill install --agent {}` (or Install in Settings → Coding agents).",
                line.agent
            );
            match line.state {
                "outdated" => problems.push(problem(
                    "skill_outdated",
                    Severity::Hint,
                    &format!("The Wormhole skill for {} is from another version of Wormhole.", line.name),
                    &fix.replace("install it", "update it"),
                )),
                "foreign" => problems.push(problem(
                    "skill_foreign",
                    Severity::Hint,
                    &format!("Another skill named wormhole is at {}; Wormhole leaves it alone.", line.path),
                    "Nothing to do unless the user wants it replaced: they remove that folder first.",
                )),
                "not_installed" if line.agent_found => problems.push(problem(
                    "skill_not_installed",
                    Severity::Hint,
                    &format!("{} is installed but has no Wormhole skill.", line.name),
                    &fix,
                )),
                _ => {}
            }
        }
        Report {
            app: app_line,
            cli: CliLine { version: env!("CARGO_PKG_VERSION"), protocol: PROTOCOL_VERSION },
            automation: facts.map(|f| f.automation),
            control_write: facts.map(|f| f.control_write),
            control_read: facts.map(|f| f.control_read),
            caller: facts.and_then(|f| f.caller.clone()),
            tab_id_in_environment: tab,
            skills,
            mcp: Mcp { add_command: mcp_add_command() },
            problems,
        }
    }

    /// Whether the app answered: the command line's exit code is 0 then, 3 otherwise.
    pub fn app_answered(&self) -> bool {
        self.app.reachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(automation: bool, write: bool, read: bool, caller: Option<Caller>) -> Facts {
        Facts {
            app_version: "1.3.0".into(),
            protocol: PROTOCOL_VERSION,
            automation,
            control_write: write,
            control_read: read,
            caller,
        }
    }

    fn ids(report: &Report) -> Vec<&'static str> {
        report.problems.iter().map(|p| p.id).collect()
    }

    fn own(verified: bool, integration: Option<bool>) -> Option<Caller> {
        Some(Caller { terminal: 3, verified, shell_integration: integration })
    }

    #[test]
    fn everything_on_in_a_terminal_with_integration_has_nothing_to_fix() {
        let report = Report::build(
            App::Answered(facts(true, true, true, own(true, Some(true)))),
            Some("3".into()),
            vec![],
        );
        assert!(report.problems.is_empty(), "{:?}", report.problems);
        assert!(report.app_answered());
        assert_eq!(report.automation, Some(true));
    }

    #[test]
    fn each_switch_that_is_off_says_which_one_and_who_turns_it_on() {
        let report =
            Report::build(App::Answered(facts(false, false, false, own(true, Some(true)))), None, vec![]);
        assert_eq!(ids(&report), ["automation_off", "control_write_off", "control_read_off"]);
        assert!(report.problems[1].fix.contains("Let the command line type and open terminals"));
        assert!(report.problems[2].fix.contains("Let the command line read terminals"));
        assert_eq!(report.problems[2].severity, Severity::Hint);
        assert!(report.problems[0].fix.starts_with("Ask the user"));
    }

    #[test]
    fn the_caller_is_told_about_what_it_cannot_do_from_where_it_runs() {
        let outside = Report::build(App::Answered(facts(true, true, true, None)), None, vec![]);
        assert_eq!(ids(&outside), ["not_in_a_terminal"]);
        let claimed =
            Report::build(App::Answered(facts(true, true, true, own(false, None))), Some("3".into()), vec![]);
        assert_eq!(ids(&claimed), ["own_terminal_unverified"]);
        let plain = Report::build(
            App::Answered(facts(true, true, true, own(true, Some(false)))),
            Some("3".into()),
            vec![],
        );
        assert_eq!(ids(&plain), ["no_shell_integration"]);
    }

    #[test]
    fn an_app_that_does_not_answer_still_gets_a_report_with_the_skills() {
        let line = SkillLine {
            agent: "claude".into(),
            name: "Claude Code".into(),
            agent_found: true,
            state: "outdated",
            path: "~/.claude/skills/wormhole".into(),
        };
        let report = Report::build(App::Down("Wormhole isn't running".into()), None, vec![line]);
        assert!(!report.app_answered());
        assert_eq!(ids(&report), ["app_unreachable", "skill_outdated"]);
        assert_eq!((report.automation, report.control_write, report.control_read), (None, None, None));
        assert!(report.problems[1].fix.contains("update it"));
    }

    #[test]
    fn skills_that_do_not_matter_are_not_problems() {
        let line = |agent_found, state| SkillLine {
            agent: "codex".into(),
            name: "Codex".into(),
            agent_found,
            state,
            path: "~/.agents/skills/wormhole".into(),
        };
        let quiet = Report::build(
            App::Answered(facts(true, true, true, own(true, Some(true)))),
            None,
            vec![line(false, "not_installed"), line(true, "current")],
        );
        assert!(quiet.problems.is_empty());
        let missing = Report::build(
            App::Answered(facts(true, true, true, own(true, Some(true)))),
            None,
            vec![line(true, "not_installed")],
        );
        assert_eq!(ids(&missing), ["skill_not_installed"]);
        assert!(missing.problems[0].fix.contains("skill install --agent codex"));
    }

    #[test]
    fn an_older_app_and_another_protocol_are_problems_of_their_own() {
        assert_eq!(ids(&Report::build(App::TooOld, None, vec![])), ["app_too_old"]);
        let mut other = facts(true, true, true, None);
        other.protocol = PROTOCOL_VERSION + 1;
        assert!(ids(&Report::build(App::Answered(other), None, vec![])).contains(&"protocol_mismatch"));
    }
}
