#!/usr/bin/env bash
# Checks a built "Wormhole Beta.app" (macos/scripts/bundle.sh makes it): what it is, and that it runs with nothing of this checkout.
#
#   macos/scripts/check-bundle.sh [path/to/Wormhole\ Beta.app]     # default macos/dist/"Wormhole Beta.app"
#
# What it is: both executables universal; the bundle id, both versions and the minimum system of beta.env; a signature that
# verifies, the app's identifier and the host's "<bundle id>.host"; the resource bundles in Contents/Resources and nothing loose
# at the bundle's root; the fonts.
# That it runs: a COPY of the app, in a scratch folder, started as a background test instance (macos/parity/DRIVING.md) with a
# scratch HOME and /bin/cat for a shell, so nothing of the person's or of this checkout is read. Through its control folder it
# must have loaded the keymap, the themes, the icons and the intro's shaders (each comes from a resource bundle, and an
# accessor that cannot find its bundle ends the app), found JetBrains Mono (not Menlo), started a session host from beside its
# own executable (a shipped library ignores WORMHOLE_HOST_BIN) and quit cleanly, leaving no host. The saved terminal text, whose
# key a shipped library keeps in the Keychain, is off in the scratch config.json, so no Keychain prompt can come of this.
# On the runner this is what proves the resource accessor: the build system's own cannot be tested here without it.
# The shaders are three separate facts (`intro-support`, DRIVING.md), and the output says which were shown: the sources found in
# the copy's own resources (always required), compiled (needs a Metal device) and the intro played (needs them compiled). A
# machine with no Metal device, a virtual machine say, is not a defect of the bundle: it passes, and says it did not play.
# When anything of the instance part fails, the tail of the app's own log is printed.
set -euo pipefail
export WORMHOLE_APP_ENV=packaging/beta.env
. "$(dirname "$0")/../../scripts/lib.sh"

app="${1:-$ROOT/macos/dist/$APP_NAME.app}"
[ -d "$app" ] || die "no app bundle at $app (macos/scripts/bundle.sh makes it)"
expect() { [ "$2" = "$3" ] || die "$1: '$2', expected '$3'"; }

# --- what it is ---------------------------------------------------------------------------------------------------------
executable="$app/Contents/MacOS/$APP_BINARY"
host="$app/Contents/MacOS/$HOST_BINARY"
architectures() { lipo -archs "$1" | tr ' ' '\n' | sort | tr '\n' ' '; }
expect "the app's architectures" "$(architectures "$executable")" "arm64 x86_64 "
expect "the host's architectures" "$(architectures "$host")" "arm64 x86_64 "
plist="$app/Contents/Info.plist"
expect CFBundleIdentifier "$(plutil -extract CFBundleIdentifier raw "$plist")" "$BUNDLE_ID"
expect CFBundleShortVersionString "$(plutil -extract CFBundleShortVersionString raw "$plist")" "${APP_VERSION%%[-+]*}"
expect WormholeVersion "$(plutil -extract WormholeVersion raw "$plist")" "$APP_VERSION"
expect LSMinimumSystemVersion "$(plutil -extract LSMinimumSystemVersion raw "$plist")" "$MIN_MACOS"
codesign --verify --deep --strict "$app" || die "the signature of $app does not verify"
identifier_of() { codesign -d -vv "$1" 2>&1 | sed -n 's/^Identifier=//p'; }
expect "the app's signing identifier" "$(identifier_of "$app")" "$BUNDLE_ID"
expect "the host's signing identifier" "$(identifier_of "$host")" "$BUNDLE_ID.host"
[ -z "$(find "$app" -mindepth 1 -maxdepth 1 ! -name Contents)" ] || die "loose content at the bundle's root: codesign refuses it"
[ -n "$(find "$app/Contents/Resources" -maxdepth 1 -name '*.bundle')" ] || die "no resource bundle in Contents/Resources"
[ -z "$(find "$app" -name '*.bundle' ! -path "$app/Contents/Resources/*")" ] || die "a resource bundle outside Contents/Resources"
for face in Regular Bold Italic BoldItalic; do
  [ -f "$app/Contents/Resources/Fonts/JetBrainsMono/JetBrainsMono-$face.ttf" ] || die "JetBrainsMono-$face.ttf is not in the bundle"
done
[ -f "$app/Contents/Resources/Fonts/JetBrainsMono/OFL.txt" ] || die "the fonts' licence is not in the bundle"
log "$app: universal, $BUNDLE_ID $APP_VERSION, signed, its parts where they belong"

# --- that it runs on its own ---------------------------------------------------------------------------------------------
scratch="$(mktemp -d "${TMPDIR:-/tmp}/wormhole-beta-check.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
run="$scratch/run"
data="$run/Wormhole-swift-check"
pid=""
# What the app itself said, so that a failure on a machine nobody can look at explains itself: what it wrote to stderr, and the
# newest of its daily log files, `wormhole.<date>.log` (the intro logs why it did not play there; the hosts' own logs are beside it).
show_logs() {
  local daily="" log
  for log in "$data"/logs/wormhole.*.log; do
    [ -e "$log" ] || continue
    if [ -z "$daily" ] || [ "$log" -nt "$daily" ]; then daily="$log"; fi
  done
  printf 'the end of %s:\n' "$run/app.log" >&2
  tail -n 30 "$run/app.log" >&2 || true
  [ -z "$daily" ] || { printf 'the end of the app log %s:\n' "$(basename "$daily")" >&2; tail -n 30 "$daily" >&2 || true; }
}
finish() {
  [ "$?" -eq 0 ] || show_logs
  [ -z "$pid" ] || stop_isolated "$pid" "$run/ctl" || true
  stop_hosts "$data"
  rm -rf "$scratch"
}
trap finish EXIT
mkdir -p "$run" "$data"
ditto "$app" "$scratch/$APP_NAME.app"
printf '{"restore_terminal_text": false, "feature_flags": {"unlocked": true, "persistent_terminals": true}}\n' > "$data/config.json"
cd "$scratch"
start_isolated "$scratch/$APP_NAME.app/Contents/MacOS/$APP_BINARY" "$run" "$scratch" /bin/cat WORMHOLE_DATA_DIR="$data"

# send <line>...: one batch to the control folder, ended by a token that the app answers when every line before it is done.
send() {
  local token="t$RANDOM$RANDOM" n=0
  [ ! -f "$run/ctl/out.txt" ] || n="$(wc -l < "$run/ctl/out.txt")"
  { printf '%s\n' "$@" "sync $token"; } > "$run/ctl/cmd.tmp" && mv "$run/ctl/cmd.tmp" "$run/ctl/cmd"
  for _ in $(seq 150); do
    kill -0 "$pid" 2>/dev/null || die "the app ended while it ran: $(head -c 600 "$run/app.log")"
    grep -q "^sync $token\$" "$run/ctl/out.txt" 2>/dev/null && break
    sleep 0.2
  done
  tail -n +"$((n + 1))" "$run/ctl/out.txt" | grep -v "^sync $token\$" || true
}
for _ in $(seq 150); do [ -d "$run/ctl" ] && kill -0 "$pid" 2>/dev/null && break; sleep 0.2; done
state="$(send state)"
grep -q 'terminals=1' <<<"$state" || die "the app has no terminal: $state"
grep -q 'Quit ' <<<"$(send menus)" || die "the keymap is not loaded: no menus"
if grep -q '^error' <<<"$(send 'theme dracula')"; then die "the themes are not loaded"; fi
# The icon's file is the copy's own (the app writes a path without its /private).
icon="$(send 'file-icon main.rs')"
[[ "$icon" == *"${scratch#/private}/$APP_NAME.app/Contents/Resources/"*.svg* ]] || die "the file icons are not the copy's resources: $icon"
# The intro's shaders: found in the copy, compiled where there is a Metal device, and played where they compiled.
support="$(send intro-support)"
shaders="$(grep '^shaders ' <<<"$support" || true)"
compile="$(sed -n 's/^compile //p' <<<"$support")"
[ -n "$shaders" ] && [ -n "$compile" ] || die "the app did not answer intro-support: '$support'"
if [[ "$shaders" == *" missing "* ]]; then
  die "the intro's shader sources are not found in the copy: $shaders ($(find "$scratch/$APP_NAME.app/Contents/Resources" -name '*.metal' | wc -l | tr -d ' ') .metal files under its Contents/Resources)"
fi
[[ "$shaders" == *"/$(basename "$scratch")/$APP_NAME.app/Contents/Resources/"* ]] || die "the intro's shaders are not the copy's own resources: $shaders"
if [ "$compile" = "skipped: there is no Metal device" ]; then
  shaders_said="found in the copy's resources; this machine has no Metal device, the intro was not played"
elif [ "$compile" = ok ]; then
  # The intro's overlay is one more window while it plays. `intro` makes it before it answers and the style is the longest
  # (7 s), so it is there at once; the wait is for a slow machine, and bounded.
  windows() { send windows | grep -c '^(untitled) visible=true' || true; }
  before="$(windows)"
  send 'intro ftl' >/dev/null
  for _ in $(seq 25); do [ "$(windows)" -gt "$before" ] && break; sleep 0.2; done
  [ "$(windows)" -gt "$before" ] || die "the shaders compiled ($(sed -n 's/^metal //p' <<<"$support")) but the intro showed no overlay window"
  shaders_said="found in the copy's resources and compiled, the intro played"
else
  die "the intro's shaders are found but do not compile: $compile"
fi
log "intro: shaders $shaders_said"
# A session host of the app's own, from beside its executable: the terminal it makes is hosted by it.
send new >/dev/null
for _ in $(seq 50); do ls "$data"/hosts/*.meta >/dev/null 2>&1 && break; sleep 0.2; done
host_pid="$(sed -n 's/^pid=//p' "$data"/hosts/*.meta | head -n 1)"
[ -n "$host_pid" ] || die "no session host started"
host_command="$(ps -p "$host_pid" -o command=)"
[[ "$host_command" == *"/$(basename "$scratch")/$APP_NAME.app/Contents/MacOS/$HOST_BINARY "* ]] \
  || die "the session host is not the one beside the app: $host_command"
if grep -q 'JetBrains Mono not found' "$run/app.log"; then die "JetBrains Mono was not found: $(head -c 300 "$run/app.log")"; fi
send quit-end-all >/dev/null || true
for _ in $(seq 100); do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
if kill -0 "$pid" 2>/dev/null; then die "the app did not quit"; fi
pid=""
log "a copy of it, outside this checkout, started, loaded its keymap, themes and icons, found its shaders ($shaders_said), found its fonts, started a session host from beside itself, and quit"
