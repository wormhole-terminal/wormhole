#!/usr/bin/env bash
# Reports whether a built Wormhole.app carries the Time Sensitive Notifications entitlement
# and an embedded provisioning profile, and fails when they disagree:
#
#   scripts/check-entitlements.sh [path]   # default: target/release/Wormhole.app
#
# - entitlement without profile: macOS refuses to launch the app (a restricted
#   entitlement needs a profile that authorizes it);
# - profile without entitlement: the build is half configured, "needs you" is delivered at Active;
# - both: the profile has to authorize the entitlement for the App ID the app is signed with.
# Neither (ad-hoc and local builds, no APPLE_PROVISIONING_PROFILE): fine, delivered at Active.
# scripts/bundle.sh runs this after signing.
set -euo pipefail
. "$(dirname "$0")/lib.sh"

app="${1:-$APP_PATH_DEFAULT}"
[ -d "$app" ] || die "not found: $app"
profile="$app/Contents/embedded.provisionprofile"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
signed="$work/signed.plist"
# An app without any entitlements prints nothing; an unsigned one makes codesign fail.
codesign -d --entitlements :- "$app" 2>/dev/null > "$signed" \
  || die "cannot read the code signature of $app (is it signed?)"

has_entitlement=0
[ "$(plist_get "$signed" ":$TIME_SENSITIVE_ENTITLEMENT")" = true ] && has_entitlement=1
has_profile=0
[ -f "$profile" ] && has_profile=1

if [ "$has_entitlement" = 1 ]; then
  printf 'entitlement  %s: present\n' "$TIME_SENSITIVE_ENTITLEMENT"
else
  printf 'entitlement  %s: absent\n' "$TIME_SENSITIVE_ENTITLEMENT"
fi
if [ "$has_profile" = 1 ]; then
  payload="$work/profile.plist"
  profile_payload "$profile" "$payload"
  printf 'profile      Contents/embedded.provisionprofile: present (%s, team %s, expires %s)\n' \
    "$(plist_get "$payload" ':Name')" "$(plist_get "$payload" ':TeamIdentifier:0')" \
    "$(plutil -extract ExpirationDate raw -o - "$payload" 2>/dev/null || echo unknown)"
else
  printf 'profile      Contents/embedded.provisionprofile: absent\n'
fi

if [ "$has_entitlement" = 1 ] && [ "$has_profile" = 0 ]; then
  die "$app carries $TIME_SENSITIVE_ENTITLEMENT without an embedded provisioning profile: macOS would refuse to launch it"
fi
if [ "$has_entitlement" = 0 ] && [ "$has_profile" = 1 ]; then
  die "$app embeds a provisioning profile but is not signed with $TIME_SENSITIVE_ENTITLEMENT (sign with scripts/bundle.sh)"
fi
if [ "$has_entitlement" = 0 ]; then
  log "no Time Sensitive entitlement: 'needs you' notifications are delivered at Active"
  exit 0
fi

# Both are there: the profile must grant what the signature claims.
app_id="$(plist_get "$signed" ':com.apple.application-identifier')"
bundle_id="$(plist_get "$app/Contents/Info.plist" ':CFBundleIdentifier')"
[ "$(plist_get "$payload" ":Entitlements:$TIME_SENSITIVE_ENTITLEMENT")" = true ] \
  || die "the embedded profile does not grant $TIME_SENSITIVE_ENTITLEMENT"
[ -n "$app_id" ] || die "the signature has no com.apple.application-identifier"
[ "$app_id" = "$(plist_get "$payload" ':Entitlements:com.apple.application-identifier')" ] \
  || die "signed App ID '$app_id' differs from the embedded profile's"
[ "$app_id" = "$(plist_get "$payload" ':TeamIdentifier:0').$bundle_id" ] \
  || die "signed App ID '$app_id' is not <profile team>.$bundle_id"
[ "$(plist_get "$signed" ':com.apple.developer.team-identifier')" = "$(plist_get "$payload" ':TeamIdentifier:0')" ] \
  || die "signed com.apple.developer.team-identifier differs from the embedded profile's team"
log "Time Sensitive entitlement is authorized by the embedded profile"
