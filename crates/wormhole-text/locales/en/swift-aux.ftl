# The Swift app's secondary windows: the quit, close and restart questions, About and Licenses, the update window,
# the project trust window and the first-run intro (the GPUI app writes the intro's words in Rust, so they live here).
# Words the GPUI app has a message for (the Session Hosts window, the agent approval sheet, the Settings pages) are
# those messages; this file holds only what has none.

## The question before quitting

# The button that quits and keeps the terminals the app can keep.
dialog-quit = Quit

# The button that quits and ends every terminal.
dialog-quit-end-all = Quit and End All

dialog-quit-title = Quit { -brand }?

dialog-quit-end-all-title = Quit and End All Terminals?

# The box under the question: whether the terminals keep running in a background helper.
dialog-keep-running = Keep terminals running in the background

## The questions before closing a terminal, a group or a window

# { $name } is the terminal's name in the sidebar.
close-session-title = Close “{ $name }”?

# { $name } is the group's name in the sidebar.
close-group-title = Close the group “{ $name }”?

# Stands for the agent's name when the question does not know it: "Its agent is still working."
close-its-agent = Its agent

close-agent-waiting = An agent is waiting for an answer.
close-agent-working = An agent is still working.
close-command-running = A command is still running.

# { $list } is the programs that end, such as "claude, npm run dev".
close-still-running = Still running: { $list }.

# { $who } is an agent's name, or "Its agent".
close-is-working = { $who } is still working.

# The same for an agent that waits. { $who } is an agent's name, or "Its agent".
close-is-waiting = { $who } is waiting for an answer.

close-shell-ends = Its shell ends.
close-ends-process = Closing ends the process.
close-ends-them = Closing ends them.

close-window-one = The terminal in this window will end.

# { $count } is a whole number of terminals, two or more.
close-window-many = The { $count } terminals in this window will end.

close-group-one = The terminal in this group will end.

# { $count } is a whole number of terminals, two or more.
close-group-many = The { $count } terminals in this group will end.

# What a closing terminal also takes with it. { $stake } is a phrase from the feature that holds it, such as "notes and tasks go too".
close-its-stake = Its { $stake }

# The same for several terminals.
close-their-stake = Their { $stake }

# Ends a list of programs or terminals that is cut short. { $count } is a whole number of those left out.
close-and-more = and { $count } more

## The question before restarting to use an update

update-restart-title = Restart to Update?
update-restart = Restart
update-restart-when-idle = Restart When Idle
update-later = Later
update-restart-note = Your terminals come back after the restart, but what runs in them ends.

# What a terminal in that question is doing. { $agent } is the agent's name.
restart-agent-working = { $agent } is working

# The same for an agent that waits for an answer. { $agent } is the agent's name.
restart-agent-waiting = { $agent } is waiting for an answer
restart-still-working = still working
restart-waiting-for-you = waiting for you
restart-command-running = a command is running

## About and Licenses

about-title = About { -brand }

# The version line of About. { $version } is the number.
about-version = Version { $version }

# Added after the version of a build made for development.
about-debug-build = (debug build)

licenses-title = Licenses
licenses-search = Search packages or licenses
licenses-no-match = No package matches.

# A package that declares no license, in the list.
licenses-none = no license declared

# Under the list. { $total } is a whole number of packages.
licenses-count = { $total ->
    [one] 1 package
   *[other] { $total } packages
}

# Under the list while a search narrows it. { $shown } and { $total } are whole numbers of packages.
licenses-count-of = { $shown } of { $total } packages

# { $license } is the license's name, such as MIT.
licenses-license = License: { $license }

licenses-no-license = No license declared
licenses-no-file = This package doesn’t include a license file. The license above is what its Cargo.toml declares; see the repository for the full text.
licenses-unreadable = The list of licenses could not be read.

## The update window

# The name of the update channel that gets a build of every change.
aux-updates-preview = Preview
aux-updates-channel-description = Preview also gets a build of every change once it passes the tests, and shows the feature flags

# { $ago } is how long ago, as the system words it ("4 minutes ago").
aux-updates-checked-ago = Checked { $ago }.

aux-updates-restart-when-idle = Restart when idle

# { $received } of { $expected } is the size downloaded so far of the whole, each with its unit.
update-download-progress = { $received } of { $expected }

# { $version } is the number.
update-available = { -brand } { $version } is available.

# Once it is installed. { $version } is the number.
update-ready = { -brand } { $version } is ready. Restart to use it.

update-security = Security update
update-release-notes = What’s new
update-no-notes = This release has no notes.

# { $why } is what went wrong.
update-failed = The update failed: { $why }

# Shown by a build that has no update feed to ask.
update-no-feed = This build has no update feed. Start it with --update-feed <url> to check one.

## The project trust window

trust-window-title = Project commands
trust-window-closed = This project is no longer open in a terminal.
trust-badge-trusted = trusted
trust-badge-changed = changed
trust-badge-not = not trusted
trust-summary-trusted = Trusted: actions, the terminal layout and project env may run.
trust-summary-changed = Files changed since you trusted this project. Review what it would run now and approve again.
trust-summary-untrusted = It wants to run these commands. Nothing runs until you approve.
trust-nothing = Nothing here would run.
trust-with-touch-id = Trust with Touch ID
trust-with-password = Trust (approve with password)

# Under the commands. { $count } is a whole number of lines.
trust-lines = { $count ->
    [one] 1 line
   *[other] { $count } lines
}

# The reason Touch ID shows. { $folder } is the project folder's name.
trust-touch-id-reason = trust the project folder “{ $folder }” and let it run commands

# The refusal of a worktree in a project that is not trusted.
trust-worktree-refused = Worktrees are made only in a trusted project: approve this folder's project in the Project tab first.

## The first-run intro: the welcome page and its steps

onboarding-dialog-label = Welcome to { -brand }
onboarding-tagline = A terminal for working with AI coding agents.
onboarding-specimen = Which agent is working, which needs input, which is done.
onboarding-get-started = Get started
onboarding-back-to-wormhole = Back to { -brand }
onboarding-skip-intro = Skip intro
onboarding-skip-setup = Skip setup
onboarding-replay = Replay intro

# The key hints under the buttons: the word after the key's name.
onboarding-hint-continue = continue
onboarding-hint-skip = skip

onboarding-continue = Continue
onboarding-start = Start

# { $step } and { $total } are whole numbers.
onboarding-step-counter = Step { $step } of { $total }

# The same on the last step, when no coding agent was found. { $step } and { $total } are whole numbers.
onboarding-step-counter-no-agents = Step { $step } of { $total } · no agents found

onboarding-folder-title = Where do you work?
onboarding-folder-intro = { -brand } is a terminal first. Pick a folder to start in; every terminal remembers its own.
onboarding-open-folder = Open folder…
onboarding-open-folder-detail = A terminal starts there; git changes, history and branch follow it.

# { $folder } is the folder's name.
onboarding-starts-in = Starts in { $folder }

onboarding-recents = Recents
onboarding-places = Places
onboarding-drop-folder = Or drop a folder here
onboarding-place-home = Home
onboarding-place-desktop = Desktop
onboarding-place-documents = Documents
onboarding-place-developer = Developer

# The button of the folder panel.
onboarding-panel-start-here = Start here

onboarding-look-title = Make it yours

# { $keys } is the shortcut of Settings, as the menu shows it.
onboarding-look-intro = Everything changes live, and all of it is in Settings ({ $keys }) later.

onboarding-primary = Primary colour
onboarding-font = Terminal font
onboarding-import-from = Import from Ghostty, iTerm2 or Alacritty…

# The button of the theme file panel.
onboarding-panel-import-theme = Import theme

# { $count } is a whole number of agents.
onboarding-agents-intro = Found { $count } on this Mac. Install status hooks for the ones you want; they report when an agent works, waits for you or is done.

# A label before the folder an agent's hooks go in.
onboarding-hooks-in = Hooks in

onboarding-repair = Repair
onboarding-on-path = On PATH
onboarding-on-path-output = On PATH · status from its output

# { $count } is a whole number of agents.
onboarding-not-found = Not found on PATH · { $count } more recognised

onboarding-installs-nothing = Start installs nothing; only Install does. Hooks add entries of their own and back up what they change. Skip any agent: Settings › Coding agents adds or removes hooks later.
onboarding-notify = Notify me when a background terminal needs me or finishes
onboarding-notify-command = Notify me when a long command in a background terminal finishes

# The project folder found in the chosen folder. { $name } is its name and { $dir } the folder's own name (".wormhole").
onboarding-trust-has = { $name }/ has a { $dir }/ folder

onboarding-trust-waits-0 = Nothing in it runs commands yet.
onboarding-trust-waits-1 = Its action runs a command, so it waits until you trust the folder.

# { $count } is a whole number of commands, two or more.
onboarding-trust-waits-n = Its actions run { $count } commands, so they wait until you trust the folder.

onboarding-trust-works-now = Its theme and notes work now.
onboarding-trust-later = Later

onboarding-primer-title = Ready when you are
onboarding-primer-intro = { -brand } didn’t find a coding agent on this Mac. Nothing here depends on one.
onboarding-use-as-terminal = Use { -brand } as a terminal
onboarding-use-as-terminal-detail = The git panel works in any shell.
onboarding-use-as-terminal-detail-notes = Notes, next steps and the git panel work in any shell.
onboarding-things-to-try = Things to try first
onboarding-try-jump = Jump to anything
onboarding-try-shortcuts = See every shortcut
onboarding-try-changes = Show what changed in git
onboarding-try-note = Jot a note for this folder
onboarding-try-panels = Hide the side panels

# { $count } is a whole number of other agents.
onboarding-no-agent = No coding agent found. Install Claude Code, Codex or one of { $count } others later and { -brand } recognises it in any terminal, then offers its hooks once.

## The names of the intro's animations

intro-style-emerge = Emerge
intro-style-vortex = Edge Vortex
intro-style-collapse = Collapse
intro-style-ftl = FTL Jump
intro-style-aurora = Aurora
intro-style-tentacles = Tentacles
intro-style-voyage = Voyage
intro-style-glow = Glow

## The close question's buttons and the worktree lines

close-confirm-session = Close
close-confirm-group = Close Group
close-window-title = Close this window?
close-confirm-window = Close Window

# The open panel's button (File → Open Folder…).
open-folder-prompt = Open

# { $branch } is the worktree's branch, { $folder } its folder.
worktree-close-one-dirty = Its worktree { $branch } at { $folder } was made by Wormhole. It has uncommitted changes, so it stays.
# { $branch } is the worktree's branch, { $folder } its folder.
worktree-close-one-clean = Its worktree { $branch } at { $folder } was made by Wormhole. Close keeps it; Close and Remove Worktree deletes its folder (the branch stays).
worktree-close-remove-one = Close and Remove Worktree
worktree-close-remove-many = Close and Remove Worktrees

# { $names } lists each worktree as "<branch> at <folder>".
worktree-close-group-lead-one = Wormhole made the worktree { $names }.
# { $names } lists each worktree as "<branch> at <folder>".
worktree-close-group-lead-many = Wormhole made the worktrees { $names }.
worktree-close-group-none = They have changes or other terminals in them, so they stay.
worktree-close-group-all = Close keeps them; Close and Remove Worktrees deletes their folders (the branches stay).
worktree-close-group-some = Close keeps them; Close and Remove Worktrees deletes the folders without changes or other terminals (the branches stay).
# One entry of that list.
worktree-close-entry = { $branch } at { $folder }
