//! Claude Code's `statusLine` entry, which hands Wormhole the plan usage limits (`usage`).
//!
//! The file has one status line, and it may be the user's own. Ours keeps theirs running: it
//! reads the status JSON once, passes it to `$WORMHOLE_BIN statusline` in the background
//! (inside Wormhole only), then pipes it to the user's command, kept verbatim at the end of
//! ours, so the line looks as it did. Uninstalling puts their command back as it was; without
//! one, the entry goes.

use serde_json::{Value, json};

/// In every status line command we install; how we recognise ours.
const MARKER: &str = "\"$WORMHOLE_BIN\" statusline";
/// The same in the status line versions up to 0.2 installed, under the app's old codename.
const LEGACY_MARKER: &str = "\"$ANDROMEDA_BIN\" statusline";
/// What comes before the user's command (single-quoted) in ours.
const CHAIN: &str = "| eval ";

/// The status line command, chaining `original` when the user had one.
fn command(original: Option<&str>) -> String {
    let forward = format!(
        "input=$(cat); [ -n \"$WORMHOLE_BIN\" ] && printf '%s' \"$input\" | {MARKER} >/dev/null 2>&1 &"
    );
    match original {
        Some(original) => format!("{forward} printf '%s' \"$input\" {CHAIN}{}", quote(original)),
        None => format!("{forward} true"),
    }
}

/// `text` as one single-quoted shell word.
fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The word [`quote`] made, back as it was; `None` if `word` isn't one.
fn unquote(word: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = word;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix(r"\'") {
            out.push('\'');
            rest = tail;
        } else {
            let inner = rest.strip_prefix('\'')?;
            let end = inner.find('\'')?;
            out.push_str(&inner[..end]);
            rest = &inner[end + 1..];
        }
    }
    Some(out)
}

fn is_ours(command: &str) -> bool {
    command.contains(MARKER) || command.contains(LEGACY_MARKER)
}

/// The user's command inside ours, if ours chains one.
fn chained(command: &str) -> Option<String> {
    let at = command.find(CHAIN)?;
    unquote(&command[at + CHAIN.len()..])
}

fn current(settings: &Value) -> Option<&str> {
    settings.get("statusLine")?.get("command")?.as_str()
}

pub(super) fn installed(settings: &Value) -> bool {
    current(settings).is_some_and(|command| command.contains(MARKER))
}

/// Ours, as an earlier version installed it under the old names.
pub(super) fn has_legacy(settings: &Value) -> bool {
    current(settings).is_some_and(|command| command.contains(LEGACY_MARKER))
}

/// Add ours, around the user's command if there is one. A status line of another kind than a
/// command is left alone (`false`: nothing to chain onto).
pub(super) fn install(settings: &mut Value) -> Result<bool, String> {
    let obj = settings.as_object_mut().ok_or("the settings file is not a JSON object")?;
    match obj.get_mut("statusLine") {
        None => {
            obj.insert(
                "statusLine".into(),
                json!({ "type": "command", "command": command(None), "padding": 0 }),
            );
            Ok(true)
        }
        Some(line) => match line.get("command").and_then(Value::as_str) {
            // An earlier version's: today's, around the same command of the user's.
            Some(existing) if existing.contains(LEGACY_MARKER) => {
                let upgraded = command(chained(existing).as_deref());
                line["command"] = Value::String(upgraded);
                Ok(true)
            }
            Some(existing) if is_ours(existing) => Ok(false),
            Some(existing) => {
                let chained = command(Some(existing));
                line["command"] = Value::String(chained);
                Ok(true)
            }
            None => Ok(false),
        },
    }
}

/// Take ours out: the user's command back, or no status line if they had none.
pub(super) fn uninstall(settings: &mut Value) -> bool {
    let Some(existing) = current(settings).filter(|c| is_ours(c)).map(str::to_string) else { return false };
    match (chained(&existing), settings.as_object_mut()) {
        (Some(original), _) => settings["statusLine"]["command"] = Value::String(original),
        (None, Some(obj)) => {
            obj.shift_remove("statusLine");
        }
        (None, None) => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{chained, command, install, installed, quote, uninstall, unquote};

    #[test]
    fn quoting_round_trips_any_command() {
        for text in ["~/.claude/statusline.sh", "jq -r '.model.display_name'", "echo \"it's $HOME\"", "", "'"]
        {
            assert_eq!(unquote(&quote(text)).as_deref(), Some(text), "{text}");
        }
        assert_eq!(unquote("not quoted"), None);
    }

    #[test]
    fn our_command_keeps_the_users_verbatim() {
        let original = "bash ~/.claude/line.sh | sed 's/a/b/'";
        assert_eq!(chained(&command(Some(original))).as_deref(), Some(original));
        assert_eq!(chained(&command(None)), None);
    }

    #[test]
    fn with_no_status_line_ours_is_added_and_removed() {
        let mut settings = json!({ "hooks": {} });
        assert_eq!(install(&mut settings), Ok(true));
        assert!(installed(&settings));
        assert_eq!(settings["statusLine"]["type"], "command");
        assert_eq!(install(&mut settings), Ok(false), "idempotent");
        assert!(uninstall(&mut settings));
        assert_eq!(settings, json!({ "hooks": {} }));
    }

    #[test]
    fn the_users_status_line_is_chained_and_restored() {
        let theirs = json!({ "type": "command", "command": "~/.claude/statusline.sh", "padding": 2 });
        let mut settings = json!({ "statusLine": theirs.clone() });
        assert_eq!(install(&mut settings), Ok(true));
        assert!(installed(&settings));
        assert_eq!(settings["statusLine"]["padding"], 2, "their other fields stay");
        assert!(uninstall(&mut settings));
        assert_eq!(settings, json!({ "statusLine": theirs }));
        assert!(!uninstall(&mut settings), "nothing of ours left");
    }

    #[test]
    fn a_status_line_without_a_command_is_left_alone() {
        let mut settings = json!({ "statusLine": { "type": "static" } });
        assert_eq!(install(&mut settings), Ok(false));
        assert!(!installed(&settings));
    }

    /// What Claude Code does with the entry: run it through a shell with the status JSON on stdin.
    fn run(command: &str, bin: Option<&std::path::Path>) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut sh = Command::new("/bin/sh");
        sh.args(["-c", command]).env_remove("WORMHOLE_BIN").stdin(Stdio::piped()).stdout(Stdio::piped());
        if let Some(bin) = bin {
            sh.env("WORMHOLE_BIN", bin);
        }
        let mut child = sh.spawn().expect("sh");
        child.stdin.take().expect("stdin").write_all(br#"{"model":"opus"}"#).expect("write");
        String::from_utf8(child.wait_with_output().expect("output").stdout).expect("utf-8")
    }

    /// In a real shell: the user's command gets the JSON and prints the line, quotes and all;
    /// inside Wormhole the app gets the same JSON and prints nothing; outside it nothing else runs.
    #[test]
    fn in_a_shell_the_users_line_prints_and_wormhole_gets_the_json() {
        let dir = tempfile::tempdir().expect("temp dir");
        let got = dir.path().join("forwarded.json");
        let bin = dir.path().join("wormhole");
        std::fs::write(
            &bin,
            format!("#!/bin/sh\n[ \"$1\" = statusline ] && cat > '{}'\necho noise\n", got.display()),
        )
        .expect("write the stand-in");
        std::fs::set_permissions(&bin, crate::platform::fs::permissions(0o755)).expect("chmod");

        let theirs = r#"m=$(sed 's/.*"model":"\([a-z]*\)".*/\1/'); echo "$m: it's mine""#;
        let ours = command(Some(theirs));
        assert_eq!(run(&ours, None), "opus: it's mine\n", "outside Wormhole: their line alone");
        assert!(!got.exists());

        assert_eq!(run(&ours, Some(&bin)), "opus: it's mine\n", "inside: still their line, no noise");
        // The forwarding runs in the background: give it a moment.
        for _ in 0..50 {
            if got.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(std::fs::read_to_string(&got).expect("forwarded"), r#"{"model":"opus"}"#);

        assert_eq!(run(&command(None), Some(&bin)), "", "no line of their own: an empty one");
    }

    /// The forwarding runs in the background with its output gone, so the user's command alone
    /// prints the line, and it does nothing outside Wormhole.
    #[test]
    fn the_command_forwards_quietly_and_only_inside_wormhole() {
        let ours = command(Some("echo hi"));
        assert!(ours.starts_with("input=$(cat); [ -n \"$WORMHOLE_BIN\" ] && "));
        assert!(ours.contains("\"$WORMHOLE_BIN\" statusline >/dev/null 2>&1 &"));
        assert!(ours.ends_with("printf '%s' \"$input\" | eval 'echo hi'"));
    }
}
