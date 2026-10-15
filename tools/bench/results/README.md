# Results

`ttystorm bench` run inside each terminal app on one machine, by `scripts/terminal-compare.sh`.
One folder per run, `<date>-<machine>/`:

| file | what |
|------|------|
| `<terminal>.json` | `ttystorm bench --save` of that terminal (`wormhole`, `ghostty`, `alacritty`, `kitty`, `wezterm`, `iterm2`, `terminal`) |
| `compare.md` | `ttystorm compare --markdown` over them, Wormhole first (the last row is every terminal's speed against it) |
| `run.md` | the setup: date, machine, load average, the checkout, the size and font asked for, each app's version, the size it really had, whether it was in front, and which terminals were skipped or failed and why |

These are one run on a laptop that had other things to do: wall-clock numbers, a guide and not a
verdict. The load average when each terminal started is in `run.md`; run again with nothing else busy
before believing a small difference. `scripts/perf.sh` and the baselines in `../baselines/` are the
regression check for Wormhole alone; this folder is for comparing with the others.
