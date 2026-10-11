#!/usr/bin/env python3
"""The parsing and comparing half of scripts/perf.sh (python3 standard library only; run with -I).

  perf_tools.py busy <sample file>
      main-thread busy share of a macOS `sample` report, as JSON
  perf_tools.py workload <name> <frames.txt> <sample file>
      one workload's numbers (median draw avg/max of the `frames` lines, main-thread busy), as JSON
  perf_tools.py throughput <ttystorm --save file>
      the medians in MB/s of a `ttystorm bench --save` file, as JSON
  perf_tools.py assemble <out> <key=value>... [--workload <name> <file>]... [--throughput <file>]
      the result file from the pieces above, plus the metadata given as key=value
  perf_tools.py display
      0 when the main display is awake, 1 when it sleeps (macOS)
  perf_tools.py report <result.json> [<baseline.json>] [--limits <draw_f> <draw_ms> <busy_f> <busy_pts> <mbps_f>]
      the table; exit 1 when something regressed against the baseline
"""

import ctypes
import json
import re
import statistics
import sys

# Leaf frames a thread sits in while it waits for something. A sample whose innermost frame is one
# of these is idle; every other sample is the thread working (or spinning).
IDLE = {
    "mach_msg2_trap",
    "mach_msg_trap",
    "mach_msg_overwrite_trap",
    "semaphore_wait_trap",
    "semaphore_timedwait_trap",
    "__psynch_cvwait",
    "__psynch_mutexwait",
    "__workq_kernreturn",
    "__semwait_signal",
    "kevent",
    "kevent64",
    "kevent_qos",
    "__select",
    "__ulock_wait",
    "__ulock_wait2",
    "poll",
    "read",
    "__read_nocancel",
    "__wait4",
    "_pthread_cond_wait",
    "pthread_cond_wait",
    "__recvmsg",
    "__accept",
    "nanosleep",
    "usleep",
    "swtch_pri",
    "thread_switch",
}

LINE = re.compile(r"^([ +!:|]*)(\d+) (.*)$")


def main_thread_busy(text):
    """(busy samples, total samples) of the main thread in a `sample` call graph."""
    lines = text.splitlines()
    start = next((i for i, l in enumerate(lines) if l.startswith("Call graph:")), None)
    if start is None:
        raise SystemExit("no call graph in the sample")
    # A thread is a line of exactly four spaces and a count; its frames are indented further.
    threads, current = [], None
    for l in lines[start + 1 :]:
        if l.startswith("Total number in stack") or l.startswith("Sort by top of stack"):
            break
        if re.match(r"^    \d+ Thread_", l):
            current = [l]
            threads.append(current)
        elif current is not None and l.strip():
            current.append(l)
    main = [t for t in threads if "com.apple.main-thread" in t[0]]
    if not main:
        # `sample` sometimes leaves the dispatch queue's name off; the main thread is then the one
        # whose stack starts at dyld's `start`.
        main = [t for t in threads if len(t) > 1 and re.search(r"\d+ start  \(in dyld\)", t[1])]
    if not main:
        raise SystemExit("no main thread in the sample")
    nodes = []  # (depth, count, name)
    for l in main[0][1:]:
        m = LINE.match(l)
        if not m:
            continue
        name = m.group(3).split("  (in ")[0].split(" + ")[0].strip()
        nodes.append((len(m.group(1)), int(m.group(2)), name))
    total = int(re.match(r"^    (\d+) ", main[0][0]).group(1))
    # A node's own samples are its count minus its children's: those are the leaf samples.
    idle = 0
    for i, (depth, count, name) in enumerate(nodes):
        kids = 0
        for d2, c2, _ in nodes[i + 1 :]:
            if d2 <= depth:
                break
            if d2 == next_depth(nodes, i):
                kids += c2
        own = count - kids
        if own > 0 and name in IDLE:
            idle += own
    return total - idle, total


def next_depth(nodes, i):
    """The indentation of node i's direct children (the next node, when it is deeper)."""
    if i + 1 < len(nodes) and nodes[i + 1][0] > nodes[i][0]:
        return nodes[i + 1][0]
    return -1


def busy_json(path):
    busy, total = main_thread_busy(open(path, errors="replace").read())
    return {"main_busy_pct": round(100.0 * busy / max(total, 1), 1), "samples": total}


def frame_runs(path):
    runs = []
    for l in open(path):
        m = re.search(r"frames n=(\d+) avg=([\d.]+)ms max=([\d.]+)ms", l)
        if m:
            runs.append((float(m.group(2)), float(m.group(3))))
    return runs


def workload_json(name, frames_path, sample_path):
    runs = frame_runs(frames_path)
    if not runs:
        raise SystemExit(f"{name}: no frames lines in {frames_path}")
    out = {
        "draw_avg_ms": round(statistics.median(r[0] for r in runs), 2),
        "draw_max_ms": round(statistics.median(r[1] for r in runs), 2),
        "draw_avg_runs": [r[0] for r in runs],
        "draw_max_runs": [r[1] for r in runs],
    }
    out.update(busy_json(sample_path))
    return out


def throughput_json(path):
    data = json.load(open(path))
    tests = {}
    for t in data["tests"]:
        if t["unit"] == "MB/s":
            tests[t["name"]] = {"mbps": round(t["median"], 1), "min": round(t["min"], 1), "max": round(t["max"], 1)}
    return {"cols": data["cols"], "rows": data["rows"], "size_mb": data["size_mb"], "tests": tests}


def display_asleep():
    try:
        q = ctypes.CDLL("/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices")
        return int(bool(q.CGDisplayIsAsleep(q.CGMainDisplayID())))
    except Exception:
        return -1


def assemble(out_path, args):
    result = {"format": "wormhole-perf", "format_version": 1, "workloads": {}}
    i = 0
    while i < len(args):
        if args[i] == "--workload":
            result["workloads"][args[i + 1]] = json.load(open(args[i + 2]))
            i += 3
        elif args[i] == "--throughput":
            result["throughput"] = json.load(open(args[i + 1]))
            i += 2
        else:
            key, _, value = args[i].partition("=")
            result[key] = int(value) if re.fullmatch(r"-?\d+", value) else value
            i += 1
    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)
        f.write("\n")


def report(result_path, base_path, limits):
    draw_f, draw_ms, busy_f, busy_pts, mbps_f = limits
    res = json.load(open(result_path))
    base = json.load(open(base_path)) if base_path else None
    fails = []
    rows = []
    for name, w in res["workloads"].items():
        b = (base or {}).get("workloads", {}).get(name)
        verdict = "-"
        if b:
            notes = []
            if w["draw_avg_ms"] > b["draw_avg_ms"] * draw_f + draw_ms:
                notes.append(f"draw {w['draw_avg_ms']:.2f} > {b['draw_avg_ms'] * draw_f + draw_ms:.2f} ms")
            if w["main_busy_pct"] > b["main_busy_pct"] * busy_f + busy_pts:
                notes.append(f"busy {w['main_busy_pct']:.0f} > {b['main_busy_pct'] * busy_f + busy_pts:.0f} %")
            verdict = "REGRESSED: " + ", ".join(notes) if notes else "ok"
            if notes:
                fails.append(f"{name}: " + ", ".join(notes))
            verdict += f"  (base {b['draw_avg_ms']:.2f} ms, {b['main_busy_pct']:.0f} %)"
        rows.append((name, f"{w['draw_avg_ms']:.2f}", f"{w['draw_max_ms']:.2f}", f"{w['main_busy_pct']:.0f}", verdict))
    out = [f"{'workload':<16}{'draw avg ms':>12}{'draw max ms':>12}{'main busy %':>12}  vs baseline"]
    out += [f"{n:<16}{a:>12}{m:>12}{p:>12}  {v}" for n, a, m, p, v in rows]
    tp = res.get("throughput")
    if tp:
        out.append("")
        out.append(f"{'throughput':<16}{'MB/s':>12}{'min':>12}{'max':>12}  vs baseline   ({tp['cols']}x{tp['rows']} cells)")
        for name, t in tp["tests"].items():
            bt = ((base or {}).get("throughput") or {}).get("tests", {}).get(name)
            verdict = "-"
            if bt:
                floor = bt["mbps"] * mbps_f
                if t["mbps"] < floor:
                    verdict = f"REGRESSED: {t['mbps']:.1f} < {floor:.1f} MB/s"
                    fails.append(f"throughput {name}: {t['mbps']:.1f} MB/s < {floor:.1f}")
                else:
                    verdict = "ok"
                verdict += f"  (base {bt['mbps']:.1f})"
            out.append(f"{name:<16}{t['mbps']:>12.1f}{t['min']:>12.1f}{t['max']:>12.1f}  {verdict}")
    if base and base.get("display_asleep") != res.get("display_asleep"):
        out.append("")
        out.append("note: the display slept in one of the two runs (main busy % differs with it)")
    print("\n".join(out))
    if fails:
        print("\nREGRESSION:\n  " + "\n  ".join(fails))
        return 1
    return 0


def main(argv):
    if len(argv) < 2:
        raise SystemExit(__doc__)
    cmd = argv[1]
    if cmd == "busy":
        print(json.dumps(busy_json(argv[2])))
    elif cmd == "workload":
        print(json.dumps(workload_json(argv[2], argv[3], argv[4])))
    elif cmd == "throughput":
        print(json.dumps(throughput_json(argv[2])))
    elif cmd == "assemble":
        assemble(argv[2], argv[3:])
    elif cmd == "display":
        print(display_asleep())
    elif cmd == "report":
        limits = (1.3, 0.5, 1.3, 5.0, 0.75)
        args = argv[2:]
        if "--limits" in args:
            i = args.index("--limits")
            limits = tuple(float(x) for x in args[i + 1 : i + 6])
            args = args[:i]
        return report(args[0], args[1] if len(args) > 1 else None, limits)
    else:
        raise SystemExit(__doc__)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
