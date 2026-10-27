//! The report for a person (or a model reading a terminal): a few lines, then the problems.

use super::Report;

fn on(state: Option<bool>) -> &'static str {
    match state {
        Some(true) => "on",
        Some(false) => "OFF",
        None => "unknown",
    }
}

/// The report as text.
pub fn render(report: &Report) -> String {
    let mut out = String::new();
    match (&report.app.version, report.app.reachable) {
        (Some(version), _) => out.push_str(&format!(
            "Wormhole {version} answers (protocol {}; this command line speaks {}).\n",
            report.app.protocol.unwrap_or_default(),
            report.cli.protocol
        )),
        (None, true) => out.push_str("Wormhole answers, but an older build.\n"),
        (None, false) => out.push_str("Wormhole does not answer.\n"),
    }
    out.push_str(&format!("Automation flag: {}\n", on(report.automation)));
    match &report.access {
        Some(access) => out.push_str(&format!(
            "Agent access: {}. You are {}: {}\n",
            access.mode.word(),
            access.who,
            access.standing.words()
        )),
        None => {
            out.push_str(&format!("Typing and opening (control_write): {}\n", on(report.control_write)));
            out.push_str(&format!("Reading other terminals (control_read): {}\n", on(report.control_read)));
        }
    }
    match &report.caller {
        Some(caller) => out.push_str(&format!(
            "This terminal: {}{}; shell integration: {}\n",
            caller.terminal,
            if caller.verified { "" } else { " (named by WORMHOLE_TAB_ID, not confirmed)" },
            match caller.shell_integration {
                Some(true) => "seen",
                Some(false) => "not seen",
                None => "its shell has ended",
            }
        )),
        None if report.app.version.is_some() => out.push_str("This terminal: not one of Wormhole's\n"),
        None => {}
    }
    for skill in &report.skills {
        if let Some(covered) = &skill.covered_by {
            out.push_str(&format!("Skill for {}: covered by {covered}\n", skill.name));
        } else if skill.agent_found || skill.state != "not_installed" {
            out.push_str(&format!(
                "Skill for {}: {} ({})\n",
                skill.name,
                skill.state.replace('_', " "),
                skill.path
            ));
        }
    }
    out.push_str(&format!("MCP: {}\n", report.mcp.add_command));
    if report.problems.is_empty() {
        out.push_str("\nNothing to fix.\n");
    } else {
        out.push_str("\nTo fix:\n");
        for (at, problem) in report.problems.iter().enumerate() {
            out.push_str(&format!(
                "{}. {} [{}]\n   Fix: {}\n",
                at + 1,
                problem.message,
                problem.id,
                problem.fix
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::{Access, App, Caller, Facts, Report, Standing, render};
    use crate::config::AgentAccess;
    use crate::ipc::control::PROTOCOL_VERSION;

    #[test]
    fn the_text_names_each_switch_and_each_fix() {
        let facts = Facts {
            app_version: "1.3.0".into(),
            protocol: PROTOCOL_VERSION,
            automation: true,
            control_write: false,
            control_read: true,
            caller: Some(Caller { terminal: 3, verified: true, shell_integration: Some(true) }),
            access: None,
        };
        let shown = render(&Report::build(App::Answered(facts), Some("3".into()), vec![]));
        assert!(shown.contains("Wormhole 1.3.0 answers"), "{shown}");
        assert!(shown.contains("Typing and opening (control_write): OFF"));
        assert!(shown.contains("Reading other terminals (control_read): on"));
        assert!(shown.contains("This terminal: 3; shell integration: seen"));
        assert!(shown.contains("Fix: Ask the user to turn on Settings → Coding agents"));
        let with_access = Facts {
            access: Some(Access {
                mode: AgentAccess::Ask,
                who: "Claude Code".into(),
                standing: Standing::UntilQuit,
            }),
            ..Facts {
                app_version: "1.3.0".into(),
                protocol: PROTOCOL_VERSION,
                automation: true,
                control_write: false,
                control_read: false,
                caller: None,
                access: None,
            }
        };
        let shown = render(&Report::build(App::Answered(with_access), None, vec![]));
        assert!(
            shown.contains("Agent access: ask. You are Claude Code: allowed until Wormhole quits"),
            "{shown}"
        );
        assert!(!shown.contains("control_write"), "the switches are replaced while the flag is on: {shown}");
        let down = render(&Report::build(App::Down("Wormhole isn't running".into()), None, vec![]));
        assert!(down.starts_with("Wormhole does not answer."));
    }
}
