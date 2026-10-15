# Suite comparison: how each run went

- date: 2026-10-15T11:48:57Z
- machine: Mac17,8, Apple M5 Pro, macOS 27.0.1
- checkout: 5e2f529c (with uncommitted changes)
- asked for: 80 x 25 cells, Menlo 13 pt (an app that can't be told from the command line has its own)
- load average at the start: 4.01 3.53 3.48

| suite | terminal | result | version | rows x cols seen | in front | load (1 min) | seconds |
|---|---|---|---|---|---|---|---|
| esctest | Wormhole | ok | 5e2f529c (this checkout) | 25x80 | yes | 3.84 | 87 |
| esctest | Ghostty | ok | 1.3.1 | 25x80 | yes | 3.72 | 134 |
| esctest | Alacritty | ok | 0.17.0 | 25x80 | no | 3.78 | 96 |
| esctest | kitty | ok | 0.49.2 | 25x80 | yes | 3.85 | 106 |
| esctest | WezTerm | ok | 20240203-110809-5046fc22 | 25x80 | no | 3.94 | 278 |
| esctest | iTerm2 | ok | 3.7.4 | 25x80 | yes | 2.82 | 132 |
| esctest | Terminal | ok | 2.15 | 25x80 | yes | 3.76 | 267 |
| ucs-detect | Wormhole | ok | 5e2f529c (this checkout) | 25x80 | no | 3.36 | 21 |
| ucs-detect | Ghostty | ok | 1.3.1 | 25x80 | yes | 3.54 | 7 |
| ucs-detect | Alacritty | ok | 0.17.0 | 25x80 | no | 3.65 | 8 |
| ucs-detect | kitty | ok | 0.49.2 | 25x80 | yes | 3.82 | 165 |
| ucs-detect | WezTerm | failed: no result within 1800s | 20240203-110809-5046fc22 | ? | yes | 3.73 | 1930 |
| ucs-detect | iTerm2 | ok | 3.7.4 | 25x80 | no | 3.33 | 485 |
| ucs-detect | Terminal | ok | 2.15 | 25x80 | no | 3.87 | 57 |

Apps drive one at a time, each with its user configuration left out (see scripts/terminal-compare.sh
for what each is given). `skipped: already running` is iTerm2 or Terminal, which have one process
and are never taken over.
