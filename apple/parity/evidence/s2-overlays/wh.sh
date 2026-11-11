#!/bin/bash
# wh.sh <x> args...: the real command line (the app's own executable) against instance <x>'s socket
X=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s2-overlays/apple/swift
out=$(env WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-s2-overlays-$X $W/.build/release/Wormhole "$@" 2>&1); rc=$?
line="$ Wormhole $* -> exit $rc"$'\n'"$out"
echo "$line"
[ -n "$EV" ] && echo "$line" >> "$EV"
