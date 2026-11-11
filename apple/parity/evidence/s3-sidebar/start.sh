#!/bin/bash
# start.sh <x> [app options...]: a Swift instance for lane S3-sidebar: --background, scratch data /tmp/Wormhole-swift-s3-sidebar-<x>, ctl in /tmp/s3sb-<x>/ctl
X=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s3-sidebar/apple/swift
D=/tmp/Wormhole-swift-s3-sidebar-$X; R=/tmp/s3sb-$X
mkdir -p $R/ctl $R/repo $D
[ -d $R/repo/.git ] || (cd $R/repo && git init -q && echo hi > a.txt && git add a.txt && git -c user.name=t -c user.email=t@t commit -qm init)
[ -n "$SP_CONFIG" ] && [ ! -e $D/config.json ] && printf '%s' "$SP_CONFIG" > $D/config.json
cd $R/repo
# SP_SHELL=none: the program comes from --shell, not WORMHOLE_SHELL, which core lets win over a terminal's own shell choice
SHELLENV="WORMHOLE_SHELL=${SP_SHELL:-/bin/zsh}"; SHELLARG=""; [ "$SP_SHELL" = none ] && { SHELLENV=""; SHELLARG="--shell ${SP_PROG:-/bin/zsh}"; }
env WORMHOLE_DATA_DIR=$D $SHELLENV WORMHOLE_DEVCTL=$R/ctl WORMHOLE_BACKGROUND=1 WORMHOLE_AUTH=${SP_AUTH:-cancel} WORMHOLE_NOTIFY=log ${SP_ENV} \
  nohup $W/.build/release/Wormhole --background $SHELLARG --appearance ${SP_APPEARANCE:-dark} "$@" > $R/app.log 2>&1 &
echo $! > $R/app.pid
echo "started pid $(cat $R/app.pid) data=$D ctl=$R/ctl"
