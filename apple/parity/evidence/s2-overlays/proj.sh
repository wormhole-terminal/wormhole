#!/bin/bash
# proj.sh <dir>: a throwaway git repo with a .wormhole/ folder: one config.json (nothing set) and two actions (a keyed run, an open)
D=$1; mkdir -p $D/.wormhole && cd $D
[ -d .git ] || { git init -q; echo hi > a.txt; git add a.txt; git -c user.name=t -c user.email=t@t commit -qm init; }
echo '{}' > .wormhole/config.json
cat > .wormhole/actions.json <<'JSON'
[
  { "id": "test", "title": "Run tests", "key": "ctrl-cmd-t", "type": "run", "command": "echo hello-from-action", "target": "new_session" },
  { "id": "docs", "title": "Open docs", "type": "open", "url": "https://example.com" }
]
JSON
