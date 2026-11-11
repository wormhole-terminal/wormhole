#!/usr/bin/env python3
"""The spike's comparison table: the Swift terminal against the GPUI app, from the result files of
`scripts/perf.sh` (GPUI) and `macos/scripts/perf-compare.sh` (Swift), with the pass line of gate G1.

    python3 -I macos/scripts/perf_table.py --gpui <perf.sh result.json> --swift <perf-compare dir> \
        [--baseline tools/bench/baselines/<machine>.json]

Pass line (plan 5.3): frame CPU at most 1.3 x GPUI on both animations, throughput at least 0.9 x,
idle with no timer wakeups (the idle part is read from the sample and `top` output by eye: it prints
what it found, it does not decide).
"""

from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from pathlib import Path

FRAME_FACTOR = 1.3
THROUGHPUT_FLOOR = 0.9
ANIMATIONS = ["warptunnel", "matrix"]
FLOODS = ["flood-ascii", "flood-truecolor"]
TESTS = ["logflood", "colorflood", "cursorstorm"]
EXTRA = ("encode_avg", "total_avg", "total_max", "cpu_avg", "gpu_avg", "wait_avg")


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def swift_extras(frames_txt: Path) -> dict[str, float]:
    """The median over the runs of each Swift-only field of the `frames` lines."""
    found: dict[str, list[float]] = {name: [] for name in EXTRA}
    for line in frames_txt.read_text(encoding="utf-8").splitlines():
        for name in EXTRA:
            match = re.search(rf"\b{name}=([\d.]+)ms", line)
            if match:
                found[name].append(float(match.group(1)))
    return {name: statistics.median(values) for name, values in found.items() if values}


def ratio(swift: float, gpui: float) -> str:
    return f"{swift / gpui:.2f}x" if gpui else "n/a"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gpui", type=Path, required=True)
    parser.add_argument("--swift", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    args = parser.parse_args()

    gpui = load(args.gpui)
    base = load(args.baseline) if args.baseline else None
    verdicts: list[tuple[str, bool]] = []

    print("| workload | GPUI now (draw avg ms) | GPUI baseline | Swift build avg | build / GPUI | Swift build+encode avg | total / GPUI | Swift CPU time | Swift GPU time | busy GPUI / Swift (%) |")
    print("|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|")
    for name in ANIMATIONS + FLOODS:
        swift_file = args.swift / f"{name}.json"
        if not swift_file.exists() or name not in gpui.get("workloads", {}):
            continue
        s, g = load(swift_file), gpui["workloads"][name]
        extras = swift_extras(args.swift / f"{name}.frames.txt")
        b = base["workloads"][name]["draw_avg_ms"] if base and name in base.get("workloads", {}) else None
        total = extras.get("total_avg")
        print(
            f"| {name} | {g['draw_avg_ms']:.2f} | {b if b is not None else '-'} | {s['draw_avg_ms']:.2f} "
            f"| {ratio(s['draw_avg_ms'], g['draw_avg_ms'])} | {total if total is not None else '-'} "
            f"| {ratio(total, g['draw_avg_ms']) if total is not None else '-'} "
            f"| {extras.get('cpu_avg', '-')} | {extras.get('gpu_avg', '-')} "
            f"| {g['main_busy_pct']:.0f} / {s['main_busy_pct']:.0f} |"
        )
        if name in ANIMATIONS:
            # The pass line is stated on the build part, which is what GPUI's `draw` measures, and
            # shown on the harsher total as well.
            verdicts.append((f"{name}: build {s['draw_avg_ms']:.2f} ms <= {FRAME_FACTOR} x GPUI {g['draw_avg_ms']:.2f} ms",
                             s["draw_avg_ms"] <= FRAME_FACTOR * g["draw_avg_ms"]))
            if total is not None:
                verdicts.append((f"{name}: build+encode {total:.2f} ms <= {FRAME_FACTOR} x GPUI (harsher)",
                                 total <= FRAME_FACTOR * g["draw_avg_ms"]))

    throughput_swift = args.swift / "throughput.json"
    if throughput_swift.exists() and "throughput" in gpui:
        print()
        print("| throughput (MB/s) | GPUI now | GPUI baseline | Swift | Swift / GPUI |")
        print("|---|--:|--:|--:|--:|")
        s_tests = load(throughput_swift)["tests"]
        g_tests = gpui["throughput"]["tests"]
        for test in TESTS:
            if test not in s_tests or test not in g_tests:
                continue
            b = base["throughput"]["tests"][test]["mbps"] if base and "throughput" in base else None
            r = s_tests[test]["mbps"] / g_tests[test]["mbps"]
            print(f"| {test} | {g_tests[test]['mbps']} | {b if b is not None else '-'} | {s_tests[test]['mbps']} | {r:.2f}x |")
            verdicts.append((f"{test}: Swift {s_tests[test]['mbps']} MB/s >= {THROUGHPUT_FLOOR} x GPUI {g_tests[test]['mbps']}",
                             r >= THROUGHPUT_FLOOR))

    print()
    for text, ok in verdicts:
        print(f"{'PASS' if ok else 'FAIL'}  {text}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
