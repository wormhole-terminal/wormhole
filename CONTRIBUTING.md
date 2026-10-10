# Contributing to Wormhole

Thanks for helping. Bug reports, fixes and small improvements are welcome as issues and pull
requests; for a larger feature, open an issue first so we can agree on the shape before you build it.

## Building

macOS 14 or later, Rust (the toolchain in `rust-toolchain.toml` installs itself) and the Xcode
command line tools.

```sh
cargo run                 # the app (binary: Wormhole)
scripts/dev.sh            # rebuild on every change; keeps the last good build running
```

The workspace also checks, lints and tests on Linux; the app itself only runs on macOS.

## The gate

A change is done when this passes:

```sh
scripts/check.sh          # scoped to what changed since origin/main; --all for everything
```

It runs `cargo fmt --all`, `cargo clippy --workspace --all-targets --features devctl -- -D warnings`,
the tests (`cargo test --workspace --features devctl`) and `cargo check --workspace` (the app as it
ships, without the `devctl` test feature). After a dependency change also run `cargo deny check` and
`scripts/gen-licenses.py`. CI only runs when started by hand, so please run the gate before you push.

Changes to the terminal's rendering, the PTY filters or the snapshot are guarded by counting tests;
also run `scripts/perf.sh` (a release build timed against your machine's baseline).

## Layout

- `crates/wormhole-core`: no GPUI. Terminal engine, git, config, storage, agent hooks, the keymap,
  layout rules. Put logic here whenever it doesn't need the UI, and test it here.
- `crates/wormhole`: the GPUI app. `workspace/` is one window, `style/` the design language,
  `terminal_view/` the terminal.
- `tools/visual/`: scenarios for screenshot comparisons (`scripts/visual-check.sh`).
- `tools/bench/`: terminal benchmarks (its own workspace).
- `packaging/`, `scripts/`: bundling, signing and releases.

## Conventions

- Rust 2024, `rustfmt.toml` (width 110). No `unwrap()` outside tests; `unsafe` only in the few FFI
  functions that carry `#[allow(unsafe_code)]` and a SAFETY note. `anyhow` for errors, `tracing` for
  logs (never `println!`).
- Files: every read or write of a user or project file goes through `wormhole_core::fsutil`
  (atomic writes, size-capped reads, no following links out of a repository). Content from a cloned
  repository is hostile input.
- Git is read-only and always goes through `wormhole_core::git` (`GitCmd`), which hardens every call.
  Never spawn `git` yourself.
- Anything that runs commands or changes the environment because of a project folder checks
  `ProjectHub::enabled` (which includes the user's Touch ID approval).
- Colors come from design tokens, never hard-coded. Spacing, type, corners and shared components
  come from `crates/wormhole/src/style/`; `tests/design_rules.rs` enforces the rules.
- The database is migrated: a schema change appends a step to `MIGRATIONS` (never edit a shipped
  one), and `store/tests/migrations.rs` checks it.
- Comments explain *why*. Split files that grow past about 600 lines.
- Commit messages follow Conventional Commits: `feat(sidebar): …`, `fix(git): …`.

## Testing the UI

Build with `--features devctl` and start an isolated instance with `scripts/devctl-instance.sh`: it
keeps all its data in a scratch folder, runs `/bin/cat` instead of a shell and never touches your
real settings, repositories or agents. The commands are documented at the top of
`crates/wormhole/src/devctl/mod.rs`. Never point tests at your real data folder or repositories.

## License

By contributing you agree that your contribution is licensed under the GNU General Public License
v3.0 or later, the license of the project.
