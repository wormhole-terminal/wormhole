//! `wh skill install | uninstall | show [--agent <id>] [--path] [<file>]`: the Wormhole skill
//! (`agents::skill`) put in coding agents' skill folders, taken out again, or shown. Like `sessions` it
//! does not go through the app: it works from files, with the app closed too. Nothing here runs unless
//! a person (or an agent they asked) runs it; the app never installs the skill by itself.

use std::path::Path;

use serde_json::{Value, json};

use super::cli::{json_by_default, json_env, print, print_err};
use super::printable::json_line;
use super::{COMMAND_NAME, Reply};
use crate::agents::skill::{self, Done, State, TARGETS, Target};
use crate::agents::{self, integrations};

/// What `skill` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Install { agent: Option<String>, json: Option<bool> },
    Uninstall { agent: Option<String>, json: Option<bool> },
    Show { agent: Option<String>, path: bool, file: Option<String>, json: Option<bool> },
}

/// The usage text of the `skill` verb.
pub fn help(wh: &str) -> String {
    format!(
        "\
  {wh} skill install [--agent <id>]
      Put the Wormhole skill in a coding agent's skill folder (every agent found on this Mac without
      --agent), or update it there. Folders that are not Wormhole's are never touched.
  {wh} skill uninstall [--agent <id>]
      Take it out again. Files you added to its folder stay.
  {wh} skill show [<file>] | [--agent <id>] [--path]
      Print the skill (SKILL.md, or a file of references/); with --path or --agent where it goes and its
      state, for each agent or the one named.
"
    )
}

/// The agents' words, for the messages.
fn agent_words() -> String {
    TARGETS.iter().map(Target::word).collect::<Vec<_>>().join(", ")
}

/// Read the words after `skill`. `--json` and `--text` are `None` when neither is given.
pub fn parse(args: &[String]) -> Result<Command, String> {
    let (verb, rest) = args.split_first().ok_or("skill needs install, uninstall or show")?;
    let mut agent = None;
    let mut path = false;
    let mut json = None;
    let mut words: Vec<&str> = Vec::new();
    let mut rest = rest.iter().map(String::as_str);
    while let Some(arg) = rest.next() {
        match arg {
            "--agent" => {
                let value = rest.next().ok_or("--agent needs a value")?;
                if agent.replace(value.to_string()).is_some() {
                    return Err("name one agent with --agent".into());
                }
            }
            "--path" if verb == "show" => path = true,
            "--json" => json = Some(true),
            "--text" => json = Some(false),
            flag if flag.starts_with('-') => return Err(format!("unknown option {flag} for skill {verb}")),
            word => words.push(word),
        }
    }
    if let Some(word) = &agent {
        resolve(word)?;
    }
    match verb.as_str() {
        "install" | "uninstall" if !words.is_empty() => {
            Err(format!("skill {verb} takes options only, not {:?}", words[0]))
        }
        "install" => Ok(Command::Install { agent, json }),
        "uninstall" => Ok(Command::Uninstall { agent, json }),
        "show" => {
            let file = match words.as_slice() {
                [] => None,
                [one] if skill::file(one).is_some() => Some((*one).to_string()),
                [one] => {
                    let names = skill::FILES.iter().map(|f| f.path).collect::<Vec<_>>().join(", ");
                    return Err(format!("the skill has no file {one:?} ({names})"));
                }
                _ => return Err("skill show takes one file".into()),
            };
            if path && file.is_some() {
                return Err("skill show takes a file or --path, not both".into());
            }
            if agent.is_some() && file.is_some() {
                return Err("skill show takes a file or an --agent (whose folder it tells), not both".into());
            }
            Ok(Command::Show { agent, path, file, json })
        }
        other => Err(format!("unknown skill command {other:?} (install, uninstall, show)")),
    }
}

/// The agent a word names, or why it is none.
fn resolve(word: &str) -> Result<&'static Target, String> {
    skill::by_word(word).ok_or_else(|| match word.trim().to_ascii_lowercase().as_str() {
        // These read the shared folder, which is the Codex row's: say so rather than "unknown".
        "goose" | "pi" => format!(
            "{word} reads ~/.agents/skills, the folder of --agent codex: use that ({})",
            agent_words()
        ),
        _ => format!("no skill folder is known for {word:?} (agents that load skills: {})", agent_words()),
    })
}

/// `Wormhole skill …`: the exit code (0 done, 1 something could not be done, 2 a mistake in the arguments).
pub fn run(args: &[String]) -> i32 {
    let (code, out, err) =
        run_in(args, &integrations::home(), |agent| agents::installed(agents::agent(agent)));
    print(&out);
    print_err(&err);
    code
}

/// [`run`] against `home`, with `found` saying which agents are on this Mac: the exit code, what goes to
/// stdout and to stderr.
pub fn run_in(
    args: &[String],
    home: &Path,
    found: impl Fn(crate::agents::AgentId) -> bool,
) -> (i32, String, String) {
    let command = match parse(args) {
        Ok(command) => command,
        Err(mistake) => return (2, String::new(), format!("Wormhole: {mistake}\n\n{}", help(COMMAND_NAME))),
    };
    let env = json_env();
    let json = |asked: Option<bool>| asked.unwrap_or_else(|| json_by_default(env.as_deref()));
    match command {
        Command::Install { agent, json: asked } => change(Change::Install, agent, home, &found, json(asked)),
        Command::Uninstall { agent, json: asked } => {
            change(Change::Uninstall, agent, home, &found, json(asked))
        }
        // Where it goes, for every agent or the one named: also what `show --agent <id>` means.
        Command::Show { agent, path: true, json: asked, .. }
        | Command::Show { agent: agent @ Some(_), json: asked, .. } => where_to(agent, home, json(asked)),
        Command::Show { file, json: asked, .. } => show(file, json(asked)),
    }
}

fn show(file: Option<String>, json: bool) -> (i32, String, String) {
    let path = file.as_deref().unwrap_or("SKILL.md");
    let Some(file) = skill::file(path) else {
        return (2, String::new(), format!("Wormhole: no file {path}\n"));
    };
    if json {
        let files: Vec<_> = skill::FILES.iter().map(|f| f.path).collect();
        let reply = Reply::ok(json!({ "file": file.path, "text": file.text, "files": files }));
        return (0, format!("{}\n", json_line(&reply)), String::new());
    }
    (0, file.text.to_string(), String::new())
}

fn where_to(agent: Option<String>, home: &Path, json: bool) -> (i32, String, String) {
    let targets: Vec<&Target> = match agent.as_deref().map(resolve) {
        Some(Ok(target)) => vec![target],
        Some(Err(mistake)) => return (2, String::new(), format!("Wormhole: {mistake}\n")),
        None => TARGETS.iter().collect(),
    };
    let lines: Vec<Value> = targets
        .iter()
        .map(|target| {
            let state = skill::state(target, home);
            json!({
                "agent": target.word(),
                "name": agents::agent(target.agent).name,
                "path": target.display_path(home),
                "state": super::doctor::SkillLine::state_word(&state),
                "documented_at": target.source,
            })
        })
        .collect();
    if json {
        let reply = Reply::ok(json!({ "agents": lines }));
        return (0, format!("{}\n", json_line(&reply)), String::new());
    }
    let text: String = lines
        .iter()
        .map(|line| {
            format!(
                "{}: {} ({})\n",
                line["agent"].as_str().unwrap_or_default(),
                line["path"].as_str().unwrap_or_default(),
                line["state"].as_str().unwrap_or_default().replace('_', " ")
            )
        })
        .collect();
    (0, text, String::new())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Change {
    Install,
    Uninstall,
}

fn change(
    what: Change,
    agent: Option<String>,
    home: &Path,
    found: &impl Fn(crate::agents::AgentId) -> bool,
    json: bool,
) -> (i32, String, String) {
    let targets: Vec<&Target> = match agent.as_deref().map(resolve) {
        Some(Ok(target)) => vec![target],
        Some(Err(mistake)) => return (2, String::new(), format!("Wormhole: {mistake}\n")),
        // Install goes where an agent is; uninstall goes wherever the skill is ours.
        None if what == Change::Install => TARGETS.iter().filter(|t| found(t.agent)).collect(),
        None => TARGETS
            .iter()
            .filter(|t| matches!(skill::state(t, home), Ok(State::Current | State::Outdated)))
            .collect(),
    };
    if targets.is_empty() {
        let words = match what {
            Change::Install => format!(
                "no coding agent that loads skills was found on this Mac's PATH; name one with --agent ({})",
                agent_words()
            ),
            Change::Uninstall => "the skill is not installed".to_string(),
        };
        // Nothing to remove is not a failure; nothing to install into is.
        let (code, ok) = if what == Change::Uninstall { (0, true) } else { (1, false) };
        return (code, outcome(&[], ok, json, &words), String::new());
    }
    // Without --agent, an agent that reads a folder where this build's skill is already (installed just
    // before, for another agent: Codex's `~/.agents/skills` is read by most) is left out: a second copy
    // would only show twice. Named with --agent, it is installed in its own.
    let skip_covered = what == Change::Install && agent.is_none();
    let mut results: Vec<Line> = Vec::new();
    for target in targets {
        let covered = skip_covered.then(|| skill::covered_by(target, home)).flatten();
        let (word, message) = match (what, covered.is_some()) {
            (Change::Install, true) => ("covered", None),
            (Change::Install, false) => match skill::install(target, home) {
                Ok(Done::Installed) => ("installed", None),
                Ok(Done::Updated) => ("updated", None),
                Ok(Done::Unchanged) => ("unchanged", None),
                Err(why) => ("error", Some(why)),
            },
            (Change::Uninstall, _) => match skill::uninstall(target, home) {
                Ok(true) => ("removed", None),
                Ok(false) => ("not_installed", None),
                Err(why) => ("error", Some(why)),
            },
        };
        results.push(Line { target, word, message, covered });
    }
    let failed = results.iter().any(|line| line.word == "error");
    if json {
        let lines: Vec<Value> = results.iter().map(|line| line.json(home)).collect();
        return (i32::from(failed), outcome(&lines, !failed, true, ""), String::new());
    }
    let text: String = results.iter().map(|line| line.text(home)).collect();
    (i32::from(failed), text, String::new())
}

/// What became of one agent's skill.
struct Line {
    target: &'static Target,
    word: &'static str,
    message: Option<String>,
    /// The folder that covers the agent, when it was left out for that.
    covered: Option<String>,
}

impl Line {
    fn json(&self, home: &Path) -> Value {
        let mut line = json!({
            "agent": self.target.word(),
            "name": agents::agent(self.target.agent).name,
            "path": self.target.display_path(home),
            "result": self.word,
        });
        if let Some(message) = &self.message {
            line["message"] = json!(message);
        }
        if let Some(covered) = &self.covered {
            line["covered_by"] = json!(covered);
        }
        line
    }

    fn text(&self, home: &Path) -> String {
        let name = agents::agent(self.target.agent).name;
        match (&self.message, &self.covered) {
            (Some(why), _) => format!("{name}: {why}\n"),
            (None, Some(covered)) => format!("{name}: covered by {covered} (not installed again)\n"),
            (None, None) => {
                format!("{name}: {} ({})\n", self.word.replace('_', " "), self.target.display_path(home))
            }
        }
    }
}

/// The JSON reply (or the words) of a change.
fn outcome(lines: &[Value], ok: bool, json: bool, words: &str) -> String {
    if !json {
        return format!("{words}\n");
    }
    let mut reply = Reply::ok(json!({ "agents": lines }));
    if !ok {
        reply.ok = false;
        let message = if words.is_empty() { "the skill could not be changed for some agents" } else { words };
        reply.error = Some(super::Error::new(super::ErrorCode::Failed, message));
    }
    format!("{}\n", json_line(&reply))
}

#[cfg(test)]
mod tests {
    use super::{Command, parse, run_in};
    use crate::agents::AgentId;

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    fn run(line: &str, home: &std::path::Path, found: &[AgentId]) -> (i32, String, String) {
        run_in(&words(line), home, |agent| found.contains(&agent))
    }

    #[test]
    fn the_arguments_are_read_and_the_mistakes_are_told() {
        assert_eq!(parse(&words("install")), Ok(Command::Install { agent: None, json: None }));
        assert_eq!(
            parse(&words("uninstall --agent codex --json")),
            Ok(Command::Uninstall { agent: Some("codex".into()), json: Some(true) })
        );
        assert!(matches!(parse(&words("show --path")), Ok(Command::Show { path: true, .. })));
        assert!(matches!(
            parse(&words("show references/errors.md")),
            Ok(Command::Show { file: Some(_), .. })
        ));
        for bad in [
            "",
            "frobnicate",
            "install --agent",
            "install --agent nope",
            "install --agent a --agent b",
            "install now",
            "show nothing.md",
            "show --path SKILL.md",
            "install --path",
        ] {
            assert!(parse(&words(bad)).is_err(), "{bad:?}");
        }
        let goose = parse(&words("install --agent goose")).expect_err("goose has no folder of its own");
        assert!(goose.contains("--agent codex"), "{goose}");
    }

    #[test]
    fn install_goes_where_the_agent_is_and_uninstall_where_the_skill_is() {
        let home = tempfile::tempdir().expect("tempdir");
        let (code, out, _) = run("install", home.path(), &[AgentId::Claude]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Claude Code: installed"), "{out}");
        assert!(home.path().join(".claude/skills/wormhole/SKILL.md").is_file());
        assert!(!home.path().join(".agents").exists(), "Codex is not on this Mac");
        assert!(run("install", home.path(), &[AgentId::Claude]).1.contains("unchanged"));

        let (code, out, _) = run("install --agent codex", home.path(), &[]);
        assert_eq!(code, 0, "{out}");
        assert!(home.path().join(".agents/skills/wormhole/SKILL.md").is_file());

        let (code, out, _) = run("uninstall", home.path(), &[]);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Claude Code: removed") && out.contains("Codex CLI: removed"), "{out}");
        assert!(!home.path().join(".claude/skills/wormhole").exists());
        assert_eq!(run("uninstall", home.path(), &[]).0, 0, "nothing to remove is not a failure");
    }

    #[test]
    fn with_no_agent_found_install_says_so_and_a_foreign_skill_fails_alone() {
        let home = tempfile::tempdir().expect("tempdir");
        let (code, out, _) = run("install", home.path(), &[]);
        assert_eq!(code, 1);
        assert!(out.contains("name one with --agent"), "{out}");
        let ours = home.path().join(".gemini/skills/wormhole");
        std::fs::create_dir_all(&ours).expect("mkdir");
        std::fs::write(ours.join("SKILL.md"), "mine").expect("write");
        let (code, out, _) = run("install", home.path(), &[AgentId::Claude, AgentId::Gemini]);
        assert_eq!(code, 1);
        assert!(out.contains("Another skill is at"), "{out}");
        assert!(home.path().join(".claude/skills/wormhole/SKILL.md").is_file(), "the others went ahead");
        assert_eq!(std::fs::read_to_string(ours.join("SKILL.md")).expect("kept"), "mine");
    }

    #[test]
    fn show_prints_the_skill_or_where_it_goes_as_text_and_as_json() {
        let home = tempfile::tempdir().expect("tempdir");
        let (code, out, _) = run("show", home.path(), &[]);
        assert_eq!(code, 0);
        assert!(out.starts_with("---\nname: wormhole\n"));
        let (_, out, _) = run("show --json", home.path(), &[]);
        let value: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
        assert_eq!(value["result"]["file"], "SKILL.md");
        let (_, out, _) = run("show --path --agent claude", home.path(), &[]);
        assert!(out.starts_with("claude: ~/.claude/skills/wormhole (not installed)"), "{out}");
        let (_, out, _) = run("show --path --json", home.path(), &[]);
        let value: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
        assert!(value["result"]["agents"].as_array().expect("agents").len() >= 7);
        assert!(
            value["result"]["agents"][0]["documented_at"].as_str().expect("source").starts_with("https://")
        );
        assert_eq!(run("show --agent nope", home.path(), &[]).0, 2);
    }

    #[test]
    fn an_agent_whose_folder_is_read_by_another_is_left_out_unless_named() {
        let home = tempfile::tempdir().expect("tempdir");
        let found = [AgentId::Codex, AgentId::Gemini, AgentId::OpenCode];
        let (code, out, _) = run("install", home.path(), &found);
        assert_eq!(code, 0, "{out}");
        assert!(out.contains("Codex CLI: installed"), "{out}");
        assert!(out.contains("Gemini CLI: covered by ~/.agents/skills"), "{out}");
        assert!(out.contains("OpenCode: covered by ~/.agents/skills"), "{out}");
        assert!(!home.path().join(".gemini").exists() && !home.path().join(".config").exists());
        let (_, json, _) = run("install --json", home.path(), &found);
        let value: serde_json::Value = serde_json::from_str(json.trim()).expect("json");
        assert_eq!(value["result"]["agents"][1]["covered_by"], "~/.agents/skills");
        // Named, it goes in its own folder too.
        let (code, out, _) = run("install --agent gemini", home.path(), &found);
        assert_eq!(code, 0, "{out}");
        assert!(home.path().join(".gemini/skills/wormhole/SKILL.md").is_file());
    }

    #[test]
    fn show_with_an_agent_tells_that_agents_folder_and_not_the_skill() {
        let home = tempfile::tempdir().expect("tempdir");
        let (code, out, _) = run("show --agent codex", home.path(), &[]);
        assert_eq!(code, 0);
        assert!(out.starts_with("codex: ~/.agents/skills/wormhole (not installed)"), "{out}");
        assert!(parse(&words("show SKILL.md --agent codex")).is_err());
    }
}
