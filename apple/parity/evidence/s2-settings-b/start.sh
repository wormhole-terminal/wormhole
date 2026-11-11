#!/bin/zsh
# usage: start.sh <x> [extra env as K=V ...]: a fresh --background instance with the welcome flag on, fake home
x=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s2-settings-b
D=/tmp/Wormhole-swift-s2b-$x; C=/tmp/hd-s2b-$x; H=/tmp/s2b-home-$x
mkdir -p $D $C $H/.config/ghostty
[ -f $D/config.json ] || echo '{"feature_flags":{"terminal_welcome":true,"unlocked":true}}' > $D/config.json
[ -f $H/.config/ghostty/config ] || printf 'font-size = 17\nbackground-opacity = 0.92\n' > $H/.config/ghostty/config
cd $W/apple/swift
env HOME=$H WORMHOLE_DATA_DIR=$D WORMHOLE_SHELL=/bin/zsh WORMHOLE_AUTH=cancel WORMHOLE_DEVCTL=$C WORMHOLE_IMPORT_HOME=$H "$@" \
  $W/apple/swift/.build/release/Wormhole --background $ARGS > $C/app.log 2>&1 &
echo $! > $C/pid
echo pid $(cat $C/pid)
