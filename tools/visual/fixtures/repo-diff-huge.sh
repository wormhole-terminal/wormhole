#!/usr/bin/env bash
# The size rules: a generated lock file rewritten from end to end (a patch over 8 MB, so
# the card asks before loading it) and a long list with 24 000 changed lines (it loads, but over
# 20 000 lines without highlighting).
#   1 data/rates.json      24 000 patch lines, 0.5 MB
#   2 package-lock.json    +60 000 −60 000, about 12 MB of patch
#   tools/visual/fixtures/repo-diff-huge.sh <directory to create>
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

lock() {
  awk -v v="$1" 'BEGIN {
    print "{"
    for (i = 1; i <= 59998; i++) {
      printf "  \"node_modules/package-%05d\": { \"version\": \"%s.%d.0\", \"resolved\": \"https://registry.npmjs.org/p/-/p-%d.tgz\" },\n", i, v, i % 97, i
    }
    print "}"
  }' > "$repo/package-lock.json"
}

rates() {
  awk -v bump="$1" 'BEGIN {
    print "["
    for (i = 1; i <= 11998; i++) printf "  { \"id\": %d, \"rate\": %.1f },\n", i, (i * 37) % 1000 / 10.0 + bump
    print "]"
  }' > "$repo/data/rates.json"
}

mkdir -p "$repo/data"
lock 1
rates 0
commit 3 "Lock the dependencies"

lock 2
rates 0.5
