#!/bin/bash
# fixture-e.sh: instance <x> (default e) with three agent-named programs (a copy of cat named claude), a trusted project, and hooks:
# terminal 1 needs input (asks to run npm test), 2 works, 3 idle in the project. Run from this folder.
X=${1:-e}; H=$(cd "$(dirname "$0")" && pwd)
A=/private/tmp/claude-501/-Users-benjamin-Projects-terminal/e0b38e0a-a3f5-44c8-b6a2-924540bddb46/scratchpad/agentbin/claude
"$H/proj.sh" /tmp/so-b/proj
"$H/stop.sh" $X >/dev/null 2>&1; sleep 2; rm -rf /tmp/Wormhole-swift-s2-overlays-$X
SP_SHELL=$A SP_AUTH=allow SP_CONFIG='{"agent_access":"open","control_write":true,"feature_flags":{"automation":true,"project_folders":true,"folder_picker":true,"unlocked":true}}' "$H/start.sh" $X; sleep 5
"$H/c.sh" $X "onboarding close" "new" "ambient-run open-project /tmp/so-b/proj" "wait-idle 1500" "project-approve" "wait-idle 1500" "window-state off on" \
  "hook 1 claude UserPromptSubmit" "hook 1 claude Notification:permission_prompt Needs permission to run npm test" "hook 2 claude UserPromptSubmit" "wait-idle 500" "counts" | tail -1
