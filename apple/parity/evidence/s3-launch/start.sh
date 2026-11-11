#!/bin/zsh
# usage: start.sh <x> [K=V ...]: a fresh --background instance of this worktree's build (data dir, control folder and home are scratch)
x=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s3-launch
D=/tmp/Wormhole-swift-s3-launch-$x; C=/tmp/hd-s3-launch-$x; H=/tmp/s3-launch-home-$x
mkdir -p $D $C $H
cd $W/apple/swift
env HOME=$H WORMHOLE_DATA_DIR=$D WORMHOLE_SHELL=/bin/zsh WORMHOLE_AUTH=cancel WORMHOLE_DEVCTL=$C WORMHOLE_AGENT_HOME=$H "$@" \
  $W/apple/swift/.build/release/Wormhole --background $ARGS > $C/app.log 2>&1 &
echo $! > $C/pid
echo pid $(cat $C/pid)
