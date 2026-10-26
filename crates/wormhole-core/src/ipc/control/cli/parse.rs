//! The arguments, read into the request they ask for.

use std::path::{Path, PathBuf};

use super::{Cli, Parsed};
use crate::ipc::control::{Action, Target};

/// Read the words after the program name (`list --json`, `send-text --tab 3 hello`) into what to
/// do. `cwd` and `home` resolve a relative folder and a leading `~`.
pub fn parse(args: &[String], cwd: &Path, home: Option<&Path>) -> Result<Parsed, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Ok(Parsed::Help),
    };
    if matches!(verb, "help" | "--help" | "-h") {
        return Ok(Parsed::Help);
    }
    let mut json = false;
    let mut text_out = false;
    let mut target = None;
    let mut title = None;
    let mut submit = false;
    let mut folder = None;
    let mut command = None;
    let mut worktree = None;
    let mut window = None;
    let mut words: Vec<&str> = Vec::new();
    let mut args = rest.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().ok_or_else(|| format!("{flag} needs a value"));
        let id = |flag: &str, value: &str| {
            value.parse::<i64>().map_err(|_| format!("{flag} takes an id (a number), not {value:?}"))
        };
        match arg {
            "--json" => json = true,
            "--text" => text_out = true,
            "--help" | "-h" => return Ok(Parsed::Help),
            "--terminal" | "--tab" | "--pane" => {
                let value = value(arg)?;
                let id = id(arg, value)?;
                let one = match arg {
                    "--terminal" => Target::Terminal(id),
                    "--tab" => Target::Tab(id),
                    _ => Target::Pane(id),
                };
                if target.replace(one).is_some() {
                    return Err("name one target: --terminal, --tab or --pane".into());
                }
            }
            "--title" if verb == "notify" => title = Some(value(arg)?.to_string()),
            "--submit" if verb == "send-text" => submit = true,
            "--folder" if verb == "new-terminal" => folder = Some(value(arg)?.to_string()),
            "--command" if verb == "new-terminal" => command = Some(value(arg)?.to_string()),
            "--worktree" if verb == "new-terminal" => worktree = Some(value(arg)?.to_string()),
            "--window" if verb == "new-terminal" => {
                let value = value(arg)?;
                window = Some(id(arg, value)?);
            }
            flag if flag.starts_with('-') && flag != "-" => {
                return Err(format!("unknown option {flag} for {verb}"));
            }
            word => words.push(word),
        }
    }
    if json && text_out {
        return Err("--json and --text can't be combined".into());
    }
    let text = words.join(" ");
    let action = match verb {
        "list" => Action::List,
        "status" => Action::Status { target },
        "notify" => {
            if text.trim().is_empty() && title.as_deref().is_none_or(|t| t.trim().is_empty()) {
                return Err("notify needs something to say: a body, or --title".into());
            }
            Action::Notify { target, title: title.unwrap_or_default(), body: text }
        }
        "send-text" => {
            let Some(target) = target else {
                return Err("send-text needs a target: --terminal, --tab or --pane".into());
            };
            if text.is_empty() {
                return Err("send-text needs the text to type (or - for stdin)".into());
            }
            Action::SendText { target, text, submit }
        }
        "new-terminal" => {
            if !text.is_empty() {
                return Err(format!(
                    "new-terminal takes options only, not {text:?} (did you mean --folder?)"
                ));
            }
            let folder = folder.map(|folder| resolve_folder(&folder, cwd, home));
            Action::NewTerminal { folder, command, worktree, window }
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    if target.is_some() && matches!(action, Action::List | Action::NewTerminal { .. }) {
        return Err(format!("{verb} takes no target"));
    }
    let stdin = matches!(&action, Action::SendText { text, .. } if text == "-");
    Ok(Parsed::Run(Cli { action, json, text: text_out, stdin }))
}

/// `folder` as an absolute path: `~` is `home`, a relative path is under `cwd`. Nothing is
/// checked on disk here: the app says whether the folder exists.
fn resolve_folder(folder: &str, cwd: &Path, home: Option<&Path>) -> String {
    let path = match (folder, home) {
        ("~", Some(home)) => home.to_path_buf(),
        (folder, Some(home)) if folder.starts_with("~/") => home.join(&folder[2..]),
        (folder, _) if Path::new(folder).is_absolute() => PathBuf::from(folder),
        (folder, _) => cwd.join(folder),
    };
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Cli, Parsed, parse};
    use crate::ipc::control::{Action, Target};

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn parsed(list: &[&str]) -> Cli {
        match parse(&words(list), Path::new("/work/here"), Some(Path::new("/Users/me"))) {
            Ok(Parsed::Run(cli)) => cli,
            other => panic!("{list:?}: {other:?}"),
        }
    }

    fn mistake(list: &[&str]) -> String {
        match parse(&words(list), Path::new("/work/here"), Some(Path::new("/Users/me"))) {
            Err(mistake) => mistake,
            other => panic!("{list:?} should be refused: {other:?}"),
        }
    }

    #[test]
    fn every_verb_parses_with_its_options() {
        assert_eq!(parsed(&["list"]), Cli { action: Action::List, json: false, text: false, stdin: false });
        assert!(parsed(&["list", "--json"]).json);
        assert_eq!(parsed(&["status"]).action, Action::Status { target: None });
        assert_eq!(
            parsed(&["status", "--terminal", "3"]).action,
            Action::Status { target: Some(Target::Terminal(3)) }
        );
        assert_eq!(
            parsed(&["notify", "--tab", "9", "--title", "Deploy", "done", "in", "3s"]).action,
            Action::Notify {
                target: Some(Target::Tab(9)),
                title: "Deploy".into(),
                body: "done in 3s".into()
            }
        );
        assert_eq!(
            parsed(&["notify", "--title", "Ready"]).action,
            Action::Notify { target: None, title: "Ready".into(), body: String::new() },
            "a title alone will do"
        );
        assert_eq!(
            parsed(&["send-text", "--pane", "4", "--submit", "run", "the", "tests"]).action,
            Action::SendText { target: Target::Pane(4), text: "run the tests".into(), submit: true }
        );
        let piped = parsed(&["send-text", "--tab", "2", "-"]);
        assert!(piped.stdin, "- reads stdin");
        assert_eq!(
            parsed(&[
                "new-terminal",
                "--folder",
                "~/src/app",
                "--command",
                "claude",
                "--window",
                "2",
                "--worktree",
                "fix"
            ])
            .action,
            Action::NewTerminal {
                folder: Some("/Users/me/src/app".into()),
                command: Some("claude".into()),
                worktree: Some("fix".into()),
                window: Some(2),
            }
        );
        assert_eq!(
            parsed(&["new-terminal", "--folder", "api"]).action,
            Action::NewTerminal {
                folder: Some("/work/here/api".into()),
                command: None,
                worktree: None,
                window: None
            },
            "relative to where the command ran"
        );
        assert_eq!(
            parsed(&["new-terminal", "--folder", "~"]).action,
            Action::NewTerminal {
                folder: Some("/Users/me".into()),
                command: None,
                worktree: None,
                window: None
            }
        );
    }

    #[test]
    fn the_output_format_is_chosen_by_a_flag() {
        assert!(parsed(&["list", "--text"]).text);
        assert!(!parsed(&["list"]).text);
        assert!(mistake(&["list", "--json", "--text"]).contains("can't be combined"));
    }

    #[test]
    fn help_is_asked_for_in_every_usual_way() {
        for list in [&[][..], &["help"], &["--help"], &["-h"], &["list", "--help"], &["send-text", "-h"]] {
            assert_eq!(parse(&words(list), Path::new("/"), None).expect("parses"), Parsed::Help, "{list:?}");
        }
        let help = crate::ipc::control::help();
        assert!(
            help.contains(&format!("{} list", crate::ipc::control::COMMAND_NAME)) && help.contains("--json")
        );
        // The usage names the command people type, never the binary's own name for it.
        assert!(!help.contains("Wormhole list") && !help.contains("Wormhole send-text"));
    }

    #[test]
    fn mistakes_are_named() {
        assert!(mistake(&["frobnicate"]).contains("unknown command"));
        assert!(mistake(&["send-text", "hello"]).contains("needs a target"));
        assert!(mistake(&["send-text", "--tab", "3"]).contains("needs the text"));
        assert!(mistake(&["send-text", "--tab", "x", "hi"]).contains("takes an id"));
        assert!(mistake(&["send-text", "--tab", "3", "--pane", "4", "hi"]).contains("one target"));
        assert!(mistake(&["notify"]).contains("something to say"));
        assert!(mistake(&["notify", "--title"]).contains("needs a value"));
        assert!(mistake(&["list", "--tab", "3"]).contains("takes no target"));
        assert!(mistake(&["new-terminal", "/work/app"]).contains("--folder"));
        assert!(mistake(&["list", "--submit"]).contains("unknown option"), "an option of another verb");
        assert!(mistake(&["status", "--bogus"]).contains("unknown option"));
    }
}
