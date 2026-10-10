#!/usr/bin/env bash
# Prints the entitlements scripts/bundle.sh signs Wormhole.app with.
#
#   scripts/entitlements.sh                           # the base set (packaging/entitlements.plist)
#   scripts/entitlements.sh --profile FILE [--out F]  # base + what the profile authorizes
#
# The base file in git is what every build gets. With --profile (a Developer ID
# .provisionprofile for the app's App ID with Time Sensitive Notifications enabled) the
# output adds the keys macOS needs to accept a restricted entitlement that an embedded
# profile authorizes: com.apple.application-identifier (<TEAMID>.<bundle id>),
# com.apple.developer.team-identifier and the Time Sensitive entitlement itself.
# The profile must really match (macOS, Developer ID, not expired, this App ID, grants the
# entitlement, same team as APPLE_TEAM_ID / APPLE_SIGNING_IDENTITY when set), otherwise
# this fails: an app that claims an entitlement its profile does not authorize will not launch.
# Output goes to stdout unless --out is given.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

profile=""
out=""
while [ $# -gt 0 ]; do
  case "$1" in
    --profile) shift; profile="${1:?--profile needs a .provisionprofile file}" ;;
    --out) shift; out="${1:?--out needs a file name}" ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
  shift
done

base="$PACKAGING/entitlements.plist"
emit() { if [ -n "$out" ]; then cp "$1" "$out"; else cat "$1"; fi; }

if [ -z "$profile" ]; then
  emit "$base"
  exit 0
fi

[ -f "$profile" ] || die "provisioning profile not found: $profile"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
payload="$work/profile.plist"
profile_payload "$profile" "$payload"

name="$(plist_get "$payload" ':Name')"
what="provisioning profile '${name:-$profile}'"
plist_get "$payload" ':Platform' | grep -qw OSX || die "$what is not a macOS profile (Platform is not OSX)"
# Developer ID profiles apply to all devices; development profiles list devices and App Store
# profiles can't run outside the store.
[ "$(plist_get "$payload" ':ProvisionsAllDevices')" = true ] \
  || die "$what is not a Developer ID profile: create it under Profiles > Distribution > Developer ID"
expires="$(plutil -extract ExpirationDate raw -o - "$payload" 2>/dev/null || true)"
[ -n "$expires" ] || die "$what has no ExpirationDate"
# ISO 8601 in UTC sorts as text.
[ "$expires" \> "$(date -u +%Y-%m-%dT%H:%M:%SZ)" ] || die "$what expired on $expires: create a new one"

team="$(plist_get "$payload" ':TeamIdentifier:0')"
[ -n "$team" ] || die "$what has no TeamIdentifier"
app_id="$(plist_get "$payload" ':Entitlements:com.apple.application-identifier')"
want_id="$team.$BUNDLE_ID"
case "$app_id" in
  *'*') die "$what is a wildcard profile ($app_id): capabilities need a profile for the explicit App ID $want_id" ;;
esac
[ "$app_id" = "$want_id" ] || die "$what is for App ID '${app_id:-none}' but this app is '$want_id'"
[ "$(plist_get "$payload" ":Entitlements:$TIME_SENSITIVE_ENTITLEMENT")" = true ] \
  || die "$what does not grant $TIME_SENSITIVE_ENTITLEMENT: enable Time Sensitive Notifications on the App ID, then regenerate and download the profile"

# The App ID prefix has to be the team that signs: compare with what the caller says it signs as.
identity_team=""
identity_re='\(([A-Z0-9]{10})\)$'
if [[ "${APPLE_SIGNING_IDENTITY:-}" =~ $identity_re ]]; then identity_team="${BASH_REMATCH[1]}"; fi
for expected in "${APPLE_TEAM_ID:-}" "${WORMHOLE_TEAM_ID:-}" "$identity_team"; do
  [ -z "$expected" ] || [ "$expected" = "$team" ] \
    || die "$what belongs to team $team but the build is configured for team $expected"
done

ents="$work/entitlements.plist"
cp "$base" "$ents"
chmod u+w "$ents"
# PlistBuddy drops the base file's XML comments, which is fine for a generated copy.
put() {
  /usr/libexec/PlistBuddy -c "Delete :$1" "$ents" >/dev/null 2>&1 || true
  /usr/libexec/PlistBuddy -c "Add :$1 $2 $3" "$ents" >/dev/null
}
put com.apple.application-identifier string "$app_id"
put com.apple.developer.team-identifier string "$team"
put "$TIME_SENSITIVE_ENTITLEMENT" bool true
plutil -lint -s "$ents"
emit "$ents"
