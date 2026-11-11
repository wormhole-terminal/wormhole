#!/bin/bash
# c.sh <x> "cmd" ...: one control command per batch; prints each answer; appends to $EV when set
X=$1; shift
C=/tmp/sp-$X/ctl
for c in "$@"; do
  : > $C/out.txt; printf '%s\n' "$c" > $C/cmd.tmp; mv $C/cmd.tmp $C/cmd; sleep ${WAIT:-1.3}
  out="> $c"$'\n'"$(cat $C/out.txt)"
  echo "$out"
  [ -n "$EV" ] && echo "$out" >> "$EV"
done
