#!/bin/bash
# stop.sh <x>: quit only our own instance (its pid) through its control folder, then make sure its hosts are gone
X=$1; R=/tmp/sp-$X
printf 'quit\n' > $R/ctl/cmd; sleep 2
kill -0 $(cat $R/app.pid) 2>/dev/null && kill $(cat $R/app.pid)
echo stopped
