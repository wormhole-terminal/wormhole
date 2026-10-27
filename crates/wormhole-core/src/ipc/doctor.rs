//! `wh doctor`: ask the app how it stands, add what the command line can see without it (the skill in
//! each agent's folder), and print the report (`control::doctor`). The exit code is 0 when the app
//! answered and 3 when it did not, as for every verb; the report is printed either way.

use std::path::Path;

use super::control::doctor::{App, Facts, Report, SkillLine, render};
use super::control::printable::json_line;
use super::control::{
    Action, Error, ErrorCode, Parsed, Reply, Request, help_with, json_env, parse, print, print_err,
    show_automation,
};
use super::exchange::{Caller, answer_wait, exchange};
use crate::agents::skill;
use crate::agents::{self, integrations};

/// What each agent's skill folder holds, as `doctor` lines.
fn skills(home: &Path) -> Vec<SkillLine> {
    skill::states(home)
        .into_iter()
        .map(|(target, state)| SkillLine {
            agent: target.word(),
            name: agents::agent(target.agent).name.to_string(),
            agent_found: agents::installed(agents::agent(target.agent)),
            state: SkillLine::state_word(&state),
            path: target.display_path(home),
            covered_by: if state == Ok(skill::State::NotInstalled) {
                skill::covered_by(target, home)
            } else {
                None
            },
        })
        .collect()
}

/// How the app's reply reads.
fn classify(reply: Reply) -> App {
    match reply.error {
        Some(error) if error.code == ErrorCode::Unsupported => App::TooOld,
        Some(error) if error.code == ErrorCode::Version => App::OtherProtocol(error.message),
        Some(error) => App::Down(error.message),
        None => match reply.result.and_then(|result| serde_json::from_value::<Facts>(result).ok()) {
            Some(facts) => App::Answered(facts),
            None => App::Down("the app's answer could not be read".into()),
        },
    }
}

/// The report as the command line prints it.
fn shown(report: &Report, json: bool) -> String {
    if !json {
        return render(report);
    }
    let mut reply = Reply::ok(report);
    if !report.app_answered() {
        reply.ok = false;
        reply.error = Some(Error::new(ErrorCode::Failed, "Wormhole does not answer"));
    }
    format!("{}\n", json_line(&reply))
}

/// `wh doctor [--json]`: the exit code. (Here, above `exchange`, which it uses: the dispatch is
/// `run_control_cli`'s.)
pub(super) fn run(args: &[String]) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let json = match parse(args, &cwd, None) {
        Ok(Parsed::Run(cli)) => cli.wants_json(json_env().as_deref()),
        Ok(Parsed::Help | Parsed::HelpJson) => {
            print(&help_with(show_automation()));
            return 0;
        }
        Err(mistake) => {
            print_err(&format!("Wormhole: {mistake}\n\n{}", help_with(show_automation())));
            return 2;
        }
    };
    let Caller { socket, from } = Caller::from_env();
    let request = Request::new(Action::Doctor, from.clone());
    let app = match exchange(&socket, &request, answer_wait(&request.action)) {
        Ok(reply) => classify(reply),
        Err(why) => App::Down(why.words()),
    };
    let report = Report::build(app, from, skills(&integrations::home()));
    print(&shown(&report, json));
    if report.app_answered() { 0 } else { 3 }
}

#[cfg(test)]
mod tests {
    use super::{App, classify, shown};
    use crate::ipc::control::doctor::{Caller, Facts, Report};
    use crate::ipc::control::{Error, ErrorCode, PROTOCOL_VERSION, Reply};

    #[test]
    fn the_apps_reply_is_read_as_facts_or_as_why_not() {
        let facts = Facts {
            app_version: "1.3.0".into(),
            protocol: PROTOCOL_VERSION,
            automation: true,
            control_write: true,
            control_read: false,
            caller: Some(Caller { terminal: 4, verified: true, shell_integration: None }),
        };
        assert_eq!(classify(Reply::ok(&facts)), App::Answered(facts));
        assert_eq!(
            classify(Reply::error(ErrorCode::Unsupported, "this Wormhole doesn't know the action `doctor`")),
            App::TooOld
        );
        assert!(
            matches!(classify(Reply::error(ErrorCode::Version, "protocol 2")), App::OtherProtocol(words) if words == "protocol 2")
        );
        assert!(matches!(classify(Reply::ok(serde_json::json!({"nope": 1}))), App::Down(_)));
    }

    #[test]
    fn json_is_a_reply_whose_ok_says_whether_the_app_answered() {
        let down = Report::build(App::Down("Wormhole isn't running".into()), None, vec![]);
        let value: serde_json::Value = serde_json::from_str(shown(&down, true).trim()).expect("json");
        assert_eq!(value["ok"], false);
        assert_eq!(value["error"]["code"], "failed");
        assert_eq!(value["result"]["app"]["reachable"], false);
        assert_eq!(value["result"]["problems"][0]["id"], "app_unreachable");
        let _ = Error::new(ErrorCode::Failed, "");
    }
}
