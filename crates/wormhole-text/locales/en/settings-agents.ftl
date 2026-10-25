# Settings window, agents page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-agents = Coding agents
    .intro = { -brand } recognises the agent running in each terminal. With status hooks it knows exactly when it's working, waiting for you, or done.

settings-agents-summary = Agents found
    .keywords = installed hooks repair path summary

## Notifications

settings-agents-group-notifications = Notifications

settings-agents-notifications = Notify me when an agent needs input or finishes
    .description = These, and notifications about long commands and the bell, have their own section
    .keywords = notifications alerts banner

## Installed

settings-agents-group-installed = Installed

settings-agents-installed = Installed agents
    .keywords = claude code codex gemini copilot cursor opencode aider amp goose qwen hooks status install remove repair

## Other agents

settings-agents-group-other-agents = Other agents

settings-agents-other = Other agents
    .description = Agents { -brand } recognises that aren't on this Mac's PATH
    .keywords = not installed path

## How status works

settings-agents-group-how-status-works = How status works

settings-agents-how = How status works
    .description = With status hooks the agent tells { -brand } the moment it starts working, waits for you or finishes; without them the status is read from the terminal output
    .keywords = hooks output heuristic backup

## Command line

settings-agents-group-command-line = Command line
    .blurb = Scripts and agents can drive { -brand } from the command line: list terminals, read a status and send notifications.

settings-agents-cli-tool = Command line tool
    .description = The { -brand } command in any terminal
    .keywords = wh wht cli command install uninstall path shell alias symlink bin usr local terminal

settings-agents-control-write = Let the command line type and open terminals
    .description = send-text types into a terminal and new-terminal opens one, as a paste; never into the terminal that asks, nor one waiting for your answer
    .keywords = cli command line control api socket send-text new-terminal scripts agents automation remote type

## settings_window/sections/agents.rs (labels, hints and notes drawn by this page's code)

settings-agents-open-notifications = Open Notifications
settings-agents-looking-for-agents = Looking for agents…
settings-agents-reporting-through-hooks = reporting through hooks
settings-agents-needs-repair = needs repair
settings-agents-recognised-not-on-path = recognised, not on PATH
settings-agents-no-coding-agent-on-this-macs = No coding agent on this Mac's PATH. Install Claude Code, Codex or one of the others below: { -brand } recognises it in any terminal, and offers its hooks here.
settings-agents-status-hooks-on = Status hooks on
settings-agents-hooks-incomplete = Hooks incomplete
settings-agents-reading-output = Reading output
settings-agents-installed-hooks-in = Installed · hooks in
settings-agents-installed-hooks-incomplete-in = Installed · hooks incomplete in
settings-agents-installed-status-from-output-hooks-go = Installed · status from output; hooks go in
settings-agents-installed-status-from-output-no-hook = Installed · status from output (no hook support)
settings-agents-remove-hooks = Remove hooks
settings-agents-repair-hooks = Repair hooks
settings-agents-install-hooks = Install hooks
settings-agents-reveal-in-finder = Reveal in Finder
settings-agents-no-hooks = No hooks
settings-agents-not-on-path = Not on PATH
settings-agents-status-hooks-available = status hooks available
settings-agents-status-from-output = status from output
settings-agents-with-status-hooks = With status hooks
settings-agents-the-agent-tells-wormhole-the-moment = The agent tells { -brand } the moment it starts working, waits for you or finishes.
settings-agents-without = Without
settings-agents-status-is-read-from-the-terminal = Status is read from the terminal output: usually right, sometimes a beat late.
settings-agents-hooks-do-nothing-outside-wormhole-only = Hooks do nothing outside { -brand }, only add entries of their own, and a backup of every changed file is kept.

## settings_window/sections/cli_tool.rs (labels, hints and notes drawn by this page's code)

settings-agents-uninstall = Uninstall
settings-agents-install = Install
settings-agents-update = Update
settings-agents-checking-your-shells-path = Checking your shell's PATH…
settings-agents-copy-line = Copy line

## Sentences with a hole in them

# The notice after installing, repairing or removing an agent's hooks failed. { $agent } is the agent's
# name and { $error } the system's reason, in English.
settings-agents-hooks-failed = { $agent } hooks: { $error }

# The warning under an agent whose hooks are only partly there. { $agent } is its name.
settings-agents-hooks-partial = Some of { $agent }'s hook events are missing (it may have been updated). Repair adds them back; your own entries stay.

# A profile's own login of an agent. { $profile } is the profile's name and { $badge } the state of the
# hooks there ("Status hooks on"); the folder follows.
settings-agents-login-state = { $profile } profile · { $badge } in

# The line that opens the list of agents found but not installed. { $count } is a whole number.
settings-agents-others-head = { $count ->
    [one] { $count } more agent { -brand } recognises
   *[other] { $count } more agents { -brand } recognises
}

## The command line tool row. In every message { $command } is the tool's name, as the terminal types it.

# The row failed to install. { $error } is the reason, in English.
settings-agents-cli-install-failed = Couldn't install { $command }: { $error }

# The row failed to remove. { $error } is the reason, in English.
settings-agents-cli-remove-failed = Couldn't remove { $command }: { $error }

# The row's quiet line while the tool is not installed.
settings-agents-cli-not-installed = { $command } in any terminal. Not installed

# The same once it is. { $path } is where it is, as the title row would write it.
settings-agents-cli-installed-at = { $command } in any terminal. Installed at { $path }

# The same while another program of that name is in the way. { $path } is where it is.
settings-agents-cli-foreign-at = { $command } in any terminal. Another { $command } is at { $path }

# The Install button's tooltip. The folders are the ones the tool goes into.
settings-agents-cli-install-tooltip = Puts { $command } in /usr/local/bin, or in ~/.local/bin when that can't be written

# The disabled Install button's tooltip.
settings-agents-cli-foreign-tooltip = Another { $command } is in the way; { -brand } never replaces it

# Under the row, when another program of that name is in the way.
settings-agents-cli-foreign-note = { -brand } never replaces a command it didn't write. Move that { $command } away to install this one.

# Under the row, when the installed copy belongs to another { -brand }.
settings-agents-cli-stale-note = The installed { $command } runs a copy of { -brand } that is somewhere else. Update points it at this one.

# Under the row, when the tool's folder is on the shell's PATH. { $dir } is that folder.
settings-agents-cli-path-on = { $dir } is on your shell's PATH: { $command } works in new terminals.

# Under the row, when it is not. { $dir } is the tool's folder and { $file } the shell's startup file the
# line below it goes in.
settings-agents-cli-path-off = { $dir } isn't on your shell's PATH, so new terminals won't find { $command }. Add this line to { $file }, then open a new terminal:

# Under the row, when the shell's PATH could not be read. { $dir } is the tool's folder.
settings-agents-cli-path-unknown = Couldn't read your shell's PATH. If { $command } isn't found, add { $dir } to it.
