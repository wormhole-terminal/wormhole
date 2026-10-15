#!/bin/bash
# The local gate (fmt, clippy, the tests, the build without devctl), run only as far as the changed files reach.
#
#   scripts/check.sh                  # what changed since the merge base with origin/main, plus the
#                                     # working tree (staged, unstaged, untracked)
#   scripts/check.sh --base HEAD      # only what isn't committed yet
#   scripts/check.sh --all            # the whole gate, whatever changed (after a rebase; when unsure)
#   scripts/check.sh --plan           # print what would run and why, run nothing
#
# Scopes, widest first; the widest one any changed file needs wins:
#   all   crates/wormhole-core/**, Cargo.toml/Cargo.lock, toolchain, lint and build config, a script
#         core includes (lib.sh, package.sh), or any path not listed here: the whole gate (the app
#         builds on core, so a core change can break it). Plus `cargo deny check` for dependencies.
#   app   crates/wormhole/** or a packaging file the app includes: fmt, clippy, tests and the
#         no-devctl check for the app alone. Core's tests can't see the app, so they're skipped.
#   text  skills, agents, rules, scripts, tools/visual, the top-level .md files (CHANGELOG.md too): only
#         design_rules (it scans their wording), never a compile of either crate.
#   none  .github, .claude/hooks and settings, .gitignore, the rest of packaging, tools/bench and
#         tools/vt-compare (their own workspaces): nothing to run.
# Whatever the scope, first the history (cheap, before any cargo): the commits since the base follow
# the rules (scripts/commits.py), a feat/fix/perf, breaking or Security commit comes with a
# CHANGELOG.md entry, and CHANGELOG.md itself is well formed when it changed (scripts/changelog.py).
# The engine's outside suites (esctest2, ucs-detect; scripts/suites-gate.sh) run last, against a committed
# expectation, when crates/wormhole-vt, the bench crate or core's terminal layer changed, and with --all.
# Output is quiet (test names only when they fail), so an agent reads results, not progress.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

base="" plan=0 force_all=0
while [ $# -gt 0 ]; do
  case "$1" in
    --base) base="${2:?--base needs a revision}"; shift ;;
    --all) force_all=1 ;;
    --plan) plan=1 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
  shift
done

# The scope one path needs, and whether it touches dependencies.
scope_of() {
  case "$1" in
    Cargo.toml|Cargo.lock|deny.toml|crates/*/Cargo.toml) echo "all deps" ;;
    rust-toolchain*|rustfmt.toml|clippy.toml|.cargo/*|.config/nextest.toml) echo all ;;
    crates/wormhole-core/*) echo all ;;
    # include_str!'d by core's updater tests; lib.sh also runs in the app's isolated_instance test.
    scripts/lib.sh|scripts/package.sh) echo all ;;
    crates/wormhole/*) echo app ;;
    # include_bytes!/include_str!'d by the app.
    packaging/app.env|packaging/updater.pub|packaging/icon/*) echo app ;;
    # Scanned for wording by crates/wormhole/tests/design_rules.rs (banned words, the devctl docs).
    scripts/*|tools/visual/*|.claude/skills/*|.claude/agents/*|.claude/rules/*) echo text ;;
    README.md|CONTRIBUTING.md|SECURITY.md|CODE_OF_CONDUCT.md|ACCESSIBILITY.md|CHANGELOG.md) echo text ;;
    .github/*|.claude/hooks/*|.claude/settings*.json|.claude/worktrees/*|.gitignore|LICENSE|packaging/*) echo none ;;
    # Their own workspaces, built and checked on their own (tools/bench/README.md, tools/vt-compare/README.md).
    tools/bench/*|tools/vt-compare/*) echo none ;;
    *) echo all ;;   # unknown: safer to run everything than to miss a test that reads it
  esac
}

rank() { case "$1" in all) echo 3 ;; app) echo 2 ;; text) echo 1 ;; *) echo 0 ;; esac; }

if [ -z "$base" ]; then
  upstream=origin/main
  git rev-parse --verify -q "$upstream" >/dev/null || upstream=main
  base="$(git merge-base HEAD "$upstream" 2>/dev/null)" || die "no merge base with $upstream; pass --base"
fi
git rev-parse --verify -q "$base^{commit}" >/dev/null || die "not a revision: $base"

changed="$( { git diff --name-only "$base"; git ls-files --others --exclude-standard; } | sort -u)"

scope=none deps=0 why=""
while IFS= read -r path; do
  [ -n "$path" ] || continue
  read -r s d <<<"$(scope_of "$path")"
  [ "$d" = deps ] && deps=1
  if [ "$(rank "$s")" -gt "$(rank "$scope")" ]; then scope=$s why=$path; fi
done <<<"$changed"
[ "$force_all" = 1 ] && { scope=all; why="--all"; }

# The outside suites test the engine, its bench host and what feeds it; their own files too.
suites=0
grep -Eq '^(crates/wormhole-vt/|crates/wormhole-vt-bench/|crates/wormhole-core/src/terminal/|scripts/suites-gate\.sh$|tools/vt-compare/(gate\.py|expected/|suites\.pin|suites/|ucs-headless\.py|fetch-suites\.sh))' <<<"$changed" && suites=1
[ "$force_all" = 1 ] && suites=1

rust_changed=0
grep -q '\.rs$' <<<"$changed" && rust_changed=1

steps=()
case "$scope" in
  all)
    steps+=("cargo fmt --all")
    steps+=("cargo clippy --quiet --workspace --all-targets --features devctl -- -D warnings")
    steps+=("cargo test --quiet --workspace --features devctl")
    steps+=("cargo check --quiet --workspace")
    ;;
  app)
    steps+=("cargo fmt --all")
    steps+=("cargo clippy --quiet -p wormhole --all-targets --features devctl -- -D warnings")
    steps+=("cargo test --quiet -p wormhole --features devctl")
    steps+=("cargo check --quiet -p wormhole")
    ;;
  text)
    [ "$rust_changed" = 1 ] && steps+=("cargo fmt --all")
    # devctl like the other scopes, so the app the test target links is the one already built.
    steps+=("cargo test --quiet -p wormhole --features devctl --test design_rules")
    ;;
esac
[ "$deps" = 1 ] && steps+=("cargo deny check")
[ "$suites" = 1 ] && steps+=("scripts/suites-gate.sh")

# The history steps go first. They read the git history, so they run for every scope, `none` included.
history=("python3 scripts/commits.py check-range $base..HEAD"
         "python3 scripts/commits.py changelog-needed --include-worktree $base..HEAD")
grep -qx 'CHANGELOG.md' <<<"$changed" && history+=("python3 scripts/changelog.py check")
grep -qx 'scripts/changelog.py' <<<"$changed" && history+=("python3 scripts/changelog.py selftest")
grep -qx 'scripts/commits.py' <<<"$changed" && history+=("python3 scripts/commits.py selftest")
steps=("${history[@]}" "${steps[@]+"${steps[@]}"}")

n=$(grep -c . <<<"$changed")
log "scope: $scope ($n changed file(s) since ${base:0:10}${why:+; widest: $why})"
if [ "$plan" = 1 ]; then
  while IFS= read -r path; do
    [ -n "$path" ] && printf '    %-5s %s\n' "$(scope_of "$path" | cut -d' ' -f1)" "$path"
  done <<<"$changed"
  for step in "${steps[@]+"${steps[@]}"}"; do echo "  would run: $step"; done
  exit 0
fi
[ ${#steps[@]} -eq 0 ] && { log "nothing to check"; exit 0; }

failed=0
for step in "${steps[@]}"; do
  log "$step"
  start=$SECONDS
  if [ "$step" = "cargo deny check" ] && ! command -v cargo-deny >/dev/null 2>&1; then
    warn "cargo-deny is not installed (cargo install cargo-deny): dependencies and licenses are unchecked"
    continue
  fi
  # Word splitting is intended: every step is a plain command line without quoting.
  # shellcheck disable=SC2086
  if ! $step; then
    printf '\033[1;31mFAILED\033[0m %s\n' "$step" >&2
    failed=1
    # A broken build fails every later step the same way; stop at the first so the error is easy to find.
    break
  fi
  echo "    ok ($((SECONDS - start)) s)" >&2
done
[ "$failed" = 0 ] && log "all passed (scope: $scope)"
exit "$failed"
