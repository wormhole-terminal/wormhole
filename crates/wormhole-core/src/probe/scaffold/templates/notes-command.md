A palette command and a rule that feeds it. `probe.toml` has a rule that finds a commit id and, on
Command-click, runs the command `show-commit` with that id as `$1`; and a second command that runs the script
`scripts/hello.sh` of this folder in a new terminal. Change the programs, the pattern and the cases in
`tests/example.toml`. `process` lists every program the Probe may run; list only what a command runs.
