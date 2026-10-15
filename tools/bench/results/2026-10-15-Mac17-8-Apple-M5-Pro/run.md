# Terminal comparison

`ttystorm bench` (ttystorm 1.1.0) run inside each app by scripts/terminal-compare.sh.

- date: 2026-10-15T04:29:36Z
- machine: Mac17,8, Apple M5 Pro, macOS 27.0.1
- load average at the start: 4.24 10.19 11.40
- checkout: 525737d4 (with uncommitted changes)
- asked for: 120 x 40 cells, Menlo 13 pt (an app that can't be told from the command line has its own)

| terminal | result | version | rows x cols seen | in front when it started | load average (1 min) when it started |
|---|---|---|---|---|---|
| wormhole | ok | 525737d4 (this checkout) | 40x120 | yes | 3.90 |
| ghostty | ok | 1.3.1 | 40x120 | yes | 3.91 |
| alacritty | ok | 0.17.0 | 40x120 | yes | 3.87 |
| kitty | ok | 0.49.2 | 40x120 | yes | 3.81 |
| wezterm | ok | 20240203-110809-5046fc22 | 40x120 | yes | 3.77 |
| iterm2 | ok | 3.7.4 | 40x120 | yes | 3.61 |
| terminal | ok | 2.15 | 40x120 | yes | 3.35 |

Per app: ghostty, kitty, wezterm and alacritty ran as a separate process each, with their user
configuration left out, size and font from their command line; iterm2 and terminal were driven by
AppleScript (columns and rows, and for terminal the font) and use their profile's font otherwise;
wormhole is a release build of this checkout in an isolated instance, font and size from its config.

iTerm2 and Terminal ran later the same day (load 3.6 and 3.4 at their start), from a release build of 5e2f529c for nothing else; iTerm2 asked before clearing the scrollback and was left to its default (not allowed).
