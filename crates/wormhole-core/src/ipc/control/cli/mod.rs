//! `Wormhole list | status | notify | send-text | new-terminal`: the command line's side of the
//! control protocol. The arguments are read into a [`Request`](super::Request) (`parse`), the app's [`Reply`] is
//! printed as text or, with `--json`, as the one JSON object it is (`render`). The round trip over the
//! socket is `ipc::exchange`'s. Folders are resolved here, against the caller's working directory
//! and home: the app cannot know where a script ran from.

use std::io::{self, Write};

use super::Action;

mod help;
mod parse;
mod render;

pub use help::help;
pub use parse::parse;
pub use render::render;

/// What the arguments asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub action: Action,
    /// Print the reply as JSON.
    pub json: bool,
    /// `send-text -`: the text is on stdin.
    pub stdin: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Run(Cli),
    Help,
}

/// What the command is called when someone types it: the name of the script that `cli_tool` puts in
/// `PATH` for Wormhole's terminals and, on request, for any terminal. One constant, so the script's
/// file name, the usage text, Settings and the docs' examples can't disagree.
pub const COMMAND_NAME: &str = "wh";

/// Whether `word` as the first argument is this command line's (`Wormhole list …`).
pub fn is_verb(word: &str) -> bool {
    matches!(
        word,
        "list" | "status" | "notify" | "send-text" | "new-terminal" | "sessions" | "help" | "--help" | "-h"
    )
}

/// Write to stdout as a command line does; a closed pipe (`| head`) is not an error worth saying.
pub(in crate::ipc) fn print(text: &str) {
    let _ = io::stdout().lock().write_all(text.as_bytes());
}

pub(in crate::ipc) fn print_err(text: &str) {
    let _ = io::stderr().lock().write_all(text.as_bytes());
}
