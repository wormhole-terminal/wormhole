# Shared helpers for the packaging and devctl scripts. Source it, don't run it.
# shellcheck shell=bash disable=SC2034  # variables are used by the sourcing scripts

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGING="$ROOT/packaging"
# shellcheck source=../packaging/app.env
. "$PACKAGING/app.env"

log()  { printf '\033[1;34m==>\033[0m %s\n' "$*" >&2; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::warning::$*"; fi; }
die()  { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error::$*"; fi; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "'$1' not found. ${2:-}"; }

# workspace.package.version from the root Cargo.toml (the only version source).
app_version() {
  awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f && /^version[[:space:]]*=/{gsub(/^[^"]*"|".*$/,"");print;exit}' \
    "$ROOT/Cargo.toml"
}

APP_VERSION="$(app_version)"
[ -n "$APP_VERSION" ] || die "could not read workspace.package.version from Cargo.toml"

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP_PATH_DEFAULT="$TARGET_DIR/release/$APP_NAME.app"
DIST_DEFAULT="$TARGET_DIR/release/dist"
ARCHIVE_NAME="$APP_NAME-$APP_VERSION-macos.app.tar.gz"
DMG_NAME="$APP_NAME-$APP_VERSION-macos.dmg"

# True when the bundle carries a real Developer ID signature (not ad-hoc).
is_developer_id_signed() {
  # Not `codesign | grep -q`: grep stops at the first match, codesign dies of SIGPIPE, and under pipefail
  # a correctly signed app then reads as unsigned (exit 141).
  local info
  info="$(codesign -dvv "$1" 2>&1)" || return 1
  grep -q '^Authority=Developer ID Application' <<<"$info"
}

# The restricted entitlement behind Time Sensitive notifications. It is only valid when an
# embedded provisioning profile authorizes it, so scripts/entitlements.sh adds it to
# Developer ID builds that have the profile and never to the base set.
TIME_SENSITIVE_ENTITLEMENT=com.apple.developer.usernotifications.time-sensitive

# plist_get <plist> <key path>: the value at a PlistBuddy key path (":A:B"), empty when absent.
plist_get() { /usr/libexec/PlistBuddy -c "Print $2" "$1" 2>/dev/null || true; }

# profile_payload <profile> <out>: the plain plist inside a (CMS-signed) .provisionprofile.
profile_payload() {
  security cms -D -i "$1" -o "$2" 2>/dev/null || die "not a readable provisioning profile: $1"
}

# The PATH a test instance gets: the system's folders alone, as an app started from the Finder
# has, so what the Mac running it has installed (Homebrew, npm, …) stays out of its screenshots.
ISOLATED_PATH=/usr/bin:/bin:/usr/sbin:/sbin

# start_isolated <binary> <run dir> <start dir> <shell> [VAR=value ...]: start a devctl build of
# Wormhole kept apart from the person's own setup: data dir, HOME, agent home and the command line
# tool's folder (cli-bin, where Settings -> Command line tool puts `wh`: never /usr/local/bin) in <run dir>,
# $ISOLATED_PATH for a PATH and no other place to find agents' commands (WORMHOLE_AGENT_PATH
# empty), Touch ID answered "cancel", notifications logged instead of shown, and background mode
# (it opens behind their windows and never takes the focus). Later VAR=value pairs win. Sets $pid
# and writes it to <run dir>/pid; commands go to <run dir>/ctl/cmd, the app's output to
# <run dir>/app.log.
start_isolated() {
  local bin="$1" run="$2" start="$3" shell="$4" home
  shift 4
  mkdir -p "$run/data" "$run/ctl" "$run/home" "$run/agent-home"
  home="$(cd "$run/home" && pwd -P)"
  # Variables a Wormhole tab sets belong to that instance, not this one (and their old names,
  # which a tab still gets and the app reads when the new ones are missing).
  env -u WORMHOLE_SOCKET -u WORMHOLE_TAB_ID -u WORMHOLE_BIN -u ANDROMEDA_SOCKET -u ANDROMEDA_TAB_ID -u ANDROMEDA_BIN \
    PATH="$ISOLATED_PATH" WORMHOLE_AGENT_PATH= \
    HOME="$home" WORMHOLE_DATA_DIR="$run/data" WORMHOLE_DEVCTL="$run/ctl" \
    WORMHOLE_AGENT_HOME="$run/agent-home" WORMHOLE_CLI_DIR="$run/cli-bin" \
    WORMHOLE_SHELL="$shell" WORMHOLE_START_DIR="$start" \
    WORMHOLE_AUTH=cancel WORMHOLE_NOTIFY=log WORMHOLE_BACKGROUND=1 "$@" \
    "$bin" </dev/null >"$run/app.log" 2>&1 &
  pid=$!
  echo "$pid" > "$run/pid"
}

# stop_isolated <pid> <ctl dir>: ask that instance to quit, then TERM and KILL that PID alone if it
# doesn't. Fails when it is still running.
stop_isolated() {
  local pid="$1" ctl="$2"
  kill -0 "$pid" 2>/dev/null || return 0
  printf 'quit\n' > "$ctl/cmd.tmp" && mv "$ctl/cmd.tmp" "$ctl/cmd"
  for _ in $(seq 50); do kill -0 "$pid" 2>/dev/null || return 0; sleep 0.2; done
  warn "instance $pid ignored quit; sending TERM"
  kill -TERM "$pid" 2>/dev/null || true
  for _ in $(seq 25); do kill -0 "$pid" 2>/dev/null || return 0; sleep 0.2; done
  kill -KILL "$pid" 2>/dev/null
  sleep 0.2
  ! kill -0 "$pid" 2>/dev/null
}
