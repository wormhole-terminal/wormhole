#!/bin/bash
# stop.sh <x>: quits that instance (by its own pid) and any wormhole-host of its data dir
X=$1; R=/tmp/s3p-$X
[ -f $R/app.pid ] && { pid=$(cat $R/app.pid); printf 'quit\n' > $R/ctl/cmd.tmp && mv $R/ctl/cmd.tmp $R/ctl/cmd; sleep 2; kill -0 $pid 2>/dev/null && kill $pid; }
echo stopped $X
