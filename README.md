# Wormhole

Wormhole is a macOS terminal for working with coding agents such as Claude Code,
Codex and Gemini CLI. A sidebar keeps your terminals and agent status visible;
each terminal has its own notes, task list and Git panel.

[Download Wormhole](https://getwormhole.app) for macOS 14 or later, on Apple
Silicon or Intel.

## Features

- **Terminal sidebar.** Name, reorder and group terminals. See which agent is
  running, whether it needs your attention, and the current Git branch.
- **Splits and windows.** Split terminals horizontally or vertically and work
  across multiple windows. Window layouts, folders, notes, tasks and saved
  scrollback are restored when you reopen the app.
- **Agent integrations.** Install status hooks in Settings → Coding agents to
  let supported agents report when they are working, finished or waiting for
  input. Without hooks, status is estimated from terminal activity.
- **Notes and tasks.** Keep notes beside each terminal, stash a prompt while you
  answer an agent, and send a note or selection back to the terminal.
- **Git changes and history.** Browse changed files, compare branches and read
  syntax-highlighted diffs in unified or split view.
- **Command blocks.** With shell integration, jump between prompts, inspect exit
  codes and timings, and copy a command or its output.
- **Inline images.** Display images through the kitty graphics protocol, iTerm2
  inline images or Sixel.
- **Shells and appearance.** Configure shell profiles, fonts and themes. Import
  Ghostty, iTerm2 or Alacritty themes, or edit colors in Theme Studio.
- **Menu bar and hotkey.** Check agent status from the menu bar and show or hide
  Wormhole with a global shortcut.

The Usage panel shows Claude Code and Codex plan limits when their integrations
provide them. Gemini CLI reports tokens for its latest model response. Enable
or update integrations in Settings → Coding agents, then restart the agent.
Usage information depends on the agent version and account type; unavailable
limits are omitted.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| ⌘T | New terminal |
| ⌘O | Open a folder in a new terminal |
| ⌘D / ⇧⌘D | Split right / down |
| ⌘1…9 | Select a terminal |
| ⌘N | New window |
| ⌘W | Close the selected terminal |
| ⌥⌘S | Stash the current prompt in a note |
| ⌘↵ / ⇧⌘↵ | Send note text / send and submit |
| ⇧⌘P | Open the command palette |
| ⌘, | Open Settings |

Open Help → Keyboard Shortcuts (⌥⌘/) for the full list.

## Build and develop

Building requires macOS 14 or later, Rust and the Xcode command line tools.
The Rust version is pinned in `rust-toolchain.toml`.

From this repository:

```sh
cargo run --release
```

For development:

```sh
scripts/dev.sh            # rebuild on changes
scripts/check.sh          # run checks for the changed files
cargo test -p wormhole-core
```

Wormhole is written in Rust with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)
and GPUI Kit. The app lives in `crates/wormhole`; configuration, storage, Git and
agent integrations live in `crates/wormhole-core`. The terminal engine,
[wormhole-vt](https://github.com/wormhole-terminal/wormhole-vt), is maintained in
a separate repository.

See [CONTRIBUTING.md](CONTRIBUTING.md) for build checks, tests and contribution
conventions.

## Privacy

Wormhole has no analytics or crash reporting, and your settings, notes and terminal output stay on
your Mac. Update checks send the app's version, the update channel, Apple silicon or Intel and the
macOS major version, and the first check of each day and month says so: that is how we count how many
copies run, with no ID and no IP address kept. Counting starts a day after Wormhole first runs, and
Settings → Updates → Count this Mac anonymously shows what is sent and turns the count off.

## Feedback

[Report a bug](https://github.com/wormhole-terminal/wormhole/issues/new/choose)
or use [Discussions](https://github.com/wormhole-terminal/wormhole/discussions)
for questions and ideas. Report security vulnerabilities privately through
[GitHub security advisories](https://github.com/wormhole-terminal/wormhole/security/advisories/new);
see [SECURITY.md](SECURITY.md).

Release notes are in [CHANGELOG.md](CHANGELOG.md).

## License

Wormhole is free software under the [GNU GPL v3.0 or later](LICENSE).
[wormhole-vt](https://github.com/wormhole-terminal/wormhole-vt) is MIT-licensed.
Bundled fonts, themes, icons and dependencies retain their own licenses; see
About → Licenses in the app.
