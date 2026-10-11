# Changelog

All notable changes to Wormhole are listed here. The format follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in
[CONTRIBUTING.md](CONTRIBUTING.md#versioning).

## [Unreleased]

### Added

- The status bar shows how full the selected terminal's agent context window is: "Context",
  a meter and the share used, in amber once it is nearly full. It needs Claude Code's status
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

- Security updates say so. When a new version fixes a security issue, Settings → Updates shows
  "Security update available", how severe the issue is, and links to the published advisories; a
  security update is offered even if you skipped that version, and it can't be skipped.

## [0.3.0] - 2026-10-10

### Changed

- Wormhole now uses its own name everywhere. At the first launch the data
  folder moves to ~/Library/Application Support/Wormhole, with a copy of the
  old database kept beside it; terminals, notes and settings come along.
- New project folders are called .wormhole; existing .andromeda folders keep
  working and stay trusted.
- Terminals get WORMHOLE_* variables; the old ANDROMEDA_* ones are still
  set, and agent hooks from earlier versions are updated by themselves.
- macOS sees the app under a new identifier: it may ask again to send
  notifications, and permissions granted in System Settings may need
  granting again. Wormhole stays your default terminal if it was.
- Updating from 0.2.0 or earlier: download this version from the website once.

## [0.2.1] - 2026-10-10

### Changed

- This version can install the next one as an update, which moves the app
  to its new identifier.

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
  - Splits and several windows; everything comes back after a restart.
  - Shell profiles: zsh, bash, fish, nu or any program.
  - Command blocks with exit codes and timing, and jump between prompts.
  - Inline images (kitty, iTerm2 and Sixel), links, find in scrollback.
- Coding agents
  - Claude Code, Codex, Gemini CLI, Copilot CLI, Cursor Agent, OpenCode,
    Aider, Amp, Goose, Qwen Code, Crush, Droid, Auggie, Kiro and Cline are
    recognised; status is exact once their hooks are installed.
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

[Unreleased]: https://github.com/wormhole-terminal/wormhole/commits/main
