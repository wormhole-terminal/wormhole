# Contributing to Wormhole

Thanks for helping. Bug reports, fixes and small improvements are welcome as issues and pull
requests; for a larger feature, open an issue first so we can agree on the shape before you build it.
Questions go to [Discussions](https://github.com/wormhole-terminal/wormhole/discussions). Everyone
taking part follows the [code of conduct](CODE_OF_CONDUCT.md).

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
ships, without the `devctl` test feature). First, whatever changed, it checks the history: the
commits since `origin/main` follow the [commit rules](#commits) and a user-visible change has its
[changelog](#changelog) entry. After a dependency change also run `cargo deny check` and
`scripts/gen-licenses.py`. CI runs the same checks on every pull request, but it is faster to find out
locally.

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
- Commit messages follow Conventional Commits: `feat(sidebar): …`, `fix(git): …` (see [Commits](#commits)).

## Testing the UI

Build with `--features devctl` and start an isolated instance with `scripts/devctl-instance.sh`: it
keeps all its data in a scratch folder, runs `/bin/cat` instead of a shell and never touches your
real settings, repositories or agents. The commands are documented at the top of
`crates/wormhole/src/devctl/mod.rs`. Never point tests at your real data folder or repositories.

## Versioning

Versions are `MAJOR.MINOR.PATCH` as in [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Before 1.0:

- A **minor** version (0.3.0 → 0.4.0) brings new features, and anything that changes what users
  already have: their data, settings, permissions or identifiers. A database migration, a config key
  that moves, a new permission the app asks for and a renamed folder or variable all make a minor.
- A **patch** version (0.3.0 → 0.3.1) is fixes only.
- A security fix always ships as a patch on the latest version as soon as it is ready; it doesn't wait
  for the next minor.
- A prerelease (`0.4.0-beta.1`) is published as a GitHub prerelease and is never offered as an update
  on the Stable channel.
- A build of `main` is numbered after the last release: `0.4.1-dev.12` after `0.4.0` (12 commits since
  its tag). Only the Unstable channel gets it (see Releases), and the next release sorts above it.
- `version` in `Cargo.toml` changes only in a release commit (`chore(release): 0.3.1`, made by
  `scripts/release.sh`). Between releases it is the last release's number.

## Commits

Commit messages follow [Conventional Commits 1.0](https://www.conventionalcommits.org/en/v1.0.0/):

```text
type(scope): subject

Optional body: why the change is needed, in plain sentences.

Footer: value
```

- **Type**: `feat` (new behaviour), `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`,
  `style` or `revert`.
- **Scope** is optional: lowercase `a-z`, `0-9` and `-`, several separated by commas without spaces
  (`fix(git,git-panel): …`). It is the part of the app, not a list to pick from; the common ones are
  `terminal`, `sidebar`, `workspace`, `settings`, `studio`, `project`, `git`, `git-panel`, `update`,
  `store`, `agents`, `onboarding`, `a11y`, `devctl`, `release`, `scripts` and `github`.
- **Subject**: starts lowercase (a name such as Wormhole or an acronym such as GPUI keeps its capitals),
  no final period, and the whole header is at most 100 characters.
- A `!` before the colon (`feat(config)!: …`), or a `BREAKING CHANGE: …` footer, marks a breaking change.
- `Security: GHSA-xxxx-xxxx-xxxx` marks a security fix. Before the advisory exists, `Security: <short text>`.
- `Changelog: none` marks a `feat`, `fix` or `perf` that needs no changelog line, for example a fix to
  something that never shipped. It never goes with `Security`.

```text
feat(sidebar): pin a terminal to the top of its folder
feat(config)!: keep shell profiles in one list
perf(terminal): shape a row once per frame

fix(update): refuse an archive signed for another version

Security: GHSA-xxxx-xxxx-xxxx
```

History is linear: no merge commits, rebase onto `origin/main` instead. A pull request is squash
merged and its title becomes the commit's header, so the title follows the same rules (at most 92
characters, GitHub adds ` (#1234)`). Squash `fixup!` commits before pushing.

`scripts/commits.py` checks all of this: `check-message`, `check-range <rev-range>` and
`changelog-needed <rev-range>`. `scripts/check.sh` and the History job in CI run it. To catch a wrong
header while you type it, `scripts/install-hooks.sh` installs a `commit-msg` hook (optional;
`--remove` takes it out again).

## Changelog

[CHANGELOG.md](CHANGELOG.md) follows [Keep a Changelog 1.1](https://keepachangelog.com/en/1.1.0/) and
is the single source of release notes: the tag message, the GitHub Release, the update feed and the
update window all read it (`scripts/changelog.py notes <version>`).

- The entry goes in the same change as the code, under `## [Unreleased]`, in the subsection it
  belongs to: **Added** (new features), **Changed** (changes to existing behaviour), **Deprecated**
  (will be removed), **Removed**, **Fixed** (bug fixes) and **Security**. Subsections keep this order
  and are left out when empty.
- It is written for the people who use Wormhole, as a sentence about what they notice, not about the
  code: "The sidebar no longer flickers a second copy of the folder band when you scroll a short
  list." A refactor, a test, a build or CI change has no entry.
- A change that needs migrating data, settings or permissions says so in its entry.
- `feat`, `fix`, `perf`, breaking and `Security` commits need an entry, or `Changelog: none` in the
  commit. A pull request without one is labelled `no changelog`. The History check in CI and
  `scripts/check.sh` enforce this.
- A security fix is listed under **Security** in this exact form, which other tools read:

  ```text
  - **High** · [GHSA-xxxx-xxxx-xxxx](https://github.com/wormhole-terminal/wormhole/security/advisories/GHSA-xxxx-xxxx-xxxx) · affects 0.2.0–0.3.0: what was wrong and what an attacker could do, in plain words.
  ```

  The severity is Critical, High, Medium or Low (GitHub's advisory levels). Without an advisory the
  link is `no advisory`. A version with a Security entry is offered by the updater as a security
  update, at the highest severity listed.

`scripts/changelog.py check` validates the file.

## Releases

On a clean `main` that has everything from `origin/main`:

```sh
scripts/release.sh 0.4.0 --dry-run    # shows the changelog diff, the tag message and what would stop it
scripts/release.sh 0.4.0              # commit and tag; prints the push command
git push origin main v0.4.0           # or pass --push
```

`release.sh` turns `[Unreleased]` in CHANGELOG.md into the version (and refuses an empty one), sets
`version` under `[workspace.package]` in `Cargo.toml` and `Cargo.lock`, commits `chore(release): 0.4.0`
and creates the annotated tag `v0.4.0`, whose message is `Wormhole 0.4.0` and the version's notes.
Run `scripts/gen-licenses.py` first if a dependency changed.

The tag starts the Release workflow. It builds one app for Apple Silicon and Intel, signs with the
Developer ID certificate, notarizes, signs the update archive with minisign and publishes everything as
the GitHub Release `v0.4.0`, with the notes from CHANGELOG.md (it fails when the version has no
section). A version with a Security entry gets `"security": {"severity": …, "advisories": […]}` in
`latest.json`, and the Release page opens with a security notice. The app checks
`https://downloads.getwormhole.app/stable/latest.json`, which redirects to the latest release's
`latest.json`; a version with a suffix (`0.4.0-beta.1`) is published as a prerelease and is never
offered on the Stable channel.

Every push to `main` that passes CI is built the same way for the Unstable channel (Settings → Updates →
Channel): numbered `X.Y.(Z+1)-dev.N` (`dev_version` in `scripts/lib.sh`), signed, notarized and put on
the rolling prerelease `unstable`, which holds only the newest build and the one before. Its notes are
CHANGELOG.md's `[Unreleased]`. The app on Unstable reads `unstable/latest.json` and `stable/latest.json`
and takes the newer, and it shows Settings → Feature flags, which a release otherwise hides.

## License

By contributing you agree that your contribution is licensed under the GNU General Public License
v3.0 or later, the license of the project.
