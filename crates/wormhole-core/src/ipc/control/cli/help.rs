//! The usage text.

use super::COMMAND_NAME;

/// The usage text, with the command by the name it is called by (`COMMAND_NAME`), not the binary's,
/// including the verbs of the automation flag.
pub fn help() -> String {
    help_with(true)
}

/// The usage text; `automation` says whether it names the verbs that are behind the automation flag
/// in this build (they still parse when it doesn't: the app has the last word).
pub fn help_with(automation: bool) -> String {
    let wh = COMMAND_NAME;
    let watching = if automation {
        format!(
            "  {wh} events [--topics <list>] [--terminal <id>]... [--window <id>]... [--output]
      Watch what happens, as it happens: terminals' status, titles and folders, commands starting and
      finishing, terminals, windows and splits. One line each (ctrl-c stops); --json gives one JSON
      object per line. Topics: windows, terminals, status, title, folder, bell, blocks, splits, and
      output (what terminals print: --output, which needs reading to be allowed).
  {wh} read [<target>] [--scrollback <n> | --lines <from>:<to> | --block <id>|last] [--join] [--styles] [--numbers]
      What a terminal shows: its screen (also a full-screen program's), the last <n> lines of its
      history, lines <from> to <to> (0 is the screen's top row, history is negative) or a command's
      block. --join puts wrapped rows back together; --json also has the size, the cursor and the
      modes (mouse, application keys) and, with --styles, each line's colours.
  {wh} blocks [<target>] [--last <n>] [--output]
      A terminal's commands: id, state, exit code, time, and with --output what they printed.
  {wh} send-keys <target> <key>... [--text <string>]... [--paste <string>]...
      Press keys in a terminal, in the order given: ctrl-c, enter, up, shift-tab, f5, alt-x, a
      character. --text types a string key by key, --paste pastes it. Keys are encoded for what the
      program in the terminal has asked for (application cursor keys, the kitty keyboard).
  {wh} mouse <target> click|press|release|move <row> <col> [--button left|middle|right] [--double]
      {wh} mouse <target> scroll <row> <col> up|down|left|right [--lines <n>] [--shift] [--alt] [--ctrl]
      Click or scroll in a program that asked for the mouse (rows and columns count from 0).
"
        )
    } else {
        String::new()
    };
    let reading = if automation {
        " Reading what terminals show is off until \"Let the command line read terminals\" is on;\na terminal can always read itself."
    } else {
        ""
    };
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
{watching}
{sessions}
  {wh} welcome [-- <program> [<args>...]]
      The first-run welcome, in this terminal; then it starts <program> (your shell) here.

  <target>: --terminal <id> | --tab <id> | --pane <id>, with the ids `list` prints.
  --json prints the reply as one JSON object (WORMHOLE_JSON=1 makes that the default, --text asks for
  text again); the exit code is 0 when it worked, 1 when the app would not or could not, 2 for a
  mistake in the arguments, 3 when Wormhole isn't running (list and status then still show the
  terminals that keep running without it).

Typing into terminals and opening them are off until Settings → Coding agents → \"Let the command
line type and open terminals\" is on; listing, status and notify always work.{reading} Inside a terminal,
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
