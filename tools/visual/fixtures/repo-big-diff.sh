#!/usr/bin/env bash
# One generated Rust file of 3000 lines with every third line changed in the working tree: a
# long, highlighted diff (well under the 20 000 lines past which highlighting stops).
#   tools/visual/fixtures/repo-big-diff.sh <directory to create>
# shellcheck source=lib.sh
. "$(dirname "$0")/lib.sh"

table() {
  awk -v changed="$1" 'BEGIN {
    print "/// Generated table of burn rates."
    print "pub const RATES: &[(u32, f64)] = &["
    for (i = 1; i <= 2996; i++) {
      rate = (i * 37) % 1000 / 10.0
      if (changed && i % 3 == 0) rate = rate + 0.5
      printf "    (%d, %.1f),\n", i, rate
    }
    print "];"
  }' > "$repo/src/rates.rs"
}

write README.md <<'EOF'
# rates

A table of burn rates.
EOF
mkdir -p "$repo/src"
table 0
commit 3 "Add the rates table"

table 1
