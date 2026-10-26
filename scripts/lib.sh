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

# set_app_build_args <comma-separated app features, may be empty> [<tree>]: fills APP_BUILD_ARGS with the
# cargo arguments that build the app and its session host together. The host is a second binary the
# app starts from beside its own executable (core::host_binary), and `cargo build` in the root builds
# only the app (default-members), so whatever copies the app out for a test instance builds and
# copies the host too. With several `-p`, a feature is named with its package (devctl becomes
# wormhole/devctl). A tree without the host crate (an old revision) builds the app alone.
set_app_build_args() {
  local tree="${2:-$ROOT}" feature features="" list=()
  APP_BUILD_ARGS=(-p wormhole)
  [ ! -d "$tree/crates/wormhole-host" ] || APP_BUILD_ARGS+=(-p wormhole-host)
  IFS=, read -ra list <<<"${1:-}"
  for feature in ${list[@]+"${list[@]}"}; do
    [ -z "$feature" ] || features="${features:+$features,}wormhole/$feature"
  done
  [ -z "$features" ] || APP_BUILD_ARGS+=(--features "$features")
}

# copy_host_beside <built app binary> <folder>: the session host built next to the app, copied into
# the folder the app's own copy went to (a clone on APFS, like the app's). Nothing when there is none.
copy_host_beside() {
  local host
  host="$(dirname "$1")/$HOST_BINARY"
  [ -x "$host" ] || return 0
  rm -f "$2/$HOST_BINARY"
  cp -c "$host" "$2/$HOST_BINARY" 2>/dev/null || cp "$host" "$2/$HOST_BINARY"
}

# workspace.package.version from the root Cargo.toml (the only version source).
app_version() {
  awk '/^\[workspace\.package\]/{f=1;next} /^\[/{f=0} f && /^version[[:space:]]*=/{gsub(/^[^"]*"|".*$/,"");print;exit}' \
    "$ROOT/Cargo.toml"
}

APP_VERSION="$(app_version)"
[ -n "$APP_VERSION" ] || die "could not read workspace.package.version from Cargo.toml"

# Sets workspace.package.version in Cargo.toml, and the workspace crates' entries in Cargo.lock
# (nothing else changes; the index is fetched only when there is no copy, as on a fresh CI runner).
# Re-source this file (or start a new script) for APP_VERSION to follow.
set_app_version() {
  local tmp
  tmp="$(mktemp)"
  awk -v v="$1" '
    /^\[workspace\.package\]/ { f = 1; print; next }
    /^\[/ { f = 0 }
    f && !done && /^version[[:space:]]*=/ { print "version = \"" v "\""; done = 1; next }
    { print }' "$ROOT/Cargo.toml" > "$tmp"
  cat "$tmp" > "$ROOT/Cargo.toml"
  rm -f "$tmp"
  [ "$(app_version)" = "$1" ] || die "could not set the version in Cargo.toml"
  (cd "$ROOT" && { cargo update --workspace --offline --quiet 2>/dev/null || cargo update --workspace --quiet; })
}

# The version of a build of main between releases (the Unstable channel): after X.Y.Z it is
# X.Y.(Z+1)-dev.N, after a prerelease X.Y.Z-beta.1 it is X.Y.Z-beta.1.dev.N, N being the commits
# since that release's tag. Both sort after the release and before the next one, so the updater
# moves across them. Prints nothing at the release's own commit, which the release itself covers.
dev_version() {
  local n major minor patch
  n="$(git -C "$ROOT" rev-list --count "v$APP_VERSION..HEAD")" || die "no tag v$APP_VERSION to count from"
  [ "$n" -gt 0 ] || return 0
  case "$APP_VERSION" in
    *-*) echo "$APP_VERSION.dev.$n" ;;
    *)
      IFS=. read -r major minor patch <<<"$APP_VERSION"
      echo "$major.$minor.$((patch + 1))-dev.$n"
      ;;
  esac
}

TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
APP_PATH_DEFAULT="$TARGET_DIR/release/$APP_NAME.app"
DIST_DEFAULT="$TARGET_DIR/release/dist"

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
# empty), the places other terminals keep settings (XDG_CONFIG_HOME, XDG_CONFIG_DIRS,
# KITTY_CONFIG_DIRECTORY, WEZTERM_CONFIG_FILE) pointed into <run dir>/home or unset, Touch ID answered
# "cancel", notifications logged instead of shown, and background mode
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
  # Where other terminals' settings are looked for (the Import page reads these) is the scratch home's,
  # never the person's own ~/.config: XDG_CONFIG_HOME inside it, the other three unset.
  env -u WORMHOLE_SOCKET -u WORMHOLE_TAB_ID -u WORMHOLE_BIN -u ANDROMEDA_SOCKET -u ANDROMEDA_TAB_ID -u ANDROMEDA_BIN \
    -u XDG_CONFIG_DIRS -u KITTY_CONFIG_DIRECTORY -u WEZTERM_CONFIG_FILE \
    PATH="$ISOLATED_PATH" WORMHOLE_AGENT_PATH= XDG_CONFIG_HOME="$home/.config" \
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

# stop_hosts <data dir>: end the session hosts (wormhole-host) of that data dir, which outlive their
# app by design. Each <data dir>/hosts/<id>.meta names its host: `pid=` and `socket=`. A pid counts only
# while `ps` still shows a process called wormhole-host (a recycled pid is somebody else's). TERM first,
# which makes the host hang up its shell and remove its socket and meta; KILL after 3 s, when its two
# files are removed here by the exact names the meta gave. The glob only lists files; nothing is
# removed with it.
stop_hosts() {
  local dir="$1/hosts" meta pid sock comm n=0
  [ -d "$dir" ] || return 0
  for meta in "$dir"/*.meta; do
    [ -f "$meta" ] || continue
    pid="$(sed -n 's/^pid=//p' "$meta" | head -n 1)"
    sock="$(sed -n 's/^socket=//p' "$meta" | head -n 1)"
    if [[ "$pid" =~ ^[0-9]+$ ]] && comm="$(ps -p "$pid" -o comm= 2>/dev/null)" && [[ "$comm" == *wormhole-host ]]; then
      kill -TERM "$pid" 2>/dev/null || true
      for _ in $(seq 15); do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
      if kill -0 "$pid" 2>/dev/null; then
        warn "host $pid ignored TERM; sending KILL"
        kill -KILL "$pid" 2>/dev/null || true
        sleep 0.2
        rm -f -- "$meta"
        case "$sock" in /*.sock) rm -f -- "$sock" ;; esac
      fi
      n=$((n + 1))
    elif ! [[ "$pid" =~ ^[0-9]+$ ]] || ! kill -0 "$pid" 2>/dev/null; then
      # A leftover of a host that is gone. A live process under another name (a host started through
      # WORMHOLE_HOST_BIN) keeps its meta: without it nothing could find that host any more.
      rm -f -- "$meta"
    else
      warn "$meta names pid $pid, which runs but is not a wormhole-host; left alone"
    fi
  done
  [ "$n" -eq 0 ] || log "ended $n session host(s) in $dir"
  return 0
}
