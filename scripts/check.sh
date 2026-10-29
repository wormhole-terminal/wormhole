#!/bin/bash
# The local gate (fmt, clippy, the tests, the build without devctl), run only as far as the changed files reach.
#
#   scripts/check.sh                  # what changed since the merge base with origin/main, plus the
#                                     # working tree (staged, unstaged, untracked)
#   scripts/check.sh --base HEAD      # only what isn't committed yet
#   scripts/check.sh --all            # the whole gate, whatever changed (after a rebase; when unsure)
#   scripts/check.sh --plan           # print what would run and why, run nothing
#   scripts/check.sh --scope          # print only `scope=<name>` and `deps=<0|1>` (CI reads these), run nothing
#   scripts/check.sh --selftest       # check the scope table below against the crates and what they include
#
# Scopes, widest first; the widest one any changed file needs wins:
#   all   every crate but the app (the others depend on each other: wire is term's, host's and core's),
#         Cargo.toml/Cargo.lock, toolchain, lint and build config, a file core includes (lib.sh,
#         package.sh, packaging/icon), or any path not listed here: the whole gate (the app builds on
#         core, so a core change can break it). Plus `cargo deny check` for dependencies.
#   app   crates/wormhole/** or a packaging file only the app includes: fmt, clippy, tests and the
#         no-devctl check for the app, and the source checks (wormhole-rules), which read the app's
#         sources. Core's tests can't see the app, so they're skipped.
#   text  the source checks themselves (crates/wormhole-rules), skills, agents, rules, scripts,
#         tools/visual, the top-level .md files (CHANGELOG.md too): the source checks (they scan the
#         wording), which link std alone, so nothing of the app or core compiles.
#   none  .github, .claude/hooks and settings, .gitignore, the rest of packaging, tools/bench (its
#         own workspace) and tools/feed-counter (bun test there): nothing to run.
# Whatever the scope, first the history (cheap, before any cargo): the commits since the base follow
# the rules (scripts/commits.py), a feat/fix/perf, breaking or Security commit comes with a
# CHANGELOG.md entry, and CHANGELOG.md itself is well formed when it changed (scripts/changelog.py).
# Then the structure (scripts/structure.py, about a second, nothing built) for every scope but `none`: a
# count the restructuring brings to zero (OS code and objc2 outside the OS edge, `use super::*`,
# cycles between modules, files over 1,000 lines, ...) may not grow, and scripts/structure.json
# (the allowlist) may only shrink, so a fixed violation must leave it in the same change.
# The terminal engine (wormhole-vt) has a repository of its own, with its own checks: the outside
# suites, the public API listing and mutation testing run there.
# Output is quiet (test names only when they fail), so an agent reads results, not progress.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
. scripts/lib.sh

base="" plan=0 force_all=0 scope_only=0 selftest=0
while [ $# -gt 0 ]; do
  case "$1" in
    --base) base="${2:?--base needs a revision}"; shift ;;
    --all) force_all=1 ;;
    --plan) plan=1 ;;
    --scope) scope_only=1 ;;
    --selftest) selftest=1 ;;
    -h|--help) sed -n '2,22p' "$0"; exit 0 ;;
    *) die "unknown argument: $1 (see --help)" ;;
  esac
  shift
done

# The scope one path needs, and whether it touches dependencies.
scope_of() {
  case "$1" in
    Cargo.toml|Cargo.lock|deny.toml|crates/*/Cargo.toml) echo "all deps" ;;
    rust-toolchain*|rustfmt.toml|clippy.toml|.cargo/*|.config/nextest.toml) echo all ;;
    crates/wormhole-core/*|crates/wormhole-term/*|crates/wormhole-host/*|crates/wormhole-wire/*|crates/wormhole-text/*) echo all ;;
    # include_str!'d by core's updater tests; lib.sh also runs in the app's isolated_instance test.
    # packaging/icon: core's intro tests include the iconset script and the chevron.
    scripts/lib.sh|scripts/package.sh|packaging/icon/*) echo all ;;
    crates/wormhole/*) echo app ;;
    # The checks read the app's sources and the repository's text; they link nothing of the workspace.
    crates/wormhole-rules/*) echo text ;;
    # include_bytes!/include_str!'d by the app alone.
    packaging/app.env|packaging/updater.pub) echo app ;;
    # Scanned for wording by crates/wormhole-rules/tests/design_rules.rs (banned words, the devctl docs).
    scripts/*|tools/visual/*|.claude/skills/*|.claude/agents/*|.claude/rules/*) echo text ;;
    README.md|CONTRIBUTING.md|SECURITY.md|CODE_OF_CONDUCT.md|ACCESSIBILITY.md|CHANGELOG.md) echo text ;;
    .github/*|.claude/hooks/*|.claude/settings*.json|.claude/worktrees/*|.gitignore|LICENSE|packaging/*) echo none ;;
    # The settings of the script linter: scripts/lint-scripts.sh runs whenever it changes, nothing builds.
    .shellcheckrc) echo none ;;
    # Its own workspace, built and checked on its own (tools/bench/README.md).
    tools/bench/*) echo none ;;
    # The download server's counter, a Cloudflare Worker with its own tests (bun test there).
    tools/feed-counter/*) echo none ;;
    *) echo all ;;   # unknown: safer to run everything than to miss a test that reads it
  esac
}

rank() { case "$1" in all) echo 4 ;; app) echo 3 ;; text) echo 1 ;; *) echo 0 ;; esac; }

# The table above is only safe while two things hold, both read from the sources (no cargo):
# - a crate whose own scope is narrower than `all` is a dependency of no other crate in the workspace;
# - a file a crate include_str!/include_bytes!s from outside itself is in a scope that runs that
#   crate's tests (`all`, or the crate's own scope).
# Wire, once wire-only, broke the first; packaging/icon, once app-only, the second.
if [ "$selftest" = 1 ]; then
  bad=0 checked=0
  for dir in crates/*/; do
    dir="${dir%/}" name="${dir#crates/}"
    s="$(scope_of "$dir/src/lib.rs" | cut -d' ' -f1)"
    [ "$s" = all ] && continue
    for toml in crates/*/Cargo.toml; do
      [ "$toml" = "$dir/Cargo.toml" ] && continue
      if grep -Eq "^${name}(\.workspace)? *=" "$toml"; then
        echo "scope table: $dir is '$s', but ${toml%/Cargo.toml} depends on it (its changes must be 'all')" >&2
        bad=1
      fi
    done
  done
  while IFS=: read -r file target; do
    [ -n "$file" ] || continue
    crate="$(cut -d/ -f1-2 <<<"$file")"
    resolved="$(cd "$(dirname "$file")/$(dirname "$target")" 2>/dev/null && pwd)/$(basename "$target")" || continue
    rel="${resolved#"$PWD"/}"
    case "$rel" in "$crate"/*) continue ;; esac
    checked=$((checked + 1))
    need="$(scope_of "$file" | cut -d' ' -f1)" got="$(scope_of "$rel" | cut -d' ' -f1)"
    if [ "$got" != all ] && [ "$got" != "$need" ]; then
      echo "scope table: $file includes $rel, which is '$got' (needs '$need'$([ "$need" = all ] || echo " or 'all'"))" >&2
      bad=1
    fi
  done < <(grep -rHoE --include='*.rs' 'include_(str|bytes)!\("[^"]+"\)' crates \
             | sed -E 's/^([^:]+):include_(str|bytes)!\("([^"]+)"\)$/\1:\3/')
  [ "$checked" -gt 0 ] || { echo "scope table: found no include_str!/include_bytes! to check" >&2; bad=1; }
  [ "$bad" = 0 ] && echo "scope table: ok ($checked includes from outside their crate)" >&2
  exit "$bad"
fi

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

if [ "$scope_only" = 1 ]; then
  echo "scope=$scope"
  echo "deps=$deps"
  exit 0
fi

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
    steps+=("cargo clippy --quiet -p wormhole -p wormhole-rules --all-targets --features wormhole/devctl -- -D warnings")
    steps+=("cargo test --quiet -p wormhole -p wormhole-rules --features wormhole/devctl")
    steps+=("cargo check --quiet -p wormhole")
    ;;
  text)
    if [ "$rust_changed" = 1 ]; then
      steps+=("cargo fmt --all")
      steps+=("cargo clippy --quiet -p wormhole-rules --all-targets -- -D warnings")
    fi
    steps+=("cargo test --quiet -p wormhole-rules")
    ;;
esac
[ "$deps" = 1 ] && steps+=("cargo deny check")

# The history steps go first. They read the git history, so they run for every scope, `none` included.
history=("python3 scripts/commits.py check-range $base..HEAD"
         "python3 scripts/commits.py changelog-needed --include-worktree $base..HEAD")
grep -qx 'CHANGELOG.md' <<<"$changed" && history+=("python3 scripts/changelog.py check")
grep -Eqx '(scripts/worktree\.sh|crates/[^/]+/Cargo\.toml)' <<<"$changed" && history+=("scripts/worktree.sh --selftest")
grep -qx 'scripts/check.sh' <<<"$changed" && history+=("scripts/check.sh --selftest")
grep -qx 'scripts/changelog.py' <<<"$changed" && history+=("python3 scripts/changelog.py selftest")
grep -qx 'scripts/commits.py' <<<"$changed" && history+=("python3 scripts/commits.py selftest")
grep -qx 'scripts/perf_tools.py' <<<"$changed" && history+=("python3 -I scripts/perf_tools.py selftest")
# The workflows and the shell scripts are linted (shellcheck, actionlint, zizmor) whatever the scope,
# `none` included: a workflow change builds nothing, so this is the only check it gets before it runs.
grep -Eq '^(\.github/workflows/|scripts/|tools/visual/|\.claude/hooks/|\.claude/skills/[^/]+/scripts/|packaging/.*\.sh$|\.shellcheckrc$)' <<<"$changed" \
  && history+=("scripts/lint-scripts.sh")
# The structure check reads the sources as text, so it comes before the first compile too.
structure=()
if [ "$scope" != none ]; then
  structure+=("python3 scripts/structure.py check")
  grep -qx 'scripts/structure.py' <<<"$changed" && structure+=("python3 scripts/structure.py selftest")
fi
steps=("${history[@]}" "${structure[@]+"${structure[@]}"}" "${steps[@]+"${steps[@]}"}")

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
