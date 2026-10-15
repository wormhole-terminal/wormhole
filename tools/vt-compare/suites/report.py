#!/usr/bin/env python3
"""Turn the results scripts/suite-compare.sh collected into markdown tables.

    report.py --out <results dir> [--ucs-detect <ucs-detect checkout>]

Reads, in <results dir>:
    meta.tsv                  key<TAB>value lines about the run (date, machine, pins, sizes)
    runs.tsv                  one line per suite and terminal: how the run went
    esctest/<terminal>.jsonl  esctest_run.py's results
    ucs-detect/<terminal>.yaml  ucs-detect's --save-yaml files
Writes esctest.md, esctest-tests.md, ucs-detect.md and run.md next to them. Run it with the suites'
Python (it needs PyYAML for ucs-detect's files).
"""
import argparse
import csv
import json
import os
import sys

TERMINAL_ORDER = ["wormhole", "ghostty", "alacritty", "kitty", "wezterm", "iterm2", "terminal"]
NAMES = {"wormhole": "Wormhole", "ghostty": "Ghostty", "alacritty": "Alacritty", "kitty": "kitty",
         "wezterm": "WezTerm", "iterm2": "iTerm2", "terminal": "Terminal"}

# esctest2 groups its tests by the sequence they test (one class each). These are coarser areas.
AREAS = {
    "Cursor movement": "CUU CUD CUF CUB CUP HVP CHA VPA VPR HPA HPR CNL CPL CHT CBT BS CR LF FF VT IND NEL RI",
    "Erasing and editing": "ED EL ECH ICH DCH IL DL REP SU SD DECSED DECSEL DECIC DECDC DECBI DECFI DECALN",
    "Rectangular operations": "DECCRA DECFRA DECERA DECSERA FillRectangle",
    "Margins": "DECSTBM",
    "Modes and resets": "SM RM DECSET DECSETTiteInhibit DECRQM DECSTR RIS DECSCL S8C1T",
    "Tabs": "HTS TBC",
    "Cursor save and restore": "DECRC SCORC SaveRestoreCursor XtermSave",
    "Reports": "DA DA2 DECID DECDSR DECRQSS",
    "Colors, titles, selection": "ChangeColor ChangeDynamicColor ChangeSpecialColor ResetColor ResetSpecialColor SGR SMTitle ManipulateSelectionData",
    "Strings (APC, DCS, PM, SOS)": "APC DCS PM SOS",
    "Window operations": "XtermWinops",
}
AREA_OF = {cls: area for area, names in AREAS.items() for cls in names.split()}


def area_of(cls):
    key = cls[:-5] if cls.endswith("Tests") else cls
    return AREA_OF.get(key, "Other")


def read_tsv(path):
    if not os.path.exists(path):
        return []
    with open(path, encoding="utf-8") as f:
        return [row for row in csv.reader(f, delimiter="\t") if row]


def table(header, rows):
    out = ["| " + " | ".join(header) + " |", "|" + "|".join("---" for _ in header) + "|"]
    for r in rows:
        out.append("| " + " | ".join(str(c) for c in r) + " |")
    return "\n".join(out)


def ordered(terminals):
    known = [t for t in TERMINAL_ORDER if t in terminals]
    return known + sorted(t for t in terminals if t not in TERMINAL_ORDER)


def pct(n, d):
    return "n/a" if not d else f"{100.0 * n / d:.1f}%"


# ---------------------------------------------------------------- run info

def load_runs(out):
    runs = {}
    for row in read_tsv(os.path.join(out, "runs.tsv")):
        row += [""] * (10 - len(row))
        suite, term, state, note, version, seen, front, load, code, secs = row[:10]
        runs[(suite, term)] = dict(state=state, note=note, version=version, seen=seen, front=front,
                                   load=load, code=code, seconds=secs)
    return runs


def load_meta(out):
    return {r[0]: r[1] for r in read_tsv(os.path.join(out, "meta.tsv")) if len(r) >= 2}


def header(title, meta, extra=()):
    lines = [f"# {title}", ""]
    for key, label in (("date", "date"), ("machine", "machine"), ("checkout", "checkout"),
                       ("asked_for", "asked for"), ("load", "load average at the start")):
        if meta.get(key):
            lines.append(f"- {label}: {meta[key]}")
    lines.extend(extra)
    return lines + [""]


def run_md(out, meta, runs):
    lines = header("Suite comparison: how each run went", meta)
    rows = []
    def order(kv):
        suite, term = kv[0]
        return (suite, TERMINAL_ORDER.index(term) if term in TERMINAL_ORDER else 99)
    for (suite, term), r in sorted(runs.items(), key=order):
        rows.append([suite, NAMES.get(term, term), r["state"] + (": " + r["note"] if r["note"] else ""),
                     r["version"], r["seen"], r["front"], r["load"], r["seconds"]])
    lines.append(table(["suite", "terminal", "result", "version", "rows x cols seen", "in front", "load (1 min)",
                        "seconds"], rows))
    lines += ["", "Apps drive one at a time, each with its user configuration left out (see scripts/terminal-compare.sh",
              "for what each is given). `skipped: already running` is iTerm2 or Terminal, which have one process",
              "and are never taken over.", ""]
    return "\n".join(lines)


# ---------------------------------------------------------------- esctest

def load_esctest(out):
    """terminal -> {probe, tests: {name: record}, summary, incomplete}"""
    result = {}
    folder = os.path.join(out, "esctest")
    if not os.path.isdir(folder):
        return result
    for fn in sorted(os.listdir(folder)):
        if not fn.endswith(".jsonl"):
            continue
        term = fn[:-6]
        probe, tests, summary = None, {}, None
        with open(os.path.join(folder, fn), encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except ValueError:
                    continue
                if rec.get("type") == "probe":
                    probe = rec
                elif rec.get("type") == "test":
                    tests[rec["name"]] = rec
                elif rec.get("type") == "summary":
                    summary = rec
        if probe is not None:
            result[term] = dict(probe=probe, tests=tests, summary=summary, incomplete=summary is None)
    return result


def yes(v):
    return "yes" if v else "no"


def esctest_md(out, meta, runs, data):
    lines = header("esctest2 in each terminal", meta, [
        f"- suite: esctest2 {meta.get('esctest_rev', '')[:12]} (Thomas Dickey, GPL-2.0), run by tools/vt-compare/suites/esctest_run.py",
        "- judged against the behaviour esctest2 describes (xterm without its bugs), not against xterm's known-bug"
        " annotations; \"empty\" and \"blank\" cells count as one",
        "- 80 x 25 cells, the size esctest2 resets to",
    ])
    if not data:
        lines.append("No esctest2 results.")
        return "\n".join(lines) + "\n"
    terms = ordered(data)

    # 1. what each terminal answers
    lines += ["## What esctest2 needs from the terminal", ""]
    rows = []
    for t in terms:
        p = data[t]["probe"]
        size = p.get("csi_18t")
        rows.append([NAMES.get(t, t), yes(p.get("cpr")), yes(p.get("da1")), yes(size),
                     yes(p.get("csi_14t")), yes(p.get("csi_16t")), yes(p.get("decrqcra")),
                     p.get("checksum") or "-"])
    lines.append(table(["terminal", "cursor position (DSR 6)", "DA1", "text area size (CSI 18 t)", "CSI 14 t",
                        "CSI 16 t", "checksum of a rectangle (DECRQCRA)", "checksum form"], rows))
    lines += ["", "DECRQCRA is how esctest2 reads the screen back, one cell at a time. A terminal that does not",
              "answer it cannot be asked what is on the screen: a test that gets as far as a screen check is",
              "`unmeasurable` for it and is left out of its pass count (tests that fail before that point are real",
              "failures). Without CSI 18 t the size is taken from the tty instead, and only the tests of CSI 18 t",
              "itself fail for it.", ""]

    # 2. totals
    all_names = sorted({n for t in terms for n in data[t]["tests"]})
    counts = {}
    for t in terms:
        c = dict(pass_=0, fail=0, unmeasurable=0, skip=0, blocked=0)
        for rec in data[t]["tests"].values():
            key = "pass_" if rec["status"] == "pass" else rec["status"]
            c[key] = c.get(key, 0) + 1
        counts[t] = c
    # the tests every terminal that could be measured at all was measured on
    usable = [t for t in terms if (counts[t]["pass_"] + counts[t]["fail"]) >= 0.2 * max(1, len(all_names))]
    common = [n for n in all_names if all(data[t]["tests"].get(n, {}).get("status") in ("pass", "fail")
                                          for t in usable)] if len(usable) >= 2 else []
    lines += ["## Result", ""]
    rows = []
    for t in terms:
        c = counts[t]
        measured = c["pass_"] + c["fail"]
        cs = [n for n in common if data[t]["tests"].get(n, {}).get("status") == "pass"]
        flag = ""
        if data[t]["incomplete"]:
            flag = " (run did not finish)"
        rows.append([NAMES.get(t, t) + flag, runs.get(("esctest", t), {}).get("version", ""), c["pass_"],
                     c["fail"], c["unmeasurable"], c["blocked"], c["skip"], measured, pct(c["pass_"], measured),
                     (f"{len(cs)}/{len(common)} ({pct(len(cs), len(common))})" if t in usable and common else "-")])
    lines.append(table(["terminal", "version", "pass", "fail", "unmeasurable", "blocked", "skipped", "measured",
                        "pass of measured", (f"pass on the {len(common)} tests every measurable terminal ran" if common else "pass on the common tests")], rows))
    lines += ["", "- `unmeasurable`: the test needed a screen check and the terminal does not answer DECRQCRA.",
              "- `blocked`: esctest2's own set-up (reset) failed, so the test did not run.",
              "- `skipped`: needs a VT level above the one asked for, or overwrites the clipboard.",
              "- \"pass of measured\" compares terminals that were measured on different tests; the last column",
              "  compares them on the same ones.", ""]

    # 3. by area
    areas = {}
    for n in all_names:
        cls = n.split(".")[0]
        areas.setdefault(area_of(cls), []).append(n)
    lines += ["## By area", "", "Cells are passes / tests measured in that area.", ""]
    rows = []
    for area in list(AREAS) + ["Other"]:
        names = areas.get(area)
        if not names:
            continue
        row = [f"{area} ({len(names)})"]
        for t in terms:
            recs = [data[t]["tests"].get(n) for n in names]
            m = [r for r in recs if r and r["status"] in ("pass", "fail")]
            p = [r for r in m if r["status"] == "pass"]
            row.append(f"{len(p)}/{len(m)}" if m else "-")
        rows.append(row)
    lines.append(table(["area (tests)"] + [NAMES.get(t, t) for t in terms], rows))
    lines.append("")

    # 4. why they fail: the commonest reasons
    lines += ["## Commonest failure reasons", ""]
    for t in terms:
        reasons = {}
        for rec in data[t]["tests"].values():
            if rec["status"] in ("fail", "blocked"):
                key = (rec.get("reason") or "")[:60]
                key = key.split(":")[0] if key.startswith(("ChecksumException", "TestFailure", "BadResponse")) else key
                reasons[key] = reasons.get(key, 0) + 1
        top = sorted(reasons.items(), key=lambda kv: -kv[1])[:4]
        if top:
            lines.append(f"- {NAMES.get(t, t)}: " + "; ".join(f"{k or '(none)'} x{v}" for k, v in top))
    lines += ["", "Every test, per terminal: esctest-tests.md. esctest2's own log and the traceback of each failed",
              "test are in esctest/<terminal>.log and .log.trace.", ""]
    return "\n".join(lines) + "\n", all_names, terms


def esctest_tests_md(meta, data, names, terms):
    symbol = {"pass": "pass", "fail": "FAIL", "unmeasurable": "-", "skip": ".", "blocked": "blocked"}
    lines = header("esctest2: every test", meta)
    lines.append("pass, FAIL, `-` unmeasurable (needs DECRQCRA), `.` skipped, blocked. `xterm bug` marks a test "
                 "esctest2 lists as a known bug of xterm itself.")
    lines.append("")
    rows = []
    for n in names:
        known = ""
        for t in terms:
            rec = data[t]["tests"].get(n)
            if rec and rec.get("xterm_known_bug"):
                known = " (xterm bug)"
                break
        rows.append([n + known] + [symbol.get((data[t]["tests"].get(n) or {}).get("status"), "?") for t in terms])
    lines.append(table(["test"] + [NAMES.get(t, t) for t in terms], rows))
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------- ucs-detect

UCS_CATEGORIES = [
    ("unicode_wide_results", "wide"),
    ("narrow_results", "narrow"),
    ("emoji_zwj_results", "ZWJ emoji"),
    ("emoji_vs16_results", "VS16"),
    ("emoji_vs15_results", "VS15"),
    ("ri_results", "flags (RI pairs)"),
    ("sri_results", "lone RI"),
    ("sfz_results", "lone skin tone"),
    ("language_results", "languages"),
]
# ucs-detect's data/<name>.yaml for the terminals that have published results.
PUBLISHED = {"wormhole": None, "ghostty": "ghostty", "alacritty": "alacritty", "kitty": "kitty",
             "wezterm": "wezterm", "iterm2": "iterm2", "terminal": "terminalapp"}


def category_score(test_results, key):
    """(errors, total) summed over the Unicode versions (or languages) a category was measured on."""
    block = (test_results or {}).get(key) or {}
    errors = total = 0
    for v in block.values():
        if isinstance(v, dict):
            errors += int(v.get("n_errors", 0))
            total += int(v.get("n_total", 0))
    return errors, total


def ucs_row(label, doc):
    tr = doc.get("test_results") or {}
    cells = []
    for key, _ in UCS_CATEGORIES:
        e, n = category_score(tr, key)
        cells.append(f"{100.0 * (n - e) / n:.1f}% ({n - e}/{n})" if n else "-")
    term = doc.get("terminal_results") or {}
    modes = term.get("modes") or {}
    m2027 = modes.get(2027) or modes.get("2027") or {}
    g = "enabled" if m2027.get("enabled") else ("off" if m2027.get("supported") else "no")
    amb = doc.get("ambiguous_width")
    return [label] + cells + [g, {1: "narrow", 2: "wide"}.get(amb, "?"), f"{float(doc.get('seconds_elapsed') or 0):.0f}"]


def ucs_md(out, meta, runs, ucs_dir):
    import yaml  # the suites' venv has it

    lines = header("ucs-detect in each terminal", meta, [
        f"- suite: ucs-detect {meta.get('ucs_detect_rev', '')[:12]} (Jeff Quast, MIT), its default run "
        "(the contested codepoints and graphemes), wcwidth from suites.pin",
        "- each cell is the share of the sequences tested that the terminal sized as Unicode says "
        "(`passed/tested`)",
    ])
    folder = os.path.join(out, "ucs-detect")
    docs = {}
    if os.path.isdir(folder):
        for fn in sorted(os.listdir(folder)):
            if fn.endswith(".yaml"):
                with open(os.path.join(folder, fn), encoding="utf-8") as f:
                    docs[fn[:-5]] = yaml.safe_load(f) or {}
    if not docs:
        return "\n".join(lines + ["No ucs-detect results."]) + "\n"
    rows = []
    notes = []
    for t in ordered(docs):
        d = docs[t]
        if d.get("error"):
            notes.append(f"- {NAMES.get(t, t)}: ucs-detect stopped: {d['error']}")
            continue
        rows.append(ucs_row(NAMES.get(t, t), d))
        # the published results of the same terminal, as a cross-check of this run
        pub = PUBLISHED.get(t)
        if pub and ucs_dir:
            path = os.path.join(ucs_dir, "data", pub + ".yaml")
            if os.path.exists(path):
                with open(path, encoding="utf-8") as f:
                    pd = yaml.safe_load(f) or {}
                label = f"{NAMES.get(t, t)} published ({pd.get('software_version', '?')}, {pd.get('system', '?')})"
                rows.append(ucs_row("_" + label + "_", pd))
    lines.append(table(["terminal"] + [name for _, name in UCS_CATEGORIES] +
                       ["grapheme clustering (mode 2027)", "ambiguous width", "seconds"], rows))
    lines += [""] + notes + [
        "", "Rows in italics are the results ucs-detect's author published for the same terminal (its data/ folder),"
        " made on another machine and often another version: they show whether this harness measures the same thing,"
        " not what this run found.", "",
        "The yaml files in ucs-detect/ hold every failed sequence per terminal.", ""]
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--out", required=True)
    ap.add_argument("--ucs-detect", default="", help="the ucs-detect checkout (for its published results)")
    args = ap.parse_args()
    out = args.out
    meta = load_meta(out)
    runs = load_runs(out)
    written = []

    with open(os.path.join(out, "run.md"), "w", encoding="utf-8") as f:
        f.write(run_md(out, meta, runs))
    written.append("run.md")

    data = load_esctest(out)
    if data:
        text, names, terms = esctest_md(out, meta, runs, data)
        with open(os.path.join(out, "esctest.md"), "w", encoding="utf-8") as f:
            f.write(text)
        with open(os.path.join(out, "esctest-tests.md"), "w", encoding="utf-8") as f:
            f.write(esctest_tests_md(meta, data, names, terms))
        written += ["esctest.md", "esctest-tests.md"]
    ucs_folder = os.path.join(out, "ucs-detect")
    if os.path.isdir(ucs_folder) and any(f.endswith(".yaml") for f in os.listdir(ucs_folder)):
        with open(os.path.join(out, "ucs-detect.md"), "w", encoding="utf-8") as f:
            f.write(ucs_md(out, meta, runs, args.ucs_detect))
        written.append("ucs-detect.md")
    print("wrote " + ", ".join(written) + " in " + out)


if __name__ == "__main__":
    sys.exit(main())
