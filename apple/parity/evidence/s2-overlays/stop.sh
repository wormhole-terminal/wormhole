#!/bin/bash
# stop.sh <x>: quit only this lane's instance <x> (its pid), and its hosts
R=/tmp/so-$1
kill -0 $(cat $R/app.pid 2>/dev/null) 2>/dev/null || { echo not running; exit 0; }
printf 'quit\n' > $R/ctl/cmd.tmp && mv $R/ctl/cmd.tmp $R/ctl/cmd; sleep 2
kill $(cat $R/app.pid) 2>/dev/null; echo stopped $1
