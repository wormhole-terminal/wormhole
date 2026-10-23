# Settings window, agents page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-agents = Coding agents
    .intro = { -brand } recognises the agent running in each terminal. With status hooks it knows exactly when it's working, waiting for you, or done.

settings-agents-summary = Agents found
    .keywords = installed hooks repair path summary

## Notifications

settings-agents-group-notifications = Notifications

settings-agents-notifications = Notify me when an agent needs you or finishes
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
