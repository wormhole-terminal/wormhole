#!/bin/bash
# start.sh <x> [app options...]: a Swift instance for lane S3-platform: --background, scratch data /tmp/Wormhole-swift-s3-platform-<x>, ctl in /tmp/s3p-<x>/ctl
# SP_PROG=<program> runs that program as the shell (--shell), SP_CONFIG seeds config.json, SP_ENV adds environment (WORMHOLE_LANGUAGE=en-XA ...)
X=$1; shift
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s3-platform/apple/swift
D=/tmp/Wormhole-swift-s3-platform-$X; R=/tmp/s3p-$X
mkdir -p $R/ctl $R/repo $D
[ -d $R/repo/.git ] || (cd $R/repo && git init -q && echo hi > a.txt && git add a.txt && git -c user.name=t -c user.email=t@t commit -qm init)
[ -n "$SP_CONFIG" ] && [ ! -e $D/config.json ] && printf '%s' "$SP_CONFIG" > $D/config.json
cd $R/repo
SHELLENV="WORMHOLE_SHELL=${SP_SHELL:-/bin/zsh}"; SHELLARG=""; [ -n "$SP_PROG" ] && { SHELLENV=""; SHELLARG="--shell $SP_PROG"; }
env WORMHOLE_DATA_DIR=$D $SHELLENV WORMHOLE_DEVCTL=$R/ctl WORMHOLE_BACKGROUND=1 WORMHOLE_AUTH=${SP_AUTH:-cancel} WORMHOLE_NOTIFY=log ${SP_ENV} \
  nohup $W/.build/release/Wormhole --background $SHELLARG --appearance ${SP_APPEARANCE:-dark} "$@" > $R/app.log 2>&1 &
echo $! > $R/app.pid
echo "started pid $(cat $R/app.pid) data=$D ctl=$R/ctl"
