#!/bin/bash
# x08.sh: every verb of the control line against a --background Swift instance, with the real command line.
# Run: SP_CONFIG='{"agent_access":"open","control_write":true,"feature_flags":{"automation":true}}' ./start.sh x08; ./x08.sh x08
X=${1:-x08}; H=$(dirname "$0"); EV=${EV:-/tmp/sp-$X/x08.txt}; export EV; : > "$EV"
w() { "$H/wh.sh" "$X" "$@" | cut -c1-600; }
c() { "$H/c.sh" "$X" "$@" | cut -c1-600; }
echo "# live run, --background Swift instance (WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-s-platform-$X), the real command line (the app's own executable), agent_access=open, automation on" >> "$EV"
W=/Users/benjamin/Projects/terminal/wormhole/.claude/worktrees/swift-s-platform/apple/swift/.build/release/Wormhole
w list; w status; w doctor | head -8
T=1
w status --terminal $T
w notify --terminal $T --title "Deploy" "finished in 3s"
c notice-log
w send-text --terminal $T --submit "echo hello-from-send-text"
sleep 1; c screen | tail -4
printf 'echo from-stdin' | w send-text --terminal $T --submit -
w new-terminal --folder /tmp --command "echo opened-by-command"
sleep 2; w list
w read --terminal $T --last 6
w send-keys --terminal $T --type 'echo keys-typed' enter
sleep 1; w read --terminal $T --last 3
w mouse --terminal $T move 2 3
w new-window --folder /tmp
NW=$(grep -o 'with terminal [0-9]*' "$EV" | tail -1 | grep -o '[0-9]*$')
sleep 2; w list
w split --terminal $NW --left --select
w split --terminal $NW --up
w split --terminal $T --right --select
w split --terminal $T --down
sleep 1; w list
w focus --terminal $T
w rename --terminal $T "renamed by wh"
w list | head -4
w wait --terminal $T --match 'keys-typed' --timeout 5
w run --terminal $T --exit-code 'echo run-ok'
w run --terminal $T --exit-code 'false'
w blocks --terminal $T --last 3 --output
w wait --terminal $T --match 'never-appears-xyz' --timeout 1
w sessions list
w help --json | head -c 300; echo
echo "--- events (the stream while a terminal is typed into; each line is one event)"
EVO=/tmp/sp-$X/events.out; : > $EVO
( env WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-s-platform-$X $W events --topics status,terminals,output --terminal $T --json > $EVO 2>&1 & EP=$!; sleep 1.5; env WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-s-platform-$X $W send-text --terminal $T --submit "echo streamed" >/dev/null; sleep 2.5; kill $EP 2>/dev/null )
cut -c1-260 $EVO | head -10 | tee -a "$EV"
echo "--- mcp (initialize, tools/list names, tools/call list_terminals and read_terminal)"
M=/tmp/sp-$X/mcp.out
{ printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"s","version":"1"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized"}' '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_terminals","arguments":{}}}' '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_terminal","arguments":{"terminal":1,"last_lines":3}}}'; sleep 3; } | env WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-s-platform-$X $W mcp > $M 2>&1
python3 - "$M" "$EV" <<'PY'
import json,sys
out=[]
for line in open(sys.argv[1]):
    try: m=json.loads(line)
    except Exception: out.append(line.strip()[:200]); continue
    r=m.get("result") or m.get("error")
    if m.get("id")==2: out.append("tools/list -> "+", ".join(t["name"] for t in r["tools"]))
    elif m.get("id")==1: out.append("initialize -> "+r["serverInfo"]["name"]+" "+r["protocolVersion"])
    else: out.append(f'tools/call id {m.get("id")} -> '+json.dumps(r)[:260])
print("\n".join(out)); open(sys.argv[2],"a").write("\n".join(out)+"\n")
PY
echo "--- close asks even for an idle terminal (the question is a sheet; it never closes on a script's say)"
w close --terminal $T
c windows question
c "question cancel"
w list | head -3
w close --terminal 2
c "question confirm"
sleep 1; w list | grep -c "  terminal 2 " | sed 's/^/terminal 2 left after Close: /' | tee -a "$EV"
c "quit-ask"
