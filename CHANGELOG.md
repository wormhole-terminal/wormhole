# Changelog

All notable changes to Wormhole are listed here. The format follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in
[CONTRIBUTING.md](CONTRIBUTING.md#versioning).

## [Unreleased]

### Fixed

- The terminal list stays put: pinned terminals first, then the rest from first made to last, and
  nothing moves when you open a terminal, split a pane, change folder or change status; a band
  keeps its place while it has a terminal. The ⌘1–⌘9 numbers are dealt down the list and then stay with their
  terminal (a new terminal gets the next free one, whatever it sits), and are dealt again only when you pin, unpin,
  drag or close one. Rows now slide to
  their new place (not with Reduce Motion). Settings → Terminals & sidebar has Order (Stable or
  Classic) and ⌘1–⌘9 numbers (Stay with the terminal or Follow the list) to bring the old behaviour back.

## [0.4.2] - 2026-10-15

### Fixed

- The About window and the welcome page show the new app icon, light or dark with the theme, and the
  intro's ship docks on its `>`.

## [0.4.1] - 2026-10-15

### Changed

- A new app icon: the `>_` prompt in glass over a blue wormhole swirl, deeper blue in dark mode.

## [0.4.0] - 2026-10-15

### Added

- Wormhole answers the Program Status Protocol (OSC 7501). Coding agents and other programs that
  report through it show working, needs you and done in the sidebar exactly, with no hooks
  installed and also over ssh. A program's question or its finishing reaches you as a
  notification, at most once in a few seconds per terminal.
- A terminal can now be failed. A command that ends with a non-zero exit code (in a shell with
  integration), and an agent or program that stops on an error, show a red cross in the sidebar
  and the tab, a red dot on the menu bar item and a "failed" count in the status bar, until you
  open the terminal. The row keeps "command · exit 101" and when it ended until the next command
  starts; Ctrl-C isn't a failure. A banner tells you about it, with the exit code and the last line
  the command printed, once it ran past the same time as a finishing command does. Settings →
  Notifications has a switch for it ("A command fails"), and the Dock badge set to "Needs you +
  finished" counts failed terminals too.
- Programs that report through OSC 7501 need not be coding agents. A script, `brew` or a build
  tool that says it works, waits for you, is done or failed shows that in its row, the status bar
  and the menu bar under the name it gives (or the command's), with the percentage it reports,
  and reaches you as a notification like an agent's question, finish or error does.
- The status bar shows how full the selected terminal's agent context window is: "Context",
  a meter and the share used, in amber once it is nearly full. It needs the agent's status
  line, like the title row's gauge.
- Pull requests show how their checks and reviews stand: a green check, a red cross or a
  dashed circle while checks run, beside the number in the title row and now also on the
  folder's band in the sidebar. A notification tells you when a pull request's checks pass
  or fail. Drafts get their own icon; the tooltip names the review decision.
- A folded folder in the sidebar shows the amber needs-you dot while a terminal inside it is
  waiting for you, so folding never hides a question.
- Window → Go to Finished Terminal (⌥⇧⌘J) goes to the terminal that finished last and you
  haven't looked at yet, then the one before it: a way through everything ready for review.
- Each worktree Wormhole makes gets ten ports of its own, in its terminals as `$WORMHOLE_PORT`
  and `$WORMHOLE_PORTS`, so dev servers in several worktrees don't fight over one port.
- A project can name a setup command for new worktrees (`"worktree": { "setup": "npm install" }`
  in `.wormhole/config.json`); it runs once in the new worktree's terminal, after you trust it.
- Comment on a diff's lines for the agent: click a line in the diff view to write what should
  change there. The comments gather across files under the diff, and Paste into Terminal puts
  them in the terminal's prompt as one message, each with its file, line and the line quoted,
  for you to send. Where the program in the terminal would run a paste line by line (a plain
  shell without bracketed paste), they are copied to the clipboard instead.
- The command line has a short name: `wh list`, `wh notify`, `wh send-text` and the rest work in
  every Wormhole terminal as they are, with no alias. For other terminals, Settings → Coding
  agents → Command line tool installs `wh` into `/usr/local/bin` or `~/.local/bin` (only when you
  press Install, never over another `wh`, and Uninstall removes it) and shows the line to add to
  your shell's startup file when that folder isn't on its `PATH`.
- Security updates say so. When a new version fixes a security issue, Settings → Updates shows
  "Security update available", how severe the issue is, and links to the published advisories; a
  security update is offered even if you skipped that version, and it can't be skipped.
- Restart to update says what is still running first. When an agent or a command is running in
  any terminal, "Restart now" in Settings → Updates lists those terminals and what runs in them,
  and offers Restart, Restart When Idle (the app restarts once everything has been idle for 30
  seconds; Settings shows the wait, with Restart now and Don't restart) or Later. With nothing
  running it restarts at once, as before.

### Fixed

- Closing a terminal ends the program running in it, not only its shell. Before, an agent or
  another program in the foreground could stay behind with its shell, using memory, until you
  quit Wormhole.
- Switching to another app no longer posts that an agent finished when it
  was idle or still working. Without the agent's hooks installed, its redraw when Wormhole
  went behind counted as work that then finished.

## [0.3.0] - 2026-10-10

### Changed

- The app is now named Wormhole. On first launch, it moves existing data to
  ~/Library/Application Support/Wormhole and keeps a backup of the database.
  Your terminals, notes and settings are preserved.
- New project folders are called .wormhole; existing .andromeda folders keep
  working and stay trusted.
- Terminals get WORMHOLE_* variables; the old ANDROMEDA_* ones are still
  set, and agent hooks from earlier versions are updated automatically.
- The app's macOS identifier has changed. You may need to allow notifications
  and grant other permissions again in System Settings. If Wormhole was your
  default terminal, it remains the default.
- To upgrade from 0.2.0 or earlier, download and install Wormhole from the
  website. Later updates can be installed from the app.

## [0.2.1] - 2026-10-10

### Changed

- Enables automatic updating to 0.3.0, including the change to the app's
  macOS identifier.

### Fixed

- An empty note's hint wraps inside the panel instead of being cut off.
- The preview in Settings → Appearance shows the window's title row, not
  the terminal tabs Wormhole no longer has.
- The first-run steps no longer mention tabs.
- Get Info and the About window name Wormhole in the copyright line.

## [0.2.0] - 2026-10-10

### Added

- Wormhole can be your Mac's default terminal: Settings → Terminal →
  Default terminal runs double-clicked .command files and programs in a
  new Wormhole terminal. Open With in Finder and folders dropped on the
  Dock icon open a terminal there.

### Changed

- Busy screens draw many times faster. Full-screen animations, fast TUIs
  and output full of CJK, emoji or braille now take a few milliseconds a
  frame instead of 30 to 80, so they run smoothly, and moving the window,
  switching terminals and opening Settings stay responsive while a
  program floods the terminal.
- Colored output is read faster.

### Fixed

- A row with Hebrew or Arabic in it no longer shows the rest of the row
  reversed, and emoji such as ❤️ sit in their own cell.

## [0.1.3] - 2026-10-09

### Fixed

- The sidebar no longer flickers a second copy of the folder band when
  you scroll a short list with the trackpad.

## [0.1.2] - 2026-10-08

### Added

- Wormhole runs on Intel Macs again: this release is a universal build
  for Apple Silicon and Intel (macOS 14 or later).

## [0.1.1] - 2026-10-08

### Changed

- A new app icon.
- A smaller, faster release process, with a corrected description of
  each version in the update feed.

### Removed

- Releases are built for Apple Silicon (macOS 14 or later). Intel Macs
  are no longer supported by new versions.

## [0.1.0] - 2026-10-07

### Added

The first release: a native macOS terminal for working with AI coding
agents.

- Terminals
  - A sidebar of terminals with live agent status (working, finished,
    waiting for you), the agent's logo, git branch, changed files and
    open notes.
  - Splits and multiple windows, with saved terminals, folders, notes and tasks
    restored after a restart. Running commands and agents do not resume.
  - Shell profiles: zsh, bash, fish, nu or any program.
  - Command blocks with exit codes and timing, and jump between prompts.
  - Inline images (kitty, iTerm2 and Sixel), links, find in scrollback.
- Coding agents
  - Claude Code, Codex, Gemini CLI, Copilot CLI, Cursor Agent, OpenCode,
    Aider, Amp, Goose, Qwen Code, Crush, Droid, Auggie, Kiro and Cline are
    recognised. Supported agents can report status directly through installed hooks.
  - Notes and next steps per terminal, and a stash for the prompt you
    were typing.
  - Agent status in the menu bar, and a global show/hide shortcut.
  - A command line (Wormhole list, status, notify, send-text,
    new-terminal) to drive terminals from scripts.
- Git
  - Changes, History and a syntax-highlighted diff, unified or split.
  - A terminal in a new git worktree.
- Look and feel
  - Translucent window with native blur, 14 built-in themes, a Theme
    Studio, theme import from Ghostty, iTerm2 and Alacritty, icon themes
    and your own fonts.
  - Project folders (.andromeda/) for shared settings, actions and notes.
- Updates
  - Builds are signed and notarized, and update themselves: checked in
    the background and verified with a minisign signature and Apple's
    code signature.

[Unreleased]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.2...HEAD
[0.4.2]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/wormhole-terminal/wormhole/releases/tag/v0.4.0
