#!/usr/bin/env bash
# One command for the Swift port's static checks, no builds: rules, layers, gallery, parity data, size, budgets.
#   apple/scripts/quality.sh            # table; exit 1 when rules, layers or parity --check fail
#   apple/scripts/quality.sh --strict   # also fail on budgets above standards-allow.json
#   apple/scripts/quality.sh --guards   # also run the frame's work guards (perf-guards.sh: builds and runs TerminalCoreTests)
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
py() { python3 -I "$here/$1" "${@:2}"; }
status=0
row() { printf '%-10s %-5s %s\n' "$1" "$2" "$3"; }

rules=$(py rules.py); rc=$?; [ $rc -ne 0 ] && status=1
row rules "$([ $rc -eq 0 ] && echo ok || echo FAIL)" "$(echo "$rules" | tail -1)"
layers=$(py layers.py); rc=$?; [ $rc -ne 0 ] && status=1
row layers "$([ $rc -eq 0 ] && echo ok || echo FAIL)" "$(echo "$layers" | tail -1)"
gallery=$(py gallery.py); rc=$?; [ $rc -ne 0 ] && status=1
row gallery "$([ $rc -eq 0 ] && echo ok || echo FAIL)" "$(echo "$gallery" | tail -1)"
parity=$(py parity.py --check); rc=$?; [ $rc -ne 0 ] && status=1
row parity "$([ $rc -eq 0 ] && echo ok || echo FAIL)" "$(echo "$parity" | tail -1)"
row size - "$(py parity.py --size | grep -E '^total|against' | head -2 | tr -s ' ' | paste -sd' ' -)"
guards=0; args=()
for a in "$@"; do [ "$a" = --guards ] && guards=1 || args+=("$a"); done
budgets=$(py standards.py ${args[@]+"${args[@]}"}); rc=$?; [ $rc -ne 0 ] && status=1
row budgets "$([ $rc -eq 0 ] && echo ok || echo FAIL)" "$(echo "$budgets" | tail -1 | sed 's/^standards: //')"
if [ $guards -eq 1 ]; then
  perf=$("$here/perf-guards.sh"); rc=$?; [ $rc -ne 0 ] && status=1
  row guards "$([ $rc -eq 0 ] && echo ok || echo FAIL)" "$(echo "$perf" | tail -1 | sed 's/^perf-guards: //')"
fi
if [ $status -ne 0 ]; then
  echo; { echo "$rules"; echo "$layers"; echo "$gallery"; echo "$parity"; echo "$budgets"; echo "${perf:-}"; } | grep -vE '^(apple/ rules: ok|layers:|ok:|standards:)' | head -20
fi
exit $status
