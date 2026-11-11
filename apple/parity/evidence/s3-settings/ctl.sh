#!/bin/zsh
# usage: ctl.sh <x> <line> [<line>...]  -> writes lines, waits, prints new output
x=$1; shift
C=/tmp/hd-s3-settings-$x
before=$(wc -l < $C/out.txt 2>/dev/null || echo 0); before=${before// /}
printf '%s\n' "$@" > $C/cmd.tmp && mv $C/cmd.tmp $C/cmd
sleep ${WAIT:-2}
tail -n +$((before+1)) $C/out.txt
