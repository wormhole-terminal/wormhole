# Changelog

All notable changes to Wormhole are listed here. The format follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in
[CONTRIBUTING.md](CONTRIBUTING.md#versioning).

## [Unreleased]

### Added

- Column dividers: the lines between the sidebar, the terminal and the right panel are their own
  `divider` colour and `metrics.divider_width` in a theme (both follow the border unless set; a width
  of 0 draws none), and Settings → Appearance → Column dividers hides them in every theme. The edges
  still resize, and Increase Contrast keeps the lines.
- Profiles: a whole Wormhole of its own for Work, Private or anything else, each with its terminals,
  theme, fonts, keys, notifications, trusted projects, start folder, environment, git identity and
  its own Claude and Codex login. Settings → Profile (the first page) edits the one you're in and
  makes new ones. Shell → Profile, the command palette ("Switch to Work") and a chip at the top of
  the sidebar (shown once there are two) switch profiles, open a window in one or move this window
  to one. Switching hides the other profile's windows, their terminals and agents still running; a
  window in another profile than the one shown wears its colour along the top and its name in the
  status bar. A folder listed under a profile opens in it, `Wormhole list` names each terminal's
  profile, and Settings → Coding agents installs the status hooks into a profile's Claude or Codex
  login too.
- Groups of your own, as in a browser's tab groups: name them, give them one of nine colours and
  keep them in your order among the terminals in no group. Click a group's name for its popover
  (name, colour, New terminal in group, Ungroup, Close group, Pin group, Move group to new window)
  and its chevron to fold it; a folded group keeps its terminals' tiles, their count and a dot while
  one needs you. Drag terminals into and out of groups, and groups by their head. A pinned group
  stays when its last terminal closes. ⌘T opens a terminal in the selected one's group, ⇧⌘T in no
  group, and a terminal split off a grouped one joins its group. Close group asks first if an agent
  is working in it.
- Terminals can show as tabs instead of in the sidebar (Settings → Terminals & sidebar → Show
  terminals in, or View → Show Terminals as Tabs): one row under the title row with the profile
  chip, a group switcher (with an amber dot while another group needs you) and that group's
  terminals. As in macOS Terminal there's no tab bar while there's only one terminal. The switcher
  searches groups and terminals ("Go to group or terminal…"); tabs keep their place, show their
  close button under the pointer and scroll sideways when they don't fit. ⌃⌥[ and ⌃⌥] go to the
  previous and next group and ⌃⌥1–9 to a group by number, in either layout.
- Ten themes of Wormhole's own. Wormhole, in the app icon's colours, is the new default: Wormhole
  Dark in Dark Mode and Wormhole Light in Light Mode; an existing setup keeps the theme it has. With
  it come Nebula, Solar Flare, Borealis, Event Horizon, Void (true black), Crimson, Cyberpunk, Matrix
  and Soltari. Each uses its own primary colour where it has a signature one, and every colour keeps
  at least 5:1 contrast on its background.
- A theme file can hold several versions of a theme (Light, Dark, Light Minimal, …): shared colours
  at the top and a `variants` list in which each version sets only what differs. Each version is a
  theme of its own ("Soltari Light"), the family's name means its first version, and the Theme Studio
  saves a version back into its family's file. Settings → Appearance shows a family as one card with
  a swatch per version.
- Settings → Updates → Channel: Unstable gets a build of every change as soon as it passes the
  tests, and shows Settings → Feature flags so you can try what's still being built. Stable, the
  default, gets releases only; going back to Stable keeps your build until the next release
  overtakes it.
- The Usage tab shows Codex's plan limits and Gemini's latest token count next to Claude's, and an
  expired reading waits for a fresh one instead of showing an unused allowance. Turn the integrations
  on or update them in Settings → Coding agents.
- Liquid Glass, off by default (Settings → Appearance): on macOS 26 and later the sidebar's cards and
  the tabs are glass and the status bar uses the window's own material. Before macOS 26, or with
  Reduce Transparency or Increase Contrast, every surface paints itself.
- Programs can ask whether the theme is light or dark (`CSI ? 996 n`) and ask to be told when it
  changes (mode 2031), so editors and TUIs that follow the terminal's colours can switch with it.
  Wormhole answers from the background of the theme in use, and reports a theme change to the
  programs that asked.
- Programs can draw double-width and double-height lines (`ESC # 6`, `ESC # 3`/`ESC # 4`, back to
  normal with `ESC # 5`), as vttest and banner tools do. A line of such a row takes twice the width
  (and a double-height line two rows at twice the size); selecting, clicking, the cursor and
  find work on it. Double-width text keeps its letter shapes and is spaced out to fill the wider
  cells.
- A program can ask to be told the terminal's size in characters and in pixels, once when it asks and
  again whenever the window changes size (in-band resize notifications, mode 2048), instead of
  waiting for a signal.
- A program that turns on xterm's modifyOtherKeys (`CSI > 4 ; 1 m` or `; 2 m`) is told which
  modifiers you hold on ordinary keys: Alt+key, Ctrl on the punctuation and digit keys with no
  control code, and Enter, Tab and Escape with a modifier arrive as `CSI 27 ; modifiers ; key ~`
  sequences as in xterm. Shift+Enter is then sent as Shift+Enter, not as Option+Enter.
- A program can save and restore the pen's attributes and the colours it changed (xterm's
  `CSI # {`, `CSI # }` and `CSI # P`, `CSI # Q`), and read back what a rectangle of the screen has in
  common (`CSI Pt ; Pl ; Pb ; Pr # |`).

### Changed

- The line that joins a split's terminals in the sidebar runs through the middle of their tiles,
  behind them, instead of beside them.
- Notes and tasks in the right panel are hidden for now, behind a feature flag in development builds.
- The Usage tab of the right panel shows while a coding agent runs in the terminal.
- Settings: the previews are always in view. The window opens wider, to the right, with a preview
  column beside the page: the window in miniature (Appearance and every page without a preview of its
  own), the font specimen on Fonts, and the Terminals list on Terminals & sidebar, each following
  what you change as you change it. The page keeps its width; in a window narrower than that, or on a
  small screen, the previews sit in their pages as before.
- Settings: a calmer look. The nav's rows are 13 pt and inset from its edges with a rounded
  selection, each group of sections starts a clear step below the one before it, and "Project
  folders" no longer cuts off against its Preview tag. Pages have more air at the sides, an intro
  that wraps at a readable width and group headings in a larger, heavier type than the rows under
  them. The window opens a little taller so the whole nav shows.
- Right-click menus (a terminal's, a group's, the + button's shells, a pane's, Open in…) are the Mac's
  own menus now, with VoiceOver, type-to-select and the system look.
- The terminal engine, `wormhole-vt`, is now under the MIT License, so other projects can use it. The
  app stays GPL-3.0-or-later.
- A new app icon: the `>_` prompt over a glowing disc that runs from blue to violet, on a plain tile,
  white in light mode and black in dark mode.
- About Wormhole looks like the Mac's own About panel: a small window with the icon, name, version
  and copyright. The licences moved to Help → Licenses; Check for Updates stays in the Wormhole menu.
- A new look for the window. The terminal is a rounded surface, its text 12 pt from the window's edge
  in line with the traffic lights, the title row, the status bar and the sidebar's and panel's
  headers; the title row stands on the terminal's own colour. Split terminals are rounded surfaces of
  their own, 8 pt apart: the one with the keyboard has a quiet outline and the others are dimmed, the
  gap resizes them, and Settings → Terminal → Gaps between split terminals joins them with a hairline
  instead. Each pane's header names its terminal as the sidebar does and leads with its status tile.
- The sidebar is a list of rounded cards: one per group and one for each terminal in no group (a
  split's terminals share one), with the selection colouring the card itself. Its heading is gone and
  its + sits at the right of the top row; a long list scrolls with a scrollbar that follows "Show
  scroll bars" in System Settings. Each
  row leads with a status tile that looks like the agent's app icon (or a terminal's): a light circles
  its edge while it works, its amber edge breathes while it needs you, and a badge marks done, failed,
  new output or the bell. Under Reduce Motion they hold still.
- Settings → Terminals & sidebar has three row densities, Regular, Compact and Dense, for the sidebar
  and the tab bar, and "Show groups as cards" turns the cards off. View → Compact Rows becomes the
  Compact density.
- A group whose terminals share a repository shows its changed files (±N) once, on its first row; a
  terminal in another repository or worktree keeps its own count.
- The right panel's Changes, History and Notes are cards like the sidebar's, under its own tab bar,
  with the compare picker and refresh in the Changes card's head.
- The status bar puts the terminal that has waited longest for you first, with how long it has waited
  (a click goes there), and counts the rest ("2 working · 1 done"). It names the program in front
  instead of the shell and says how the terminal on screen is doing ("needs you", "done · 2m"); the
  branch, its changes and the agent's context stay in the title row.
- Settings → Appearance → Use the theme's primary colour, on by default. Turn it off to use your own
  primary in every theme; turn it back on and the theme's returns, with yours kept for next time. A
  colour you had already chosen stays in use.
- Light themes are easier to read: cards keep a visible edge on white, the amber of a terminal that
  needs you holds 3:1, and a profile's letter turns black on a light colour.
- Reading the window title back is now a setting of its own, off by default (Settings → Terminal →
  Privacy, "Let programs read the window title back", or `allow_title_report` in `config.json`). It
  used to come with "Let programs control the window", so if you turned that on and a program needs
  the title, turn this on too. A title that a program sets can no longer carry control characters
  into the window's title, the sidebar or a terminal's folder.

### Removed

- Grouping by folder, and with it the sidebar's filter (Show, search, Hide idle), Group by, its Settings
  gear and shell menu, and the Order setting: terminals keep the order you give them. Right-click the +
  for another shell. The tab bar's two-row, stacked and bottom-bar layouts are gone too. Settings that
  named any of these still load and do nothing.

### Fixed

- The Changes and History tabs of a terminal whose folder is in no git repository no longer blink
  every couple of seconds ("Reading git status…" flashed in over the note while a poll ran, and
  History redrew on each poll). Git is no longer asked about such a folder again and again; a cheap
  look for a `.git` in the folder and its parents runs instead, so `git init`, `git clone` into the
  folder or a `cd` into a repository shows the repository within about half a second.
- A crash is written down: the log gets the panic's message and where it happened, and
  `logs/last-panic.txt` keeps the last one, so a crash of an app started from the Finder leaves more
  than a crash report.
- A screen that keeps printing new words, such as `ttystorm donut` with a font that has ligatures, no
  longer slows the window down: a frame took 30 ms there and takes about 3 now. A word with no time to
  be shaped shows without its ligatures for a frame or two.
- A program can no longer freeze a terminal for seconds by moving the cursor far left over and over
  with reverse wraparound on.
- A terminal's scrollback stays near its memory limit when the output is full of combining marks or
  links, which could make it hold hundreds of megabytes. Past 1 MiB of link addresses in one stretch of
  history, new links show as plain text.
- After a program turns on 8-bit controls, a title or other text with a dash, curly quotes or `œ` in it
  is no longer cut short, with the rest shown as stray characters.
- A program that asks for thousands of reports without reading them no longer gets half an answer typed
  into its input when Wormhole stops answering.
- A saved terminal state that a program sends back with a mistake in it now changes nothing, instead
  of putting back part of it.
- A protected wide character (Chinese, Japanese, an emoji) at the edge of a selective erase is no longer
  erased with the rest.
- A line that ends in a tab no longer takes an extra, empty row when the window gets narrower.
- Dragging on the tab bar no longer moves the window.
- Scrolling the Theme Studio is smooth again: the component gallery and the token list draw only what
  is in view, each part of the window redraws on its own, and a turning status spinner no longer makes
  the whole window redraw. The icons of Settings → Feature flags and of the palette's profile
  commands, which were missing, show.
- A program can now switch on the application keypad, X10 mouse reports (mode 9), URXVT mouse reports
  (1015), pixel-position mouse reports (1016) and Backspace sending BS (DECBKM), and the terminal does
  what it asks. Until now only the other mouse modes and the cursor keys were followed.
- Pixel-position mouse reports (mode 1016) count from 1, as xterm's do, instead of from 0.

### Security

- **Low** · no advisory · affects 1.0.0–1.1.0: a program could put text of its choosing on your command
  line. Wormhole's answer to a clipboard read repeated whatever the program named as the clipboard, so a
  file shown with `cat` could type a command at your prompt (it did not run it: the text could not
  hold a Return). Only the names of real clipboards come back now.

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
