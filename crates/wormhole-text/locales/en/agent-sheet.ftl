# The question Wormhole asks when a program wants to control it (the command line, MCP, a script): a
# native sheet on the front window. The words that name the program, its path and its signer are data
# and are not here; what surrounds them is.

## The title

# { $name } is the program's name, e.g. Claude Code.
agent-sheet-title = Allow “{ $name }” to control { -brand }?

agent-sheet-title-scripts = Allow scripts to control { -brand }?

## What it wants, in one line

agent-sheet-wants-read = It wants to read what terminals show.
agent-sheet-wants-type = It wants to type into terminals.
agent-sheet-wants-run = It wants to run commands in terminals.
agent-sheet-wants-open = It wants to open and arrange terminals and windows.
agent-sheet-wants-close = It wants to ask you to close a terminal.

## What allowing means

agent-sheet-detail = Allowing lets this program read what terminals show, type into them, run commands and open windows. It can never close a terminal without asking you.
agent-sheet-detail-scripts = Any program that is not a coding agent { -brand } knows counts as Scripts: a script, a tool, or a command you typed at your own prompt. Allowing it lets all of them in until { -brand } quits.

## Warnings, one line each

# { $name } is the program's name, e.g. claude.
agent-sheet-warning-unusual = This program is called { $name } but is not signed by its usual developer.

agent-sheet-warning-unsigned = It is not signed, so only this exact file is remembered.
agent-sheet-warning-ad-hoc = It has an ad hoc signature, so only this exact code is remembered: an update asks again.
agent-sheet-warning-changed = It has changed since you allowed it.
agent-sheet-warning-root = It runs as root.

## The details block, one labelled line each

agent-sheet-label-path = Path
agent-sheet-label-pid = PID
agent-sheet-label-signed-by = Signed by
agent-sheet-label-started-by = Started by
agent-sheet-label-runs-in = Runs in
agent-sheet-label-runs-on = Runs on
agent-sheet-label-asked-by = Asked by

# The signer of a program with a Developer ID. { $subject } is who signed it, e.g. Anthropic PBC, and { $team } its ten-character team id.
agent-sheet-signed-developer = { $subject } (Developer ID, team { $team })

# The signer of a program signed with a Developer ID that names no organisation. { $team } is its team id.
agent-sheet-signed-team = Developer ID, team { $team }

agent-sheet-signed-apple = Apple
agent-sheet-signed-ad-hoc = Ad hoc signature
agent-sheet-signed-other = Signed, not by a Developer ID
agent-sheet-signed-none = Not signed
agent-sheet-signed-unreadable = Could not be read

# The terminal a program runs in. { $title } is the terminal's name in the sidebar.
agent-sheet-in-terminal = terminal “{ $title }”

# The program an interpreter runs on. { $name } is the interpreter, e.g. node, and { $signer } its signature as above.
agent-sheet-runs-on = { $name }, signed by { $signer }

## A program that is let in reaches for a terminal it did not open

# The title of the question about reading a terminal. { $name } is the program's name, e.g. Claude Code, and { $terminal } the terminal's name in the sidebar.
agent-terminal-title-read = Allow “{ $name }” to read “{ $terminal }”?

# The same about typing into it. { $name } is the program's name and { $terminal } the terminal's name in the sidebar.
agent-terminal-title-type = Allow “{ $name }” to type into “{ $terminal }”?

# The same about running commands in it. { $name } is the program's name and { $terminal } the terminal's name in the sidebar.
agent-terminal-title-run = Allow “{ $name }” to run commands in “{ $terminal }”?

# The same about renaming it or bringing it forward. { $name } is the program's name and { $terminal } the terminal's name in the sidebar.
agent-terminal-title-open = Allow “{ $name }” to rename or bring forward “{ $terminal }”?

# The same four for Scripts, which have no name. { $terminal } is the terminal's name in the sidebar.
agent-terminal-title-scripts-read = Allow scripts to read “{ $terminal }”?

# Scripts typing into a terminal. { $terminal } is the terminal's name in the sidebar.
agent-terminal-title-scripts-type = Allow scripts to type into “{ $terminal }”?

# Scripts running commands in a terminal. { $terminal } is the terminal's name in the sidebar.
agent-terminal-title-scripts-run = Allow scripts to run commands in “{ $terminal }”?

# Scripts renaming a terminal or bringing it forward. { $terminal } is the terminal's name in the sidebar.
agent-terminal-title-scripts-open = Allow scripts to rename or bring forward “{ $terminal }”?

agent-terminal-detail = It did not open this terminal and does not run in it. “Allow for This Session” lets it use this terminal, and no other, while the program runs; “Until { -brand } Quits”, until { -brand } quits.
agent-terminal-detail-scripts = Scripts did not open this terminal. Allowing it lets scripts use this terminal, and no other, until { -brand } quits.

agent-terminal-label-terminal = Terminal
agent-terminal-label-folder = Folder

## The buttons

agent-sheet-button-deny = Don’t Allow
agent-sheet-button-session = Allow for This Session
agent-sheet-button-quit = Until { -brand } Quits
agent-sheet-button-always = Always

## A mode that was changed outside { -brand }

# The title of the question. { $mode } is the new mode: Off, Ask or Allowed while open.
agent-change-title = Agent access was changed outside { -brand } to “{ $mode }”. Keep it?

# { $previous } is the mode that stays in force until the answer: Off, Ask or Allowed while open.
agent-change-detail = Something other than { -brand }’s Settings edited the file, and the new mode lets more programs in. Until you answer, “{ $previous }” stays in force; Don’t Keep puts it back in the file.

agent-sheet-button-dont-keep = Don’t Keep
agent-sheet-button-keep = Keep

## The name of the identity that is everything but a known agent

agent-scripts-label = Scripts
