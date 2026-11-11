#!/bin/zsh
# usage: start.sh <x> [extra env K=V ...]: a fresh --background Swift instance (scratch HOME with fake Ghostty config)
x=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s3-settings
D=/tmp/Wormhole-swift-s3-settings-$x; C=/tmp/hd-s3-settings-$x; H=/tmp/s3-settings-home-$x
mkdir -p $D $C $H/.config/ghostty
[ -f $D/config.json ] || echo '{"feature_flags":{"unlocked":true}}' > $D/config.json
[ -f $H/.config/ghostty/config ] || printf 'font-size = 17\nbackground-opacity = 0.92\n' > $H/.config/ghostty/config
cd $W/apple/swift
env HOME=$H WORMHOLE_DATA_DIR=$D WORMHOLE_SHELL=/bin/zsh WORMHOLE_AUTH=${AUTH:-cancel} WORMHOLE_DEVCTL=$C WORMHOLE_IMPORT_HOME=$H "$@" \
  $W/apple/swift/.build/release/Wormhole --background > $C/app.log 2>&1 &
echo $! > $C/pid
echo pid $(cat $C/pid) data=$D ctl=$C
