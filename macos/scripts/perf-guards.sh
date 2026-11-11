#!/usr/bin/env bash
# The Swift terminal's work guards, like GPUI's counting tests (repo-docs/docs/performance.md): `TerminalCoreTests` holds
# the renderer's counters (only changed rows rebuilt, words shaped once, one wake per flood) and `FrameWorkGuardTests`
# the main-thread allocations a frame may make. It BUILDS the debug test product (a heavy build, a while the first time):
# where builds are rationed, set BUILD_SLOT to the command that holds a slot (it runs `$BUILD_SLOT swift test ...`).
#   macos/scripts/perf-guards.sh        # exit 1 when a guard fails; the last line says how many ran
set -uo pipefail
cd "$(dirname "$0")/../swift" || exit 1
out=$(${BUILD_SLOT:-} swift test --filter TerminalCoreTests 2>&1); rc=$?
summary=$(echo "$out" | grep -E "Test run with|error:" | tail -1)
[ $rc -ne 0 ] && echo "$out" | grep -E "recorded an issue|allocated|error:" | head -20
echo "perf-guards: ${summary:-no test summary (rc=$rc)}"
exit $rc
