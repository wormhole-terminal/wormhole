# Recorded sessions for the terminal engine

Raw output of real programs, to measure and compare terminal engines on what programs really send
next to the generated streams in `crates/wormhole-vt-bench/src/corpus/` (vim, less, htop, tmux,
`git log --color`, a coding agent's TUI, ...). Nothing here is recorded automatically; add a
recording when a program shows something the generators do not.

```sh
scripts/vt-record.sh git-log -- git -C ~/some/repo log --color --stat -n 30   # from a terminal
scripts/vt-record.sh --size 120x40 htop -- htop                              # press keys, then q
scripts/vt-record.sh --show git-log                                          # play it back
cargo run --release -p wormhole-vt-bench --bin vtbench                       # measured with the rest
```

A recording is two files:

- `<name>.raw`: every byte the command wrote to its terminal, as the terminal driver delivered it
  (so line ends are CR LF). Typed input is not in it, only what the program printed.
- `<name>.meta`: `key=value` lines: `cols`, `rows`, `term`, `command`, `recorded`, `os`, `bytes`.
  `vtbench` feeds a recording to a terminal of that size.

Read a recording before committing it. It holds whatever was on the screen: paths, host names,
user names, file contents, anything a command printed. Record in a scratch repository and a clean
shell, keep recordings small (under a few MB) and re-record rather than edit a stream by hand:
the escape sequences must stay exactly as the program sent them.

Golden final screens for the recordings (what each must look like at the end) are checked by the
conformance tooling once an engine can be compared against another; until then a recording is a
speed measurement and an input for the differential tests.
