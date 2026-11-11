#!/bin/bash
# pl03.sh: PL03's banner kinds and rules through the log seam (the banners as the notification centre would be handed them).
# Needs an agent program named `claude`: mkdir /tmp/s3p-agent; cp /bin/cat /tmp/s3p-agent/claude; codesign --force -s - /tmp/s3p-agent/claude
# and SP_PROG=/tmp/s3p-agent/claude, SP_CONFIG={"agent_access":"open","control_write":true,"feature_flags":{...}} (see start.sh).
# What is announced for plain terminals (a failed or long command, the bell, a program's own notification words) is the engine lane's.
X=${1:-e}; H=$(cd "$(dirname "$0")" && pwd); EV=/tmp/s3p-$X/pl03.txt; export EV WAIT=2.5; : > "$EV"
w() { "$H/wh.sh" "$X" "$@" | cut -c1-300; }
c() { "$H/c.sh" "$X" "$@" | grep -v "^hook\|^set \|^select"; }
sec() { echo "## $*" | tee -a "$EV"; }
for i in 1 2 3 4 5; do w new-terminal --folder /tmp > /dev/null; done; c "select 6" > /dev/null
sec "needs input: banner, Needs-you category, Active level (no Time Sensitive entitlement in a test instance), sound, thread per terminal"
c "hook 1 claude UserPromptSubmit" "hook 1 claude Notification Needs permission to run npm test" "notice-log"
sec "a newer message about the same wait is the same banner (core posts on the change of state, not of words)"
c "hook 1 claude Notification Approve cargo build?" "notice-log"
sec "a response to someone else's banner changes nothing; show-changes brings that terminal and the Changes tab; open brings it"
c "notice-respond wormhole/tab/99 reply nothing-here" "notice-log" "notice-respond wormhole/tab/1 show-changes" "state"
sec "reply typed from the banner goes into that terminal and takes the banner down"
c "select 6" "notice-respond wormhole/tab/1 reply typed-from-the-banner" "notice-log"
w read --terminal 1 --last 2
sec "agent finished: its own banner (Finished category, no sound)"
c "hook 1 claude UserPromptSubmit" "hook 1 claude Stop" "notice-log"
sec "looking at the terminal takes its banner down"
c "select 1" "notice-log" "select 6"
sec "Needs input off: no banner; Agent done off: no finished banner"
c "set notify_needs_you=false" "set notify_agent_done=false" "hook 2 claude UserPromptSubmit" "hook 2 claude Notification Needs you again" "notice-log" "hook 2 claude Stop" "notice-log" "set notify_needs_you=true" "set notify_agent_done=true"
sleep 12
sec "sound setting none: the banner has no sound"
c "set notify_sound=\"none\"" "hook 3 claude UserPromptSubmit" "hook 3 claude Notification Quiet" "notice-log" "set notify_sound=\"needs_you\""
sleep 12
sec "a burst: three agents finishing within ten seconds become one summary naming them and each loses its own banner (a wait is never grouped)"
C=/tmp/s3p-$X/ctl; : > $C/out.txt
printf 'hook 4 claude UserPromptSubmit\nhook 5 claude UserPromptSubmit\nhook 6 claude UserPromptSubmit\nhook 4 claude Stop\nhook 5 claude Stop\nhook 6 claude Stop\n' > $C/cmd.tmp; mv $C/cmd.tmp $C/cmd; sleep 9
c "notice-log"
sec "group_bursts off: each finished agent keeps its own banner"
c "set notify_group_bursts=false" "select 6"
printf 'hook 2 claude UserPromptSubmit\nhook 3 claude UserPromptSubmit\nhook 4 claude UserPromptSubmit\nhook 2 claude Stop\nhook 3 claude Stop\nhook 4 claude Stop\n' > $C/cmd.tmp; mv $C/cmd.tmp $C/cmd; sleep 9
c "notice-log"
sec "Send test (Settings): the banner needs input would be"
c "notice-test" "notice-log"
