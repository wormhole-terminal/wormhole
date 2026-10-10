# Wormhole

A native macOS terminal built for working with AI coding agents like
Claude Code, Codex, Gemini CLI and many more. Written in Rust on [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) (Zed's UI framework)
and GPUI Kit, with alacritty's terminal core.

**[Download Wormhole](https://getwormhole.app)** (macOS 14 or later, Apple Silicon). It updates itself.

## Features

- **Terminals sidebar** – every terminal (one shell, one row) is named after the agent's
  title or folder, with live status: working,
  finished, or waiting for you. Rows show
  the status dot, the logo of the agent running there, git branch, changed
  files and open notes (each can be switched off in Settings → Terminals & sidebar).
  The funnel next to “+ New” groups the list by folder (the git repository a
  terminal is in, else its working directory); ⌘1…9 count terminals in the order listed. A
  folder's band names its path, its changed files and its branch under its name. Right-click a
  band to pin the folder: its band then stays at the top of every window's list, even with no
  terminal in it, and its + starts one there; or to give it a color, which tints its band and
  its terminals' title row (the same eight colors are in the Project tab, which can also create the
  folder's `.wormhole/`).
- **Splits and windows** – there are no tabs: every terminal is a row of the sidebar. ⌘D and
  ⇧⌘D split the one you are in (the agent beside a dev server, say); each pane is a terminal of
  its own, with its own notes, tasks and status, and the rows of a split are joined in the list.
  Open as many windows as you like,
  each with its own terminals; everything comes back after a restart, windows in
  place. Closing the last window quits (saved); closing another one ends its
  terminals, and asks first when notes, tasks or a working agent would be lost.
- **Shells** – zsh, bash, fish, nu or any program as a profile (a command with its
  arguments, in `config.json`). Pick the default in Settings → Terminal; open a
  terminal with another from a right-click on the +, the Shell menu or the command
  palette. A terminal remembers its shell across a restart.
- **Command blocks** – with shell integration (OSC 133) every command is a block:
  a green or red mark in the gutter, ⌘↑ / ⌘↓ between prompts, the exit code and
  time on hover, and Select / Copy Output, Copy Command and Rerun from the Edit
  menu.
- **Inline images** – kitty's graphics protocol, iTerm2's inline images and
  Sixel show pictures in the grid that scroll and reflow with the text (`icat`,
  `imgcat`, `chafa`, file managers' previews).
- **Many coding agents** – Claude Code, Codex CLI, Gemini CLI, GitHub Copilot
  CLI, Cursor Agent, OpenCode, Aider, Amp, Goose, Qwen Code, Crush, Droid,
  Auggie, Kiro and Cline are recognised from the terminal's foreground process.
  For Claude Code, Codex, Gemini, Copilot, OpenCode, Goose, Qwen Code and Droid
  the status is exact once their status hooks are installed (Settings → Coding
  agents); the others are tracked from their output.
- **Notes & next steps per terminal** – collapsible notes and a task list. Stash
  the prompt you were typing (⌘⇧S), answer the agent, then send it back
  (⌘↵ / ⌘⇧↵).
- **Git panel** – Changes (list or tree, compare with any branch), History,
  and a syntax-highlighted diff view (unified or split, wrap or scroll).
- **Made for macOS** – translucent window with native blur, Finder-style
  title bar, resizable and collapsible side panels.
- **Design tokens & Theme Studio** – every color in the app is a named token
  derived from the theme (57 of them, with guaranteed contrast). The Theme
  Studio (⌥⌘,) shows every component in every state and lets you edit any
  token live, then save a JSON theme. 14 built-ins plus Ghostty / iTerm2 /
  Alacritty import.
- **Project folders** – an optional, committed `.wormhole/` can carry project
  settings, a theme, actions (with shortcuts), prompts, a terminal layout and
  shared `notes.md`. Every part can be switched off per user; anything that
  runs commands needs your approval with Touch ID. A terminal's Project tab makes the folder
  for you (Create project folder).
- **Menu bar & global hotkey** – agent status of every terminal in the menu bar
  (with a count when something needs you), quick actions and recent projects;
  an optional system-wide shortcut shows or hides Wormhole.
- **File icon themes** – colored file and folder icons in Changes and History;
  Zed icon themes (JSON) work as-is.
- **Updates** – signed builds update themselves (checked in the background,
  verified with a minisign signature and Apple's code signature).
- **Customizable** – a searchable settings window (⌘,) for your primary
  color, opacity, blur and more; `config.json` applies live.
- **Fonts** – JetBrains Mono ships with the app and is the default terminal
  and code font; the interface uses the system font. Separate terminal,
  interface and code fonts, picked from the bundled font, your own font files
  (drop `.ttf` / `.otf` / `.ttc` into the fonts folder, no install needed) or
  the fonts installed on your Mac; monospace fonts first for terminal and
  code.
- **Everything restores** – terminals, folders, notes, tasks, layout and
  scrollback survive restarts.

## Build & run

Requires macOS 14 (Sonoma) or later. Building needs Rust (stable) and the Xcode command line tools.

```sh
cargo run --release
scripts/dev.sh        # dev loop: rebuilds on change, keeps the last good build running
```

## Shortcuts

macOS's own keys first; every menu shows its key, and Help → Keyboard Shortcuts
(⌥⌘/) will list them all. The table is `wormhole_core::keymap`.

| Keys | Action |
|---|---|
| ⌘N / ⇧⌘W | New / close window |
| ⌘T (⇧⌘N) | New terminal |
| ⌘O | Open a folder in a new terminal |
| ⌘W | Close the selected terminal (its pane, in a split) |
| ⌘D / ⇧⌘D | Split right / down: a new terminal beside this one |
| ⌥⌘← → ↑ ↓ | Move the keyboard to the pane beside this one |
| ⌃⌘↑ / ⌃⌘↓ (⇧⌘[ / ⇧⌘]) , ⌘1…9 | Previous / next terminal, terminal by its ⌘ keycap |
| ⌘R, double-click | Rename terminal |
| ⌃⌘S / ⌥⌘I | Show or hide the sidebar / right panel |
| ⌃⌘1…5 | Changes / History / Notes / Usage / Project |
| ⇧⌘E | Move focus between terminal and notes |
| F6 / ⇧F6 | Move focus to the next / previous area, from the terminal too |
| ⌥⌘N / ⌥⌘S | New note / stash the prompt typed in the terminal into a note |
| ⌘↵ / ⌘⇧↵ | Send note paragraph (or selection) / send and submit |
| ⌘+ / ⌘- / ⌘0 | Terminal text bigger / smaller / back to the size in Settings |
| ⌘/ | Show or hide the status bar |
| ⌥⌘, / ⌘, | Theme Studio / Settings |
| Esc | Close the diff view (or the Settings / Theme Studio window) |

## Feedback

Found a bug or missing something? [Open an issue](https://github.com/wormhole-terminal/wormhole/issues/new/choose).
Security problems go through [a private report](https://github.com/wormhole-terminal/wormhole/security/advisories/new)
instead (see `SECURITY.md`).

## Development

```sh
scripts/check.sh                            # the gate, scoped to what changed: fmt, clippy, tests, build
cargo test -p wormhole-core                 # fast: the UI-independent core only
cargo test --workspace --features devctl    # everything
```

`crates/wormhole-core` holds everything that doesn't need the UI (terminal engine, git, config,
storage, agent hooks) and is tested on its own; `crates/wormhole` is the GPUI app. Lints live in the
workspace `Cargo.toml`, and `crates/wormhole/tests/design_rules.rs` checks the design rules (spacing
grid, corner radii, colors from design tokens). See `CONTRIBUTING.md`.

## License

Wormhole is free software, released under the [GNU General Public License v3.0 or later](LICENSE).
Bundled third-party assets keep their own licenses: JetBrains Mono (SIL OFL 1.1), the built-in themes
and icons (see the license files next to them), and the Rust dependencies listed in the app's
About → Licenses window.
