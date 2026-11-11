#!/bin/bash
# a03-fixture.sh <dir>: a throwaway minisign key, a signed fake release (99.0.0, whose executable leaves a marker when launched), a local
# feed served on 127.0.0.1 and a scratch .app at 0.0.1 inside the instance's data folder. Never the app's key or an installed app.
set -e
F=$1; DATA=$2; ID=app.getwormhole.terminal.swift
rm -rf "$F"; mkdir -p "$F/feed/stable" "$F/feed/releases" "$F/new" "$DATA/Fixture/Wormhole.app/Contents/MacOS"
minisign -G -W -f -p "$F/key.pub" -s "$F/key.sec" >/dev/null
plist() { printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>%s</string><key>CFBundleShortVersionString</key><string>%s</string><key>CFBundleExecutable</key><string>Wormhole</string></dict></plist>\n' "$ID" "$1"; }
# the scratch bundle the install replaces
mkdir -p "$DATA/Fixture/Wormhole.app/Contents"; plist 0.0.1 > "$DATA/Fixture/Wormhole.app/Contents/Info.plist"
printf '#!/bin/sh\necho old\n' > "$DATA/Fixture/Wormhole.app/Contents/MacOS/Wormhole"; chmod 755 "$DATA/Fixture/Wormhole.app/Contents/MacOS/Wormhole"
# the release: the same shape, its executable leaves a marker (what a restart opens)
mkdir -p "$F/new/Wormhole.app/Contents/MacOS"; plist 99.0.0 > "$F/new/Wormhole.app/Contents/Info.plist"
printf '#!/bin/sh\ndate > "%s/relaunched.marker"\n' "$F" > "$F/new/Wormhole.app/Contents/MacOS/Wormhole"; chmod 755 "$F/new/Wormhole.app/Contents/MacOS/Wormhole"
FILE=Wormhole-99.0.0-macos.app.tar.gz
COPYFILE_DISABLE=1 tar czf "$F/feed/releases/$FILE" -C "$F/new" Wormhole.app
minisign -S -s "$F/key.sec" -m "$F/feed/releases/$FILE" -t "Wormhole 99.0.0 $FILE" -x "$F/sig.minisig" >/dev/null
PORT=$((20000 + RANDOM % 20000))
python3 -I - "$F" "$PORT" "$FILE" <<'PY'
import json,sys,hashlib,os,platform
f,port,file=sys.argv[1],sys.argv[2],sys.argv[3]
body=open(f"{f}/feed/releases/{file}","rb").read()
arch="aarch64" if platform.machine()=="arm64" else platform.machine()
json.dump({"version":"99.0.0","pub_date":"2026-11-11T12:00:00Z","notes":"## Fixture\n- a signed release from a throwaway key","platforms":{f"darwin-{arch}":{"url":f"http://127.0.0.1:{port}/releases/{file}","sha256":hashlib.sha256(body).hexdigest(),"signature":open(f"{f}/sig.minisig").read(),"size":len(body)}}},open(f"{f}/feed/stable/latest.json","w"))
open(f"{f}/port","w").write(port)
PY
( cd "$F/feed" && nohup python3 -I -m http.server "$PORT" --bind 127.0.0.1 >"$F/feed.log" 2>&1 & echo $! > "$F/feed.pid" )
echo "feed http://127.0.0.1:$PORT key=$F/key.pub bundle=$DATA/Fixture/Wormhole.app"
