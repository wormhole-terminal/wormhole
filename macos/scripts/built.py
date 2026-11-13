#!/usr/bin/env python3
"""Built against verified: `features.csv` holds what was checked (works_real); `built.csv` holds what is written and
compiles but has not been driven or tested yet (0, 0.5 or 1 per row; never below the row's works_real). Prints both
figures, weighted by the tracker's weights. `--check` fails on a missing or unknown row or a bad grade."""
import csv, sys
from pathlib import Path

parity = Path(__file__).resolve().parent.parent / "parity"
features = {r["id"]: r for r in csv.DictReader(open(parity / "features.csv"))}
built = {r["id"]: r["built"] for r in csv.DictReader(open(parity / "built.csv"))}
problems = [f"{i}: not in built.csv" for i in features if i not in built] + [f"{i}: not a tracker row" for i in built if i not in features]
problems += [f"{i}: built {g} is not 0, 0.5 or 1" for i, g in built.items() if g not in ("0", "0.5", "1", "1.0", "0.0")]
if problems:
    print("\n".join(problems)); sys.exit(1)
weight = sum(float(r["weight"]) for r in features.values())
verified = sum(float(r["weight"]) * float(r["works_real"]) for r in features.values()) / weight * 100
written = sum(float(r["weight"]) * max(float(built[i]), float(r["works_real"])) for i, r in features.items()) / weight * 100
waiting = sorted(i for i, r in features.items() if float(built[i]) > float(r["works_real"]))
# The last audit's finer figure, where there is one (`written.csv`: per row, how much of it is written and reachable, 0 to 100).
audit = parity / "written.csv"
if audit.exists():
    read = {r["id"]: float(r["written"]) for r in csv.DictReader(open(audit))}
    unknown = [i for i in read if i not in features] + [i for i in features if i not in read]
    if unknown:
        print("written.csv: rows do not match the tracker: " + " ".join(unknown)); sys.exit(1)
    print(f"written by the last audit {sum(float(r['weight']) * read[i] for i, r in features.items()) / weight:.1f} %")
print(f"built {written:.1f} %   verified {verified:.1f} %   ({len(waiting)} rows built and waiting for the check: {' '.join(waiting)})")
