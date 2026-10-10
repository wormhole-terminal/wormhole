#!/usr/bin/env bash
# Compiles packaging/icon/AppIcon.icon (the Icon Composer document) into what the app bundle ships:
#   packaging/Assets.car   the icon for macOS 26+ (Liquid Glass; light, dark, tinted and clear looks)
#                          and the flattened renders older macOS reads through CFBundleIconName
#   packaging/AppIcon.icns the CFBundleIconFile fallback
# Needs Xcode (actool, swift, iconutil) and Icon Composer.app (ictool). Both outputs are committed, so this
# only runs when the icon changes.
#
# actool's own AppIcon.icns is a flat approximation: it drops the glass, the specular highlights and the blur
# the icon is made of. So the .icns comes from Icon Composer's own render of the document (ictool), laid on
# the standard macOS tile with its shadow by make-iconset.swift. Without Icon Composer it falls back to
# actool's copy, with a warning.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
packaging="$(cd "$here/.." && pwd)"
# shellcheck source=../app.env
. "$packaging/app.env"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

xcrun actool "$here/AppIcon.icon" --compile "$work" --platform macosx \
  --minimum-deployment-target "$MIN_MACOS" --app-icon AppIcon \
  --output-partial-info-plist "$work/partial-Info.plist" \
  --errors --warnings --output-format human-readable-text >"$work/actool.log" 2>&1 || {
  cat "$work/actool.log" >&2
  exit 1
}
# actool writes its files even when the document fails validation (e.g. more than four groups).
if grep -qiE 'error|warning' "$work/actool.log"; then
  cat "$work/actool.log" >&2
  exit 1
fi
cp "$work/Assets.car" "$packaging/Assets.car"

ictool="/Applications/Icon Composer.app/Contents/Executables/ictool"
if [ -x "$ictool" ]; then
  # Design generation 27: what Icon Composer shows and exports by default (the same pixels as its
  # File > Export), pinned so a newer Icon Composer cannot change the look unnoticed. 26 is the older,
  # flatter glass.
  "$ictool" "$here/AppIcon.icon" --export-image --output-file "$work/render.png" --platform macOS \
    --rendition Default --width 1024 --height 1024 --scale 1 --design-generation 27 >"$work/ictool.log" 2>&1 || {
    cat "$work/ictool.log" >&2
    exit 1
  }
  swift "$here/make-iconset.swift" "$work/render.png" "$work/AppIcon.iconset"
  iconutil -c icns "$work/AppIcon.iconset" -o "$packaging/AppIcon.icns"
else
  echo "warning: Icon Composer.app not found, using actool's flat AppIcon.icns" >&2
  cp "$work/AppIcon.icns" "$packaging/AppIcon.icns"
fi
echo "wrote $packaging/Assets.car and $packaging/AppIcon.icns"
