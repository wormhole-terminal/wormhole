#!/bin/bash
# start.sh <x> [app options...]: a Swift instance for lane S-platform: --background, scratch data /tmp/Wormhole-swift-s-platform-<x>, ctl in /tmp/sp-<x>/ctl
X=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s-platform/apple/swift
D=/tmp/Wormhole-swift-s-platform-$X; R=/tmp/sp-$X
mkdir -p $R/ctl $R/repo $D
[ -d $R/repo/.git ] || (cd $R/repo && git init -q && echo hi > a.txt && git add a.txt && git -c user.name=t -c user.email=t@t commit -qm init)
[ -n "$SP_CONFIG" ] && [ ! -e $D/config.json ] && printf '%s' "$SP_CONFIG" > $D/config.json
cd $R/repo
env WORMHOLE_DATA_DIR=$D WORMHOLE_SHELL=${SP_SHELL:-/bin/zsh} WORMHOLE_DEVCTL=$R/ctl WORMHOLE_BACKGROUND=1 WORMHOLE_AUTH=${SP_AUTH:-cancel} WORMHOLE_NOTIFY=log ${SP_ENV} \
  nohup $W/.build/release/Wormhole --background --appearance dark "$@" > $R/app.log 2>&1 &
echo $! > $R/app.pid
echo "started pid $(cat $R/app.pid) data=$D ctl=$R/ctl"
