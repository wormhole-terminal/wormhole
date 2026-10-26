//! The usage text.

use super::COMMAND_NAME;

/// The usage text, with the command by the name it is called by (`COMMAND_NAME`), not the binary's.
pub fn help() -> String {
    let wh = COMMAND_NAME;
    let sessions = crate::ipc::control::sessions::help(wh);
    let sessions = sessions.trim_end();
    format!(
        "\
Wormhole: control the running app from the command line

  {wh} list
      Every window, terminal, tab and pane, with their ids.
  {wh} status [<target>]
      One terminal's status: the target's, or this tab's inside a terminal; a summary otherwise.
  {wh} notify [<target>] [--title <text>] <body>...
      A notification about a terminal (this tab without a target), in your words.
  {wh} send-text <target> [--submit] <text>... | -
      Type into a terminal, as a paste (`-` reads the text from stdin); --submit presses Return.
  {wh} new-terminal [--folder <path>] [--command <command>] [--window <id>] [--worktree <branch>]
      A new terminal in the front window (or --window), in a folder, running a command in its shell.

{sessions}
  {wh} welcome [-- <program> [<args>...]]
      The first-run welcome, in this terminal; then it starts <program> (your shell) here.

  <target>: --terminal <id> | --tab <id> | --pane <id>, with the ids `list` prints.
  --json prints the reply as one JSON object (WORMHOLE_JSON=1 makes that the default, --text asks for
  text again); the exit code is 0 when it worked, 1 when the app would not or could not, 2 for a
  mistake in the arguments, 3 when Wormhole isn't running (list and status then still show the
  terminals that keep running without it).

Typing into terminals and opening them are off until Settings → Coding agents → \"Let the command
line type and open terminals\" is on; listing, status and notify always work. Inside a terminal,
WORMHOLE_TAB_ID says which tab asked.

{wh} runs in Wormhole's own terminals; for any other terminal, Settings → Coding agents → \"Command
line tool\" installs it.
"
    )
}

#[cfg(test)]
mod tests {
    use super::super::is_verb;
    use super::help;

    #[test]
    fn the_sessions_verb_is_ours_and_in_the_usage() {
        assert!(is_verb("sessions"));
        let help = help();
        for line in ["sessions list", "sessions end <id> | --all", "sessions forget <id>"] {
            assert!(help.contains(line), "{line}");
        }
    }
}
