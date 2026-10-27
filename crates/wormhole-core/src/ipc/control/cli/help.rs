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
  {wh} read [<target>] [--last <n> | --lines <from>:<to> | --block <id>|last] [--join] [--styles] [--numbers]
      What a terminal shows: its screen (also a full-screen program's), the last <n> lines of its
      history, lines <from> to <to> (0 is the screen's top row, history is negative) or a command's
      block. --join puts wrapped rows back together; --json also has the size, the cursor and the
      modes (mouse, application keys) and, with --styles, each line's colours.
  {wh} blocks [<target>] [--last <n>] [--output]
      A terminal's commands: id, state, exit code, time, and with --output what they printed.
  {wh} send-keys <target> <key>... [--type <string>]... [--paste <string>]...
      Press keys in a terminal, in the order given: ctrl-c, enter, up, shift-tab, f5, alt-x, a
      character. --type types a string key by key, --paste pastes it. Keys are encoded for what the
      program in the terminal has asked for (application cursor keys, the kitty keyboard).
  {wh} mouse <target> click|press|release|move <row> <col> [--button left|middle|right] [--double]
        [--shift] [--alt] [--ctrl]
  {wh} mouse <target> scroll <row> <col> up|down|left|right [--by <n>] [--shift] [--alt] [--ctrl]
      Click, drag (move --button left) or scroll in a program that asked for the mouse (rows and
      columns count from 0).
  {wh} new-window [--folder <path>] [--command <command>] [--activate]
      A window with one new terminal. It opens behind your other windows and takes no keys unless
      --activate. Prints the new window's and terminal's ids.
  {wh} split <target> [--right|--down|--left|--up] [--size <percent>] [--folder <path>]
        [--command <command>] [--select]
      A new terminal beside the target (right by default), in its folder and shell unless --folder,
      in the target's split. --size is its share of the split. The selection stays where it is
      unless --select. Prints the ids; nothing changes if the window has no room.
  {wh} focus <target> [--activate]
      Select a terminal and show its split. The keys stay where they were; --activate gives them to the
      terminal and brings the window forward.
  {wh} rename <target> <name>... | --clear
      Name a terminal as the sidebar's rename does; --clear calls it after what runs in it again.
  {wh} close <target>
      Asks you, in the terminal's window, whether to close it. It never closes on a script's say; the
      reply only says the question is up, and `events` tells if you said yes (terminal_closed).
  {wh} wait <target> [--status <state>[,<state>]...] [--text <regex>] [--gone <regex>] [--stable <ms>]
        [--block-done] [--timeout <s>]
      Wait until one of the conditions given holds and say which: the terminal's state (idle, working,
      needs_you, finished, failed, exited), a regular expression found on its screen, one no longer
      found, no output for <ms>, or a command finishing. 30 s by default (at most 600); exits 124 when
      the time is up. The text conditions need reading to be allowed.
  {wh} run <target> [--timeout <s>] [--exit-code] [--] <command>...
      Run a command at the terminal's shell prompt and wait for it: prints what it printed, and its exit
      code on stderr. Needs shell integration, a shell at its prompt, and typing to be allowed; the
      output needs reading to be allowed. --exit-code exits with the command's own code (124 if it
      timed out); 10 minutes by default (at most 3600). It clears the prompt line first (^U).
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
