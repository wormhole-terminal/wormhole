//! `Wormhole list | status | notify | send-text | new-terminal | events | read | blocks | send-keys |
//! mouse | new-window | split | focus | rename | close | wait | run | doctor`: the command line's side of the
//! control protocol. The arguments are read into a [`Request`](super::Request) (`parse`), the app's
//! [`Reply`] is printed as text or, with `--json` (or `WORMHOLE_JSON`, `--text` to override it), as the one JSON
//! object it is (`render`). The round trip over the
//! socket is `ipc::exchange`'s. Folders are resolved here, against the caller's working directory
//! and home: the app cannot know where a script ran from.

use std::io::{self, Write};

use super::Action;

pub(in crate::ipc) mod events;
mod help;
mod help_json;
mod parse;
#[cfg(test)]
mod parse_tests;
mod read_text;
mod render;
mod structure_text;
pub(in crate::ipc) mod verbs;
mod wait_text;

pub use help::{help, help_with};
pub use help_json::help_json;
pub use parse::parse;
pub(in crate::ipc) use parse::resolve_folder;
pub use render::{render, render_with};
pub(in crate::ipc) use wait_text::run_parts;

/// What the arguments asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub action: Action,
    /// Print the reply as JSON.
    pub json: bool,
    /// Print it as text, whatever `WORMHOLE_JSON` says.
    pub text: bool,
    /// `send-text -`: the text is on stdin.
    pub stdin: bool,
    /// `read --numbers`: a gutter of line numbers in the text output.
    pub numbers: bool,
    /// `run --exit-code`: exit with the command's own exit code, for shell scripts.
    pub exit_code: bool,
}

impl Cli {
    /// Whether to print JSON: `--json`, or `WORMHOLE_JSON` (`env`: its value) turning it on for every
    /// verb, unless `--text` asks for text.
    pub fn wants_json(&self, env: Option<&str>) -> bool {
        self.json || (!self.text && json_by_default(env))
    }
}

/// Whether the value of `WORMHOLE_JSON` makes JSON the default output: set and not a "no".
pub(super) fn json_by_default(env: Option<&str>) -> bool {
    env.is_some_and(|value| {
        let value = value.trim();
        !value.is_empty() && !["0", "false", "no", "off"].iter().any(|no| value.eq_ignore_ascii_case(no))
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Run(Cli),
    Help,
    /// `help --json`: the verbs, flags and codes as data.
    HelpJson,
}

/// What the command is called when someone types it: the name of the script that `cli_tool` puts in
/// `PATH` for Wormhole's terminals and, on request, for any terminal. One constant, so the script's
/// file name, the usage text, Settings and the docs' examples can't disagree.
pub const COMMAND_NAME: &str = "wh";

/// Whether `word` as the first argument is this command line's (`Wormhole list …`): a verb of the table
/// (`verbs`), or a way to ask for the usage.
pub fn is_verb(word: &str) -> bool {
    // `probe` is behind the Probes flag and in no release yet, so the table (and with it `help --json` and
    // the skill agents learn the command line from) leaves it out until it ships.
    verbs::find(word).is_some() || matches!(word, "probe" | "--help" | "-h")
}

/// `WORMHOLE_JSON` as it is set in this process.
pub(in crate::ipc) fn json_env() -> Option<String> {
    std::env::var("WORMHOLE_JSON").ok()
}

/// Write to stdout as a command line does; a closed pipe (`| head`) is not an error worth saying.
pub(in crate::ipc) fn print(text: &str) {
    let _ = io::stdout().lock().write_all(text.as_bytes());
}

pub(in crate::ipc) fn print_err(text: &str) {
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

/// Whether the usage names the verbs behind the automation flag: the flag as the Default profile's
/// `config.json` has it, read here without opening the app (and without creating the file, which
/// loading it would: a command line that only prints help leaves nothing behind).
pub(in crate::ipc) fn show_automation() -> bool {
    use crate::config::Config;
    use crate::config::features::FeatureFlag;
    if !Config::path().exists() {
        return crate::config::Settings::default().feature_enabled(FeatureFlag::Automation);
    }
    Config::load().settings.feature_enabled(FeatureFlag::Automation)
}

#[cfg(test)]
mod tests {
    use super::{Cli, json_by_default};
    use crate::ipc::control::Action;

    fn cli(json: bool, text: bool) -> Cli {
        Cli { action: Action::List, json, text, stdin: false, numbers: false, exit_code: false }
    }

    #[test]
    fn json_is_a_flag_or_the_environments_default_and_text_overrides_it() {
        assert!(!cli(false, false).wants_json(None));
        assert!(cli(true, false).wants_json(None));
        assert!(cli(false, false).wants_json(Some("1")));
        assert!(!cli(false, true).wants_json(Some("1")), "--text wins over the environment");
        assert!(cli(true, false).wants_json(Some("0")), "--json wins over it too");
    }

    #[test]
    fn the_environments_value_is_read_as_a_switch() {
        for on in ["1", "true", "yes", "json", " 1 "] {
            assert!(json_by_default(Some(on)), "{on:?}");
        }
        for off in ["", "0", "false", "No", "OFF", "  "] {
            assert!(!json_by_default(Some(off)), "{off:?}");
        }
        assert!(!json_by_default(None));
    }
}
