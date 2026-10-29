---
name: verify
description: Run the checks that define "done" for Wormhole (rustfmt, clippy with -D warnings, tests) in the right order and report precisely. Use before finishing any code change, before committing or opening a PR, and whenever the user asks whether the build is green.
allowed-tools: Bash(scripts/check.sh:*), Bash(cargo fmt:*), Bash(cargo clippy:*), Bash(cargo test:*), Bash(cargo check:*), Bash(cargo deny check:*)
---

# Verify a change

A change is done when the gate passes: `cargo fmt --all`, clippy with
`-D warnings`, the workspace tests with `--features devctl`, a plain `cargo check --workspace` (the app
as it ships, without devctl: a devctl-only item re-exported without its gate only fails there), and
`cargo deny check` if you touched dependencies or licenses. CI runs on pull requests and pushes to main
only once the repository is public, so until then this is the only gate. After `cargo fmt --all`, check that `git diff` only holds
your changes.

## Run only what the change needs

`scripts/check.sh` is the gate scoped to the changed files (since the merge base with `origin/main`,
plus staged, unstaged and untracked files). The widest scope any file needs wins:

| Changed | Runs |
| --- | --- |
| `crates/wormhole-core/**`, `crates/wormhole-term/**`, `Cargo.toml`/`Cargo.lock`, toolchain/lint config, `scripts/lib.sh`/`package.sh`, anything unlisted | the whole gate (the app builds on core); `cargo deny check` for dependency files |
| `crates/wormhole/**`, `packaging/app.env`, `updater.pub`, `icon/` | fmt, clippy, tests and the no-devctl check for `-p wormhole` only |
| docs, skills, agents, rules, `scripts/`, `tools/visual/`, top-level `.md` | only `design_rules` (it scans their wording); nothing compiles |
| `.github/`, `.claude/hooks`, settings, the rest of `packaging/` | nothing |

- `--plan` prints each file's scope and the commands without running them.
- `--base HEAD` checks only what isn't committed (earlier commits already passed).
- `--all` runs the whole gate: after a rebase onto new `main` commits, before a release, or when a change
  reaches further than its path says (a test that reads a file the table doesn't know about: add it to
  `scope_of` in the script).
- It stops at the first failing step and keeps output quiet; rerun the failing command alone to iterate.
- When a test reads a new file outside its crate (`include_str!`, `CARGO_MANIFEST_DIR/../..`), add the
  path to `scope_of`, or the script can skip the test that would catch a change to it.

Run it once when the work is done, not after every edit; while working, use the fast loop.

## Fast loop while working

- Core only (no GPUI, seconds): `cargo test -p wormhole-core <name filter>`; the terminal layer
  (PTY, engine host, screen) is its own crate: `cargo test -p wormhole-term <name filter>`
- Whole workspace type-check: `cargo check --workspace --features devctl`
- One app test: `cargo test -p wormhole --features devctl <name filter>` (with devctl it reuses the
  gate's build; without, the app compiles a second time)
- One test file: `cargo test -p wormhole-rules --test design_rules`

## macOS vs Linux

Cloud sessions and containers run Linux: the whole workspace builds, lints and tests there (macOS-only
code has stubs), but the GUI cannot be launched. Link failures mentioning `-lxkbcommon` mean the system
packages from `.claude/hooks/session-start.sh` are missing. Say so when a change affects rendering and
you could not look at it; on macOS use the `devctl` skill.

## Reporting

Report each command with its result. If something fails, show the failing test or lint (file:line and
message), say whether it is yours or pre-existing (check with `git stash`-free methods: compare against
`git worktree add --detach <dir> HEAD`), and fix yours before reporting. Never skip, `#[ignore]` or
weaken a test to get green, and never add `#[allow(...)]` without a comment saying why.

## Several worktrees or agents at once

Create the worktree yourself (`git worktree add .claude/worktrees/<name> -b <branch> <base>`), then run
`scripts/worktree.sh <worktree path>` to prepare it, `cd` into it and `export
CARGO_TARGET_DIR=<main checkout>/target` so third-party dependencies compile once. Cargo serializes
builds on that directory; keep runs targeted. A per-worktree target dir (`target/<name>`) needs no
preparation but compiles everything again.

The shared target dir also shares its binaries: `$CARGO_TARGET_DIR/debug/Wormhole` is whatever any
worktree built last. Never run it for a devctl session straight from there; copy it first (see the
`devctl` skill).

The script writes an untracked `.cargo/config.toml` that every shared-target setup needs:

- **A distinct build identity.** Cargo hashes workspace crates by their workspace-relative path, so
  `wormhole-core`, `wormhole-term` and `wormhole` get the *same* unit hash in every worktree. Worktrees sharing a
  target dir then overwrite each other's artifacts, and a build can silently link another worktree's
  code (false green or false red). A unique `codegen-units` value per worktree gives each its own hash;
  dependencies keep theirs and stay shared.
- **`strip = "debuginfo"`, repeated in each package table** (a package with its own table does not
  inherit `[profile.dev]` strip). About 93% of a linked test binary is DWARF (1.2 GB each for the app);
  stripping shrinks executables ~14x and rebuilds no dependency. Without it the disk allowance fills.

Run cargo with the worktree as the current directory (config discovery uses the cwd, not
`--manifest-path`) and never commit the generated file. Check isolation with
`cargo check -p wormhole-core --message-format=json | jq -r 'select(.reason=="compiler-artifact" and .target.name=="wormhole_core") | .filenames[0]'`:
two worktrees must print different hashes.

## What Linux cannot tell you

macOS-only code is compiled out on Linux, so a green Linux run can hide macOS failures: unused imports
in `cfg(target_os = "linux")`-only tests, AppKit/FFI changes, `cfg(not(macos))` stubs drifting from the
real thing. With CI off, a macOS run of this skill is the gate for those: say so when a change
touches macOS-only code and you could only check it on Linux. Tests that need
GPUI's test support compile with plain `cargo test` because the app crate has it as a dev-dependency.
