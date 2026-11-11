#!/bin/bash
# a03.sh: the updater end to end in a scratch dir: check, download, sha256 + minisign check, bundle swap, the chip, the question, restart.
X=a03; H=$(dirname "$0"); EV=/tmp/sp-$X/a03.txt; export EV; mkdir -p /tmp/sp-$X; : > "$EV"
F=/tmp/sp-$X/fx; D=/tmp/Wormhole-swift-s-platform-$X
c() { "$H/c.sh" $X "$@" | cut -c1-400; }
echo "# the fixture: $(cat $F/port 2>/dev/null) feed, throwaway key, scratch bundle inside the data folder (never an installed app)" >> "$EV"
c "update" "update check" "updates"
sleep 2; c "update"
echo "--- bundle before: $(cat $D/Fixture/Wormhole.app/Contents/MacOS/Wormhole | tail -1) / $(grep -A1 ShortVersion $D/Fixture/Wormhole.app/Contents/Info.plist | grep -o '[0-9.]*</string>' )" | tee -a "$EV"
c "update install"
sleep 4; c "update" "notice chip" "window-shot /tmp/sp-$X/chip-update-ready.png"
echo "--- bundle after: $(tail -1 $D/Fixture/Wormhole.app/Contents/MacOS/Wormhole) / $(grep -o '99.0.0' $D/Fixture/Wormhole.app/Contents/Info.plist)" | tee -a "$EV"
ls -A "$D/Fixture" | tee -a "$EV"
echo "--- a click on the chip while a command runs: the question first (Cancel leaves everything), then Restart When Idle"
c 'type sleep 9\r' "update restart" "question" "question cancel" "update restart"
c "question also"
echo "--- the app waits for the terminals to be idle for 30 s, then restarts into the new bundle (the marker is what the new bundle's executable writes)"
sleep 52
ls $F | tr '\n' ' ' | tee -a "$EV"; echo
[ -f $F/relaunched.marker ] && echo "relaunched: $(cat $F/relaunched.marker)" | tee -a "$EV"
kill -0 $(cat /tmp/sp-$X/app.pid) 2>/dev/null && echo "the app is still running" | tee -a "$EV" || echo "the app quit for the restart" | tee -a "$EV"
