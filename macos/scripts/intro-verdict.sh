# shellcheck shell=bash
# What macos/scripts/check-bundle.sh says of the intro once the shaders compiled and `intro` was asked to play. Source it, don't run it.
#
#   intro_verdict <intro-support> <windows before> <windows after> <what `intro` replied> <the `windows` list>
#
# The check passes when the intro's overlay was one more window, or when the app says it sees no screen (a virtual machine with no
# display: the overlay has nowhere to open, and nothing is lost of the bundle, whose shaders were found and compiled). Any other
# silence fails, and the facts the app gave come with it, so a machine that should have shown the intro and did not says why in
# the log of the run. Prints one line when it passes; when it fails the first line is the failure and the facts follow it.
# Returns 0 to go on, 1 to fail.
indent() {
  local line
  while IFS= read -r line; do printf '    %s\n' "$line"; done
}

intro_verdict() {
  local support="$1" before="$2" after="$3" reply="$4" windows="$5" screens
  if [ "$after" -gt "$before" ]; then
    echo "found in the copy's resources and compiled, the intro played"
    return 0
  fi
  screens="$(sed -n 's/^screens //p' <<<"$support" | head -n 1)"
  if [ "${screens%% *}" = 0 ]; then
    echo "found in the copy's resources and compiled; the app sees no screen on this machine, so the intro had nowhere to open and was not played"
    return 0
  fi
  echo "the shaders compiled ($(sed -n 's/^metal //p' <<<"$support")) but the intro showed no overlay window"
  echo "  the app replied to intro: $reply"
  echo "  its windows, $before shown before and $after after:"
  indent <<<"$windows"
  echo "  intro-support:"
  indent <<<"$support"
  return 1
}
