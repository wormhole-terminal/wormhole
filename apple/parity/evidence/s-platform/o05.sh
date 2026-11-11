#!/bin/bash
# o05.sh <x>: the notice chip's causes and PL03's banner path, live, --background (banners to the log, questions held)
X=${1:-o05}; H=$(dirname "$0"); EV=/tmp/sp-$X/o05.txt; export EV; : > "$EV"
w() { "$H/wh.sh" "$X" "$@" | cut -c1-400; }
c() { "$H/c.sh" "$X" "$@" | cut -c1-500; }
echo "# live run, --background, data /tmp/Wormhole-swift-s-platform-$X, marking shell, agent_access open, automation on, notices on" >> "$EV"
echo "## a script or agent driving a terminal says so on the chip (Driven)"
w send-text --terminal 1 --submit "echo typed-by-a-command-line"
c "notice chip"
w read --terminal 1 --last 2 >/dev/null
c "notice chip" "window-shot /tmp/sp-$X/chip-driven.png"
echo "## the warnings and failures the chip can say"
for what in "update-ready 9.9.9" "data-folder permission denied" "host-fallback no such file: wormhole-host (os error 2)" "move" "history-kept" "copy-path" "typing Codex CLI" "reading" ; do
  c "notice $what" "notice chip"
done
c "notice host-fallback boom" "window-shot /tmp/sp-$X/chip-host-fallback.png"
echo "## PL03: a waiting agent's banner, its category and level, and a reply typed from it"
w new-terminal --folder /tmp
c "hook 1 claude UserPromptSubmit" "hook 1 claude Notification Needs permission to run npm test" "notice-log"
ID=$(grep -o '^wormhole/tab/[0-9]*' /tmp/sp-$X/ctl/out.txt | head -1)
c "notice-respond $ID reply echo replied-via-the-banner"
sleep 1; w read --terminal 1 --last 3
c "notice-log" "hook 1 claude Stop" "notice-log" "notice-respond wormhole/tab/1 open" "notice-respond wormhole/tab/1 default"
echo "## UpdateReady / DataFolder are posted by their causes elsewhere (A03, profile switch); here only the chip"
