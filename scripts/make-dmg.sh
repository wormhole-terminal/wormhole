#!/usr/bin/env bash
# Builds the download image: the app and a shortcut to /Applications on a background picture, in a window of
# a fixed size, with the app's icon as the volume's. Two ways, tried in this order:
#
#   1. dmgbuild (a pinned PyPI package, installed into target/dmg-venv on first use) writes the window layout
#      into the image's .DS_Store itself. No Finder, desktop session or permission: it also runs on CI.
#   2. Finder, driven by AppleScript, when dmgbuild cannot be installed (offline). Needs a desktop session and,
#      the first time, your OK for the terminal to control Finder.
#
#   scripts/make-dmg.sh <Wormhole.app> <out.dmg>
#
# Env:
#   REQUIRE_DMG_LAYOUT=1  fail when neither works (default: warn and ship a plain image)
#   DMG_LAYOUT=skip       do not try; a plain image
#
# The background is packaging/dmg/background.png + background@2x.png (packaging/dmg/make-background.swift draws
# them); the window and the icon positions are in packaging/dmg/settings.py and below, where its arrow expects them.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

app="${1:?usage: make-dmg.sh <Wormhole.app> <out.dmg>}"
out="${2:?usage: make-dmg.sh <Wormhole.app> <out.dmg>}"
[ -d "$app" ] || die "no app bundle at $app"
need hdiutil; need tiffutil

# Exact versions: this runs on the machine that signs the release.
DMGBUILD_PACKAGES=(dmgbuild==1.6.5 ds-store==1.3.1 mac-alias==2.2.2)

work="$(mktemp -d)"
dev=""
cleanup() {
  if [ -n "$dev" ]; then hdiutil detach "$dev" -quiet -force >/dev/null 2>&1 || true; fi
  rm -rf "$work"
}
trap cleanup EXIT

app_name="$(basename "$app")"
mkdir -p "$(dirname "$out")"
background="$work/background.tiff"
tiffutil -cathidpicheck "$PACKAGING/dmg/background.png" "$PACKAGING/dmg/background@2x.png" -out "$background" 2>/dev/null

plain() { # the app and the Applications shortcut, nothing else
  local stage="$work/plain"
  mkdir -p "$stage"
  ditto "$app" "$stage/$app_name"
  ln -s /Applications "$stage/Applications"
  cp "$PACKAGING/AppIcon.icns" "$stage/.VolumeIcon.icns"
  rm -f "$out"
  hdiutil create -quiet -volname "$APP_NAME" -srcfolder "$stage" -fs HFS+ -format ULFO -ov "$out"
}

with_dmgbuild() {
  local venv="$TARGET_DIR/dmg-venv"
  if [ ! -x "$venv/bin/dmgbuild" ]; then
    need python3
    log "installing dmgbuild into $venv"
    python3 -m venv "$venv" \
      && "$venv/bin/pip" install --quiet --disable-pip-version-check "${DMGBUILD_PACKAGES[@]}" || return 1
  fi
  rm -f "$out"
  "$venv/bin/dmgbuild" -s "$PACKAGING/dmg/settings.py" \
    -D "app=$app" -D "background=$background" -D "icon=$PACKAGING/AppIcon.icns" \
    "$APP_NAME" "$out" >"$work/dmgbuild.log" 2>&1 || { cat "$work/dmgbuild.log" >&2; return 1; }
}

with_finder() {
  need osascript
  # The same window as settings.py: 660 x 400 of content, and the title bar Finder puts on top of it.
  local win_x=200 win_y=120 win_w=660 win_h=400 title_bar=28
  local icon_size=128 text_size=13 app_x=165 app_y=185 apps_x=495 apps_y=185
  local stage="$work/finder" rw="$work/rw.dmg" attach mnt disk bounds
  mkdir -p "$stage/.background"
  ditto "$app" "$stage/$app_name"
  ln -s /Applications "$stage/Applications"
  cp "$background" "$stage/.background/background.tiff"
  hdiutil create -quiet -volname "$APP_NAME" -srcfolder "$stage" -fs HFS+ -format UDRW \
    -size "$(( $(du -sm "$stage" | cut -f1) + 40 ))m" -ov "$rw" || return 1

  # Finder must see the volume, so no -nobrowse. If a volume of this name is already mounted, this one gets
  # another name, which is read back from the mount point.
  attach="$(hdiutil attach "$rw" -readwrite -noverify -noautoopen 2>/dev/null)" || return 1
  dev="$(printf '%s\n' "$attach" | awk '/Apple_HFS/ {print $1; exit}')"
  mnt="$(printf '%s\n' "$attach" | awk -F'\t' '/Apple_HFS/ {print $NF; exit}')"
  [ -n "$dev" ] && [ -d "$mnt" ] || return 1
  disk="$(basename "$mnt")"
  bounds="$win_x, $win_y, $((win_x + win_w)), $((win_y + win_h + title_bar))"
  osascript >"$work/osa.log" 2>&1 <<EOF || { cat "$work/osa.log" >&2; return 1; }
tell application "Finder"
  tell disk "$disk"
    open
    set current view of container window to icon view
    set toolbar visible of container window to false
    set statusbar visible of container window to false
    set bounds of container window to {$bounds}
    set viewOptions to the icon view options of container window
    set arrangement of viewOptions to not arranged
    set icon size of viewOptions to $icon_size
    set text size of viewOptions to $text_size
    set background picture of viewOptions to file ".background:background.tiff"
    set position of item "$app_name" of container window to {$app_x, $app_y}
    set position of item "Applications" of container window to {$apps_x, $apps_y}
    update without registering applications
    delay 2
    close
  end tell
end tell
EOF
  # Finder writes .DS_Store when the window closes. It also deletes a .VolumeIcon.icns that was there before,
  # so the volume's icon goes in only now.
  for _ in 1 2 3 4 5 6 7 8 9 10; do [ -f "$mnt/.DS_Store" ] && break; sleep 1; done
  [ -f "$mnt/.DS_Store" ] || return 1
  cp "$PACKAGING/AppIcon.icns" "$mnt/.VolumeIcon.icns"
  SetFile -c icnC "$mnt/.VolumeIcon.icns"
  SetFile -a C "$mnt"
  chmod -Rf go-w "$mnt" 2>/dev/null || true
  sync
  local detached=""
  for _ in 1 2 3 4 5 6; do
    if hdiutil detach "$dev" -quiet >/dev/null 2>&1; then detached=1; break; fi
    sleep 2
  done
  [ -n "$detached" ] || hdiutil detach "$dev" -quiet -force >/dev/null
  dev=""
  rm -f "$out"
  hdiutil convert "$rw" -quiet -format ULFO -o "$out"
}

if [ "${DMG_LAYOUT:-}" = skip ]; then plain; exit 0; fi

if with_dmgbuild; then exit 0; fi
warn "dmgbuild did not make the disk image: trying Finder"
if [ -n "$dev" ]; then hdiutil detach "$dev" -quiet -force >/dev/null 2>&1 || true; dev=""; fi
if with_finder; then exit 0; fi
if [ -n "$dev" ]; then hdiutil detach "$dev" -quiet -force >/dev/null 2>&1 || true; dev=""; fi

if [ "${REQUIRE_DMG_LAYOUT:-0}" = 1 ]; then die "could not lay out the disk image with dmgbuild or Finder"; fi
warn "could not lay out the disk image: shipping a plain one"
plain
