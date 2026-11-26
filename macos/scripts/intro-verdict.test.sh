#!/usr/bin/env bash
# The bundle check's verdict on the intro (intro-verdict.sh), on what the app says on each kind of machine: one that shows the
# overlay, one with no screen, and every way a machine with a screen can end with none. No app is started.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=intro-verdict.sh
. "$here/intro-verdict.sh"

failed=0
fail() { printf 'FAIL %s\n' "$*"; failed=$((failed + 1)); }

shaders=$'shaders 12/12 /copy/Contents/Resources/Wormhole_AuxWindows.bundle/Contents/Resources/shader\nmetal Apple Paravirtual device\ncompile ok'
# support <screens line> <window> <play> <playing>: the app's answer with those facts.
support() {
  printf '%s\n%s\nreduce-motion off\noverlay above the window\nwindow %s\nplay %s\nplaying %s' "$shaders" "$1" "$2" "$3" "$4"
}
windows=$'(untitled) visible=true sheet=false edited=false\nWormhole visible=true sheet=false edited=false'

# verdict <name> <expected status> <expected words, or none> <intro-support> <before> <after>
verdict() {
  local name="$1" status="$2" words="$3" out rc=0
  out="$(intro_verdict "$4" "$5" "$6" "intro ftl" "$windows")" || rc=$?
  [ "$rc" = "$status" ] || fail "$name: returned $rc, expected $status: $out"
  [ "$words" = none ] || [[ "$out" == *"$words"* ]] || fail "$name: no '$words' in: $out"
  if [ "$status" = 0 ] && [ "$(wc -l <<<"$out")" -ne 1 ]; then fail "$name: a pass is one line: $out"; fi
}

verdict "an overlay that is one more window" 0 "the intro played" "$(support "screens 1 1024x768" shown "opened shown" yes)" 1 2
verdict "a machine with no screen" 0 "the app sees no screen" "$(support "screens 0" shown "failed: noDevice" no)" 1 1
verdict "no screen, and the overlay counted all the same" 0 "the intro played" "$(support "screens 0" shown "opened shown" yes)" 1 2

verdict "never asked" 1 "play none" "$(support "screens 1 1024x768" shown none no)" 1 1
verdict "no window to play over" 1 "play no window" "$(support "screens 1 1024x768" none "no window" no)" 1 1
verdict "the overlay could not be made" 1 "play failed: noQueue" "$(support "screens 2 1512x982 1920x1080" shown "failed: noQueue" no)" 1 1
verdict "opened where nothing shows it" 1 "play opened hidden" "$(support "screens 1 1024x768" hidden "opened hidden" yes)" 1 1
verdict "opened and shown, not counted" 1 "play opened shown" "$(support "screens 1 1024x768" shown "opened shown" yes)" 1 1
verdict "an app that does not say" 1 "the shaders compiled (Apple Paravirtual device)" "$shaders" 1 1
verdict "the facts and the windows come with a failure" 1 "(untitled) visible=true" "$(support "screens 1 1024x768" shown none no)" 1 1
verdict "what the app replied comes with it" 1 "the app replied to intro: intro ftl" "$(support "screens 1 1024x768" shown none no)" 1 1

[ "$failed" = 0 ] || { printf 'intro verdict: %d failed\n' "$failed"; exit 1; }
echo "intro verdict: ok"
