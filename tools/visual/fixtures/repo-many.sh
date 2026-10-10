#!/usr/bin/env bash
# "1,284 changed files": 182 tracked files changed and 1,102 untracked, 1,090 of them in a
# build folder nobody ignored, so the Untracked section starts folded.
#   tools/visual/fixtures/repo-many.sh <directory to create>
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

folders=(crates/app/src crates/api/src crates/api/src/models crates/core/src crates/ui/src crates/cli/src)
for i in $(seq 1 182); do
  folder=${folders[$(( i % ${#folders[@]} ))]}
  mkdir -p "$repo/$folder"
  printf 'pub fn f%d() {}\n' "$i" > "$repo/$folder/file$i.rs"
done
commit 5 "Start the workspace"

for i in $(seq 1 182); do
  folder=${folders[$(( i % ${#folders[@]} ))]}
  printf 'pub fn f%d() {}\npub fn g%d() {}\n' "$i" "$i" > "$repo/$folder/file$i.rs"
done
mkdir -p "$repo/target/debug/deps"
for i in $(seq 1 1090); do
  printf 'artifact %d\n' "$i" > "$repo/target/debug/deps/unit$i.d"
done
mkdir -p "$repo/scratch"
for i in $(seq 1 12); do
  printf 'note %d\n' "$i" > "$repo/scratch/note$i.txt"
done
