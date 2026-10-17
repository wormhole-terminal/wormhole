# Changelog

All notable changes to Wormhole are listed here. The format follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in
[CONTRIBUTING.md](CONTRIBUTING.md#versioning).

## [Unreleased]

### Added

- The status tile looks like an app icon: black for a terminal, Claude on its orange, Gemini on its
  blue, with a faint sheen and edge. The amber edge of a terminal that needs you breathes more slowly,
  and the tile comes in 32, 20, 18, 16 and 14 points for rows, tabs, the title row and the status bar.
- The terminal list is redrawn as rounded cards, set in from the sidebar's edges, with a rounded
  selection: a card for each group and a card of its own for each terminal in no group. Each
  two-line row leads with a status tile: the agent's logo or the terminal symbol, with a light
  circling its edge while it works (grey while a command runs), a breathing amber edge when it needs
  you, and a badge for done, failed, new output or the bell. Under Reduce Motion the light and the
  edge hold still.
- Your own groups, as in a browser's tab groups: groups you name and colour yourself, in your order
  among the terminals in no group. Drag a terminal into or out of a group, or a group by its head, to
  rearrange; a row's menu adds it to a group or takes it out, and a group's menu renames it, recolours
  it, opens a new terminal in it or ungroups it. A group goes with its last terminal. New terminals
  open in the selected terminal's group.
- Terminals can show as tabs instead of the sidebar (Settings → Show terminals in, or View → Show
  Terminals as Tabs): one row across the top with the selected terminal's group (a menu switches to
  another) and its tabs. The title row then shows the terminal's status as a small tile.
- The right panel is drawn in cards like the sidebar's: Changes has a card per section, History's
  commits are one card, Notes has its Next steps and Notes cards, and Usage and Project follow. The
  panel has no title row any more; its first card starts under the top of the window.
- The panel's tabs (Changes, History, Notes, Usage, Project) sit in a narrow strip down the window's
  right edge that stays when the panel is closed: a click on a tab opens the panel on it, a click on
  the open tab closes it, and an amber dot on Changes says its files changed since you last looked.
  The button at the foot stashes the typed prompt into a note. The panel's button in the title row is
  gone; its key and menu item still work.
- The status bar puts the terminal that has waited longest for you first, as a button with its tile,
  its name and how long it has waited (one click goes there), and counts the rest ("2 working · 1
  done"). Below about 1100 points the words go first.

### Changed

- New windows open with a 320 point sidebar and a 300 point panel (plus its 28 point strip); saved
  widths are kept.

- Settings → Terminals & sidebar has three densities, Regular (48 pt rows with a second line), Compact
  (32 pt, one line) and Dense (26 pt, tighter cards), each with its status tile, for the sidebar and
  the tab bar; and "Show groups as cards" turns the cards off, leaving each group a head and its rows.
  The old compact switch becomes Compact; View → Compact Rows is gone.

### Removed

- The sidebar no longer groups terminals by folder, and its header is only "Terminals" and the +: the
  filter funnel (Show, search, Hide idle), Group by, the new-group button, the Settings gear and the
  shell chevron are gone (a right-click on the + still offers the other shells). The Order setting
  (Stable or Classic) is gone with the folder bands it ordered; terminals keep the order you give them.
  The tab bar's two-row, stacks and bottom-bar layouts are gone too. Old settings that named any of
  these still load and do nothing.

## [1.1.0] - 2026-10-15

### Added

- Terminals have a scrollbar that follows System Settings: with "Show scroll bars" on automatically it
  is a thin thumb over the right edge that shows while you scroll and fades out after a second (it
  widens under the pointer), and with "Always" it is a gutter beside the text. Clicking the track
  pages or jumps to the spot as "Click in the scroll bar to" says (Option does the other one), and the
  thumb can be dragged. It is hidden on the alternate screen and while the minimap is shown.

### Fixed

- The secondary device attributes reply (`CSI > c`) carries an xterm-style version (777) again, as
  programs like vim expect; since 1.0.0 it sent 10000. Wormhole's own version is still in XTVERSION.
- Resizing the window while a full-screen program (vim, less, an agent's interface) is showing no
  longer lets that screen collect lines above it and leaves the terminal's scrollback limit on the wrong
  screen; the full-screen program has no history and the shell's scrollback keeps its configured size.
- With autowrap off and grapheme clustering on, a combining mark or emoji modifier sent after a
  character in the last column joins that character, not the one before it.

## [1.0.0] - 2026-10-15

### Added

- Blinking text (SGR 5 and 6) now blinks, fading once a second while the window is in front, and
  stays steady when Reduce Motion is on; overlined text (SGR 53) gets its line along the top of
  the cell, in the underline color when the program set one.
- Programs can edit rectangles of the screen: fill (DECFRA), erase (DECERA, and DECSERA, which
  keeps protected cells), copy (DECCRA) and change or reverse attributes (DECCARA, DECRARA, with
  DECSACE choosing the stream of cells or the rectangle). They count from the scrolling region in
  origin mode and are cut to the screen.
- A program can ask for the checksum of part of the screen (DECRQCRA), which test suites such as
  esctest2 read the screen back with. It is off: it also lets a program, over ssh too, read the
  screen cell by cell. Settings → Terminal → "Let programs read the screen back" (or
  `allow_screen_checksum` in `config.json`) turns it on for new terminals.
- A program can move, resize, minimise, raise and full-screen the window and read its title back
  (xterm's window operations, `CSI … t`). It is off, because a title is text a program chose and
  reading it back types it into the next program: Settings → Terminal → "Let programs control the
  window" (or `allow_window_ops` in `config.json`) turns it on for new terminals. A size in rows
  and columns resizes the window so that the terminal that asked gets that size. Reports of the
  window's size and place are always answered.
- Colors a program sets (OSC 4, 5, 10 to 19) also paint the pane's padding and are reset by
  OSC 104, 105 and 110 to 119; a `#` color is read as the X11 convention does (`#3a7` is
  `#3000a0007000`).
- Programs can switch to 8-bit controls (S8C1T) and set a conformance level (DECSCL), and
  reverse wraparound (modes 45 and 1045) works for backspace and cursor-left.
- National replacement character sets (German, French, Swedish and the rest of the VT200 family,
  after `CSI ? 42 h`), the DEC Technical and DEC Supplemental sets and the ISO Latin-1 supplement
  can be designated and shifted into place (SCS, SO, SI, LS2, LS3, SS2, SS3). Shifting columns
  sideways (SL, SR), the DEC locator (DECELR, DECSLE, DECRQLP, DECEFR: the mouse answers as a
  locator when a program asks for it) and the state reports (DECRQPSR for the cursor and the tab
  stops, DECRQTSR, with DECRSPS and DECRSTS to put them back, DECRQDE) are understood too. The
  device attributes (DA1) now list exactly these and the rest of what the engine does.

### Changed

- Wormhole runs on its own terminal engine, written from the specifications, in place of
  alacritty's. It reads program output about three times as fast (2.85 times alacritty's core in
  the engine benchmark), and in our comparison of seven terminals on the same Mac the app is the
  fastest overall, six times as fast as Terminal.app. It passes every case of its own conformance
  suite, 544 of 567 esctest2 tests and every ucs-detect category, and adds what alacritty's core
  lacked: left and right margins (DECLRMM), selective erase, every DEC character set, setting and
  terminfo reports (DECRQSS, XTGETTCAP), and pictures that live in the grid, so kitty, Sixel and
  iTerm2 images scroll and reflow with the text.
- Text is sized as Unicode 18 says, and the sizing matches ucs-detect's in every category. A
  conjunct of an Indic script or Khmer, a base with a spacing vowel sign (Bengali, Burmese,
  Javanese, Thai, Lao and more) and the Arabic number sign take the cells they are drawn in, and
  an emoji followed by the text selector (a watch or a heart with U+FE0E) steps back to one cell.
- Grapheme clustering (mode 2027) is on from the start, as in Ghostty, and after a reset. Emoji
  sequences, flags and the scripts above are one character of their own width to the terminal
  without the program asking; a program that counts code points can turn it off with
  `CSI ? 2027 l`.

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

[Unreleased]: https://github.com/wormhole-terminal/wormhole/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/wormhole-terminal/wormhole/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.2...v1.0.0
[0.4.2]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/wormhole-terminal/wormhole/releases/tag/v0.4.0
