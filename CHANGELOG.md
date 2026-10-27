# Changelog

All notable changes to Wormhole are listed here. The format follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/),
and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html) as described in
[CONTRIBUTING.md](CONTRIBUTING.md#versioning).

## [Unreleased]

### Added

- Copying says so in a small chip that drops onto the top of the terminal, in the system's own glass:
  "Copied 3 lines", "Copied command", "Copied path". A program's copy in the terminal you're typing in
  is said there too, instead of the bar over the terminal's last rows. VoiceOver reads it out, and
  Reduce Motion fades it in instead.
- Pi (pi.dev) is recognised as a coding agent: its logo in the sidebar, and Settings → Coding agents
  installs a small extension so its terminal shows when it is working, waiting on a question, done,
  stopped or failed.
- Codex, Amp, goose and Kiro show their own logos instead of two letters.
- Codex, Kiro, Qwen, Crush and Aider sit on their own colours in the sidebar, as Claude and Gemini already do.
- `fm chat`, the chat with Apple's on-device model that comes with macOS 27, is recognised as an agent
  (Foundation Models). It has no hooks, so its status is read from the screen. `fm respond` in a
  script and `fm serve` aren't taken for an agent.
- OpenCode's terminal shows that it needs you when it asks a question, not only for a permission.
  Repair OpenCode's hooks in Settings → Coding agents once to get it.
- The first run's Look step asks "How much to show" one question at a time, as a shell prompt's setup
  wizard does: where to start (Zen, Balanced or Everything), then the title row, the status bar, the
  minimap, the lines between the columns and the right panel with its tools. Each answer is a small
  drawing of the window it would give; click one or press 1, 2 or 3. Every answer is applied at once,
  Back and Start over go around, and a summary ends the walk.
- `features` in config.json switches the right panel's optional tools off and on (`"history": false`);
  Changes and Usage always stay, and a key that switches nothing is said so. Settings has no page for it
  yet; the first run's walk through how much the window shows will set it.
- Settings → Import brings your font, colours, shell and more over from Ghostty, iTerm2, Terminal.app,
  Alacritty, kitty, WezTerm, Warp, Zed and VS Code, lists what doesn't carry over, and shows how each
  terminal compares (sources checked 2026-10). The first-run "Look" step, in the setup window and in
  `Wormhole welcome`, offers "Use my … settings" when it finds one. Nothing of the other terminal is
  changed or run.
- Update checks let us count how many copies of Wormhole run, without telling one from another. A
  check now names the update channel, Apple silicon or Intel and the macOS major version, and the
  first check of each day and of each month says so; counting those gives the copies running each
  day and month. There is no ID, so two checks from one Mac can't be linked, and the server keeps
  only the counts, not your IP address. Counting starts a day after this version first runs, so
  there is time to turn it off: Settings → Updates → Count this Mac anonymously (`anonymous_count`),
  which also shows exactly what a check sends. Off, a check sends the version alone, as before.
- Settings → Updates has a Privacy group: why update checks are counted, the switch, exactly what a
  check sends from this Mac and what it never sends, and how many hours are left before the first
  count.
- Each release has a download for Apple silicon and one for Intel Macs besides the universal one, and
  an update downloads only your Mac's architecture, about half the size it was.
- When Wormhole's download server doesn't answer, an update check asks GitHub, where the releases are
  published, so updates keep coming. That check sends the version alone and is not counted.
- Fourteen new Wormhole themes in the website's accent colours: Electric Blue, Vivid Blue, Klein Blue,
  Cobalt, Azure, Sky, Ultramarine, Volt, Signal Orange, Hot Pink, Ultraviolet, Acid Cyan, Red and
  White. Each comes in a light and a dark version, built on Wormhole's own palette.
- Settings → Appearance → Branch colour picks the colour of git branch names and their chips: the
  theme's own, your primary colour, or any colour you choose (`branch_color`, `branch_color_custom`).
  A colour that wouldn't read on the window is made readable, as the theme's is.
- You choose what the title row shows and where. Right-click the title row to show or hide each part:
  the folder, the branch and its status, the terminal's name and status, the agent's model and context,
  open ports, the pull request, Open in… and the split buttons. Settings → Appearance → Title row lists
  them too: drag a part to move it, and the parts above the flexible space sit at the row's left, the
  others at its right (`title_row`). The four switches for the row's extras became this list; what you
  turned off stays hidden.
- Icon buttons answer with their own drawing. The sidebar and right panel buttons show whether
  their column is open: the column's strip in the icon is filled while it is, drains as the column
  slides away and fills back in as it returns. The + turns, Refresh and Reset spin their arrows,
  Copy fills and turns into a check, the split buttons fill the pane they make, the pane zoom's
  corners swap, and the chevron beside Open in… points up while its menu is open. A right panel
  tab's icon plays once as the tab opens. Settings → Appearance → Animate icons turns all of it
  off, and Reduce Motion does too.
- `wh` prints JSON by default when `WORMHOLE_JSON=1` is set; `--text` asks for text again.

### Fixed

- A working terminal's light circles its tile smoothly. It moved in uneven steps, about 20 to 24 a
  second; now it moves 60 times a second, evenly, and the sidebar's spinners and pulses keep a steady 30.
- `wh new-terminal --command` (and a command given to a new window or split) now runs: it is typed once the
  shell shows its prompt, where it used to be typed at once and was dropped while a terminal kept running
  behind the app had no shell yet, or lost to a shell still reading its startup files.
- In bash, a command's block (its gutter mark, ⌘↑ and `wh blocks`) no longer disappears after the window
  is resized: a prompt the shell draws again without a new start mark stays the prompt.
- A Claude Code terminal no longer shows finished (with a banner and a sound) while its agent waits
  on work it started in the background: a `Monitor`, or a build or test a subagent runs in the
  background. It stays working until that work wakes the agent. A server the agent leaves running on
  purpose still lets the turn count as finished.
- An update that downloaded but failed to install (for example, without permission to replace the app)
  is no longer downloaded again at every automatic check. Settings → Updates offers it with Install
  update, and the next launch tries once more.
- The keyboard shortcuts sheet names only the panel tabs there are: with notes and project folders off,
  its row reads "Changes · History · Usage" with each key, instead of naming tabs that don't show.
- The Theme Studio no longer shows notes and tasks, or a project's trust prompt, while they are hidden.
- A newer `wh` talking to this Wormhole about something it doesn't know gets an answer at once instead
  of waiting ten seconds.
- `wh send-text` no longer types into the terminal it runs in just because `WORMHOLE_TAB_ID` is
  missing or wrong: the terminal is told by where the command runs (its place under the terminal's
  shell, or on its terminal device), not only by what it says about itself. A program that starts a
  session of its own can still hide where it runs.
- `wh send-text` types into a terminal whose program only rang the bell. It is still refused while a
  coding agent waits for your answer (a permission prompt or a question), as it always was meant to be.
- A working terminal's light, and the breathing amber edge of one that needs you, no longer start over
  when its row is drawn again (a row moving, a status changing); they go on from where they were.
- With more than one screen, a program's request to move its window (`CSI 3 ; x ; y t`) puts the window on
  the screen it is on, at that place of it, not at the place of the first screen listed.
- With more than one screen, a new window opens on the screen of the window it comes from, and a
  window comes back on the screen it was on, instead of always on the main screen.
- A terminal moved to a window of its own with its group listens to that window's focus, not the old
  one's: a program that asked for focus events hears it when the new window is active, and the cursor is
  filled by it. Its find bar and font size follow it too.
- A terminal moved to another window with its group is saved under its new window at once, so its notes,
  tasks and text are not forgotten if the old window saves first and Wormhole stops before the new one does.

## [1.3.0] - 2026-10-25

### Added

- A group can have a folder: "Set folder…" in its menu or the popover its name opens picks one, and
  new terminals in the group (its + and ⌘T while one of its terminals is selected) start there. The
  folder shows when you rest on the group's name and in its menu; "Clear folder" takes it away, and
  a folder that has gone is skipped. A group with a folder is pinned, so it stays when its last
  terminal closes, like a project; clearing the folder leaves it pinned.
- A + shows on every group's head while the pointer is over it (not only on an empty, pinned
  group): it opens a new terminal in the group.
- A group with a folder shows it under its name on its head (`~/Projects/app`, cut from the front when
  long; after the name in Dense). Settings → Terminals & sidebar → Show group folders turns it off.
- A group's colour can be taken away: "No colour" in its menu (checked when it has none), and an
  empty ring with a slash first among the colours in the popover its name opens.
- Settings → Terminal → Restore terminal text after restart (on, as before): off, Wormhole no longer
  saves the last lines of your terminals to show them again after a restart, and erases what it has
  saved. For one terminal, "Don't keep this terminal's text" in its menu in the sidebar does the
  same.

- Settings → Appearance: any theme, built-in or yours, can be hidden from the theme picker with
  the eye on its card or from its menu (right-click), and "Hidden themes" under the cards brings
  each back with Show. A theme you imported can also be moved to the Trash from its menu.
- Settings shows a preview that belongs to the page: Coding agents the Terminals list with agents
  working, asking and finished, Notifications two banners, Menu bar & hotkey the menu bar item and
  the quick panel. Pages with nothing to preview have no preview column and use its width.

### Changed

- In the tab bar (Settings → Show terminals in → Tabs), the terminals in no group are called
  "Ungrouped" in the group switcher, and a group with a single terminal still shows its tab beside
  the switcher. With one terminal and no groups there is still no bar.
- A group's colour fills its card evenly now. Settings → Terminals & sidebar → Fade group colours
  brings back the fade, where the colour is strongest at the card's left edge and fades out across it.
- A group's colour is stronger on its card (22 % of it, up from 12 %). Where the card's text would
  stop reading on that much of a bright or dark colour, the card gets the colour a shade nearer the
  card's own lightness, so names, second lines and the amber of a terminal that needs you stay clear
  in every theme.
- Lists and panels bounce a little at their ends when you scroll past them with a trackpad, as in
  other Mac apps.
- Full-screen programs that draw each frame as a synchronized update (many coding-agent interfaces
  and animations do) redraw the terminal less often, so they use less CPU.
- A frame in which only a terminal's output changes no longer builds the sidebar, the side panel, the
  title row or the status bar again, which takes about a quarter off the time a full-screen animation's
  frame spends drawing the window.
- The terminal's text stays where it is when a side column opens or closes or the window is resized:
  the grid keeps its place at the top left and what doesn't make a whole cell goes to the right and
  bottom edges, instead of the text shifting sideways by up to half a character.
- Notes sync with a project's notes.md now needs the project to be trusted. Until then the file is
  neither merged into the terminal's notes nor written back, and the Project tab says it is waiting
  for trust.
- A terminal waiting for an answer is now shown as "Needs input" instead of "Needs you": in the
  sidebar, the status bar, the menu bar, the palette, notifications, VoiceOver and Settings. The
  `Wormhole list` and `status` command line keeps saying `needs you`, so scripts that read it still
  work.

- Settings → Appearance: the mode decides which themes show. Dark or Light shows the themes of
  that kind; Match system shows two rows, "In Dark Mode" and "In Light Mode", each with its pick
  checked. The Dark/Light filter and the moon and sun marks are gone, and a theme with dark and
  light versions (Wormhole) has a card in each row, with its name in full.
- A setting's hover fill reaches a little past its text on both sides, so the words no longer
  touch its edges.

### Fixed

- Choosing a theme card, recording a shortcut or picking a colour in the Studio no longer moves
  what is inside by a point: the ring is drawn round the edge instead of thickening the border.
- Settings → Coding agents shows each agent's logo where Wormhole has one, instead of two letters.
- Settings → Appearance's preview is the window's own parts at their real size (the Terminals
  list, the title row, the terminal in your font, the status bar) instead of a drawing of them, so
  it always shows the app as it is now; the column shows the window's left part.
- Liquid Glass is a feature flag now (Settings → Feature flags, in development builds and on the
  Unstable channel) instead of a switch in Settings → Appearance. Release builds draw no glass
  until it is finished; a `liquid_glass` left in config.json says where it went.
- Across Settings, text links ("Edit in Fonts", "Choose app…") line up with the edge of the page,
  the Coding agents page's arrow and icons sit before the column so the words line up with the
  rows, and the Profile page's name, environment and agent logins put their label beside the first
  line of the fields.
- The window's traffic lights now match the theme when it differs from the system's Light or Dark
  Mode, instead of showing the other mode's lights on the title row.
- The History tab's commit ages ("2h ago") keep up with the clock while the tab is shown, instead of
  staying as they were when something last redrew it.
- A program that exits in the middle of a synchronized update no longer leaves its last screen
  undrawn.
- Fast output of long lines with a colour per character (gradients, colour floods) draws several
  times faster.
- A window of a profile that is not the one shown now follows its own profile's project approvals
  instead of the shown profile's. A project folder you trusted in one profile no longer runs its
  actions, environment, worktree setup or pull request lookup in a window of another profile, and one
  trusted only in that other profile works there. A window of another profile no longer restyles the
  shown profile's windows with its project's look, and the menu bar's project actions run in a window
  of the shown profile.
- Quitting Wormhole with `kill` (or anything else that sends it SIGTERM) now saves every window and
  the terminals' text first, as ⌘Q does, instead of losing what changed since the last save.
- Closing Settings while it records a shortcut no longer leaves the global hotkey switched off until
  Settings opens again, and the menu bar no longer shows it as set while it is off.
- The View menu's Project line no longer shows, greyed out, while project folders are off.
- The text of a closed terminal stayed on disk. Wormhole saves each terminal's recent text so it can
  come back after a restart; when the terminal was closed, the text was deleted from the database but
  its bytes stayed in the file's free space and in its journal, where any program running as you could
  read them, along with whatever secrets were printed. Deleted text is now overwritten and the journal
  emptied, and what earlier versions left behind is erased the first time this version opens the
  database. Backups made before the update (Time Machine, for example) still hold that text.
- Clearing a terminal with ⌘K left the copy of its text that Wormhole saves on disk for up to a
  minute, so a secret you cleared right away could still be read from the file or come back after a
  restart. Clearing now erases that saved copy at once.
- Copies of the database that Wormhole keeps before an upgrade or when it sets a damaged one aside
  were never deleted, and each held every terminal's saved text as it was then. Wormhole now keeps the
  newest copy from before an upgrade for two weeks, in case you need to go back, and deletes the
  others, and any copy older than two weeks, at launch.
- Wormhole's data folder and the files in it (the database with your terminals' saved text, settings,
  profiles and their environment, logs) could be read by other accounts on the Mac wherever the folder
  wasn't already protected by macOS, for example when it was moved or copied elsewhere. Wormhole now
  makes the folder and everything in it readable by you alone, and fixes an existing folder at launch.
- The text Wormhole saves from each terminal, so it can come back after a restart, was kept in the
  same file as your windows and notes, so Time Machine backed it up along with any secrets that were
  printed. The text now has a file of its own that backups skip, and the copy of the database Wormhole
  keeps before an upgrade no longer holds it. Copies made by earlier versions keep their text until
  Wormhole deletes them, within two weeks, and backups made before the update still hold it.
- The text Wormhole saves from each terminal was stored as plain text, so any program running as you
  could copy the file and read whatever secrets had been printed (tokens shown by `env`, `cat .env` or
  `gh auth token`, for example). The text is now encrypted with a key Wormhole keeps in your login
  Keychain, so a copy of the file is useless without it. If the Keychain can't be used or you deny
  access, the text isn't kept at all rather than saved unencrypted.
- The review comments you paste into a terminal for a coding agent named each file as the repository
  does, and a file name can hold line breaks and control characters. They went into the pasted message
  as they were, and where the program in the terminal takes no paste as text and the message is pasted
  by hand into a shell, each line of such a name would run as a command. Wormhole now leaves control
  characters out of the file name, as it does out of the quoted code.

## [1.2.0] - 2026-10-22

### Added

- Submodules get their own rows in Changes and History, with a folder-and-git icon and what changed
  in words ("2 new commits", "1 commit back", "uncommitted changes", "not checked out") where a file
  has line counts. A click lists the commits between the commit the repository records and the one
  the submodule is at, newest first, with each commit's subject, hash, author and age (and the
  commits left behind when the pointer went back), instead of git's two `Subproject commit` lines.
  The button on a row opens a terminal in the submodule.
- A terminal inside a submodule says whose it is in the title row ("in terminal").
- A small icon opens a file of the git panel in the editor chosen for Open in…: on the right of a
  Changes row while the pointer is over it, over a diff line (the file opens at that line, where
  the editor can: VS Code and its forks, Zed, Sublime Text and the JetBrains IDEs), in the diff's
  header, and on the files of a commit in History while they are still in the working tree. With
  Finder chosen it shows the file there.
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
  find work on it. Double-width text is stretched to twice its width (bitmap emoji
  keep their size and are centred in their cells).
- A program can ask to be told the terminal's size in characters and in pixels, once when it asks and
  again whenever the window changes size (in-band resize notifications, mode 2048), instead of
  waiting for a signal.
- A program that turns on xterm's modifyOtherKeys (`CSI > 4 ; 1 m` or `; 2 m`) is told which
  modifiers you hold on ordinary keys: Alt+key, Ctrl on the punctuation and digit keys with no
  control code, and Enter, Tab and Escape with a modifier arrive as `CSI 27 ; modifiers ; key ~`
  sequences as in xterm. Shift+Enter is then sent as Shift+Enter, not as Option+Enter.
- Programs written for the VT52 work: `CSI ? 2 l` switches to VT52 mode (cursor movement, direct
  cursor addressing, the graphics set, the identify request), with its cursor and keypad keys, and
  `ESC <` switches back.
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
- The folder's hover card and a group's popover are the Mac's own popovers now. Resting on the folder in
  the title row opens a card with the path, Open in Finder, Copy path and Insert into terminal; a
  click on it still opens the folder in Finder, and a right-click opens a menu like the one a Finder
  window's title has, with the folders down to the disk and their Finder icons. Clicking a group's
  name opens a card with its name in a field, the nine colours and the actions; a colour wheel after
  the colours opens the Mac's Colours window for a colour of your own, which the group follows as you
  choose it and which is kept with the group (the group's right-click menu has Other colour… too, and
  Rename… edits the name in place on the group's head).
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
- Edit → Rerun Command is now Put Command at Prompt: it types a finished block's command at the
  prompt and leaves the Return to you, and says why in a notice over the pane when it will not.

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
- The Open folder in… menu in the title row shows each app's icon again (Zed, Xcode, Finder …), as
  the Mac gives it to Finder.
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
- A repeat (`CSI Ps b`, REP) now repeats a character only when it comes right after one, as the
  standard and xterm have it: after a colour change, a cursor move, a line feed or another repeat it
  repeats nothing, where it used to repeat the last character printed.
- Text a program writes to the status line (`CSI 2 $ ~`, then `CSI 1 $ }`) is no longer drawn on the
  main screen at the cursor: Wormhole has no status line, so it drops what is written there and the
  screen and cursor stay as they were.
- Programs that ask a terminal for its line parameters (`CSI 0 x`, DECREQTPARM, which some old
  VT100 software still does) now get an answer instead of waiting for one.
- A program can switch the terminal between 80 and 132 columns with `CSI 80 $ |` and `CSI 132 $ |`
  (DECSCPP), without clearing the screen, like the other window resizes it may ask for.
- A program can ask where it is on the title stack (`CSI # S`) and read back its modifyOtherKeys level
  and title modes with a status string request (`DCS $ q > 4 m ST`, `DCS $ q > 0 t ST`).
- A terminal that a program has put back to the VT100 level (by leaving VT52 mode, or `CSI 61 " p`) no
  longer answers status string requests (DECRQSS), as a real VT100 doesn't.
- With ISO Latin-1 in a 96-character set slot (`ESC - A`, `ESC . A`), DEL now shows `ÿ`, the last
  character of the set, where it was dropped.
- Return sends CR LF instead of CR while a program has new line mode set (`CSI 20 h`), as on a VT100.
- A terminal whose program is a short command rather than a shell (a shell profile that runs one, a
  program that fails at once) could stay empty, its output, often the error that explains why, lost:
  on macOS the system throws away what a program wrote when its terminal closes before it is read,
  and a short command is gone before the terminal's reader gets to it. The terminal now keeps its
  end open until the output is read.
- A program could hide text in a notification. Wormhole cleaned notification text of control
  characters and some invisible ones, but let through characters that draw nothing (Hangul and
  Mongolian fillers, the combining grapheme joiner, tag characters and variation selectors), which can
  carry a message you cannot see or make a line read differently from what it holds. They are dropped
  now, except the one emoji or text selector that follows a symbol. A malformed percent escape in a
  folder a program reports (`%+F`) is refused too.
- A program could freeze a terminal for seconds with a few kilobytes of output. The 80/132-column
  switch (`CSI ? 3 h` and `l`, after `CSI ? 40 h`) rewrapped the whole scrollback twice each time, so
  a file shown with `cat` that toggled it thousands of times stalled the pane. The switch now clears
  the screen and homes the cursor without changing the width, unless "Let programs control the window"
  is on.
- A program could put text of its choosing on your command line. Wormhole's answer to a clipboard read
  repeated whatever the program named as the clipboard, so a file shown with `cat` could type a
  command at your prompt (it did not run it: the text could not hold a Return). Only the names of real
  clipboards come back now.
- A program could read your clipboard. A program in a terminal, also one over ssh or a file shown with
  `cat`, could ask for the clipboard with OSC 52 and got its text at once, which is often a password.
  Now a bar over that terminal asks first, naming the program: Allow once, Always for this terminal
  (until it closes) or Deny. One question at a time, none for a terminal you cannot see, and 30
  seconds without an answer deny. Settings → Terminal → Privacy has "Programs reading the clipboard"
  (`clipboard_read`: ask, allow or deny; a project cannot set it). A program setting the clipboard is
  still allowed, now with a notice over the pane saying how much it copied, and "Programs setting the
  clipboard" (`clipboard_write`) turns it off.
- A program could make Rerun Command run something other than the command you saw. The command is read
  back from the screen, and a program can draw text so it cannot be seen (concealed, in the
  background's own colour, or as characters that draw nothing), so a block that looked like `ls` could
  hold `ls; rm -rf ~`; Rerun typed it and pressed Return. Put Command at Prompt (the renamed action)
  refuses a command with any such text, and never presses Return.

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

[Unreleased]: https://github.com/wormhole-terminal/wormhole/compare/v1.3.0...HEAD
[1.3.0]: https://github.com/wormhole-terminal/wormhole/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/wormhole-terminal/wormhole/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/wormhole-terminal/wormhole/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.2...v1.0.0
[0.4.2]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.1...v0.4.2
[0.4.1]: https://github.com/wormhole-terminal/wormhole/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/wormhole-terminal/wormhole/releases/tag/v0.4.0
