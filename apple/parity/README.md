# Feature parity tracker

How much of the GPUI app's UI the Swift app (`apple/swift`) already has, as two numbers anyone can recompute:

```sh
python3 -I apple/scripts/parity.py              # looks %, works %, per area, 10 heaviest gaps, unweighted counts
python3 -I apple/scripts/parity.py --markdown   # the same, for the status report
python3 -I apple/scripts/parity.py --size       # Swift lines per module against the GPUI app's 121,651
python3 -I apple/scripts/parity.py --check      # validate features.csv only
python3 -I apple/scripts/parity.py --selftest   # the script's own tests
```

`features.csv` is the data; the script holds no grades.

## Columns

| Column | Meaning |
|---|---|
| `id`, `area` | unique id; one of terminal, sidebar, window, panel, overlays, platform, aux, settings, studio, crosscut |
| `feature` | one feature, split finer than the inventory's rows where a row bundles distinct ones (selection vs mouse modes) |
| `weight` | GPUI production Rust lines of that feature (below) |
| `looks` | 0, 0.25, 0.5, 0.75, 1; blank when there is nothing visual to match (a key encoder) |
| `works_mock` | 0, 0.5, 1: how much of the feature behaves on sample or in-memory backends, in the module's tests or gallery or in the app |
| `works_real` | 0, 0.5, 1: how much of it runs in the app against the real Rust core or engine (or is real AppKit behaviour with nothing to mock) |
| `works` | the combined number, kept for continuity; computed, and `--check` rejects any other value: `works_real`, else 0.5 when `works_mock` > 0 and the feature is wired |
| `wired` | `yes` the app reaches it; `part` reached only in a mode or without all its triggers; `no` merged in a module but nothing in the app opens it |
| `evidence` | what justifies the grade: `;`-separated repo paths (checked to exist; the script uses the csv module, so a comma in a field is quoted) or `png:<folder>/<file>` for a screenshot. Screenshots are not in the tree (41 MB): those taken up to 2026-11-11 are on the branch `backup/exp-swift-2026-11-11` under `apple/parity/evidence/`, the same folder and file names; a new one is made again with the steps in the row's notes. A row with any grade above 0 must have evidence |
| `owner` | worktree currently responsible (optional) |
| `notes` | what is missing, and what is merged but unwired |

## Rubric

**looks**: how closely the Swift UI matches the GPUI app on sample data, compared side by side with the GPUI
reference render (`gpui-ref/`) in the same theme.

| Grade | Meaning |
|---|---|
| 0 | not drawn |
| 0.25 | something is there but it is a different thing (a plain menu where GPUI has a card) |
| 0.5 | the right parts and wording, but visibly different structure or missing parts |
| 0.75 | the same layout, metrics, colours and wording; small differences remain (icons, a state) |
| 1 | cannot be told apart in the compared states, in light and dark |

**works_real**: the behaviour runs in the app against the real Rust core or engine.

| Grade | Meaning |
|---|---|
| 0 | not at all; a feature nothing in the app opens (`wired` no) is always 0 |
| 0.5 | part of it is real (the list comes from the core, the git facts are samples) |
| 1 | all of it runs against the real core or engine, or is real AppKit/SwiftUI behaviour with nothing to mock |

**works_mock**: 1 when a complete behaviour exists on sample or in-memory backends (tests, gallery or window), 0.5 when part of
it does, 0 when nothing does. It is graded even for a module nothing opens, so the work shows; but a module with `wired`
no adds nothing to the combined `works`, because no person can reach it.

"Real" needs the FFI build and a program (`--shell`); the default run is the sample workspace and mock sources.

Grade only what is on the branch you are grading, from the Swift code and the renders, never from an agent's
report. When in doubt give the lower grade. A feature that GPUI does not have (IME, bidi) is an improvement, not
parity: it is not a row (the notes of the nearest row mention it).

## Weights

A feature's weight is the production lines (no tests) of the GPUI app crate that implement it, from
`repo-docs/docs/research/swift-parity-ui-inventory.md` section 2 ("Rust (app)" column, measured with `wc -l`).
Where a row bundles several features the lines are split by the files named in the row (for example `input.rs`
579 lines became selection 300 and mouse 280); where the row names no files the split is an estimate and says
`about` in the inventory or here. The rows added from section 3 (design language 11,363; theme bridge 960; dev tooling
5,609) use that section's numbers.

The 85 rows add up to 105,270 of the app crate's 121,651 production lines; the remaining 16k is platform-edge
code spread across features (objc2 ceremony), in-module dev files and plumbing no feature owns. Weight is
GPUI's cost, not Swift's: a feature that SwiftUI gives for free still counts its GPUI lines, so the number says
how much of the old app's scope exists, not how much Swift was written.

## The numbers

- **LOOKS %** = sum(weight x looks) / sum(weight) over the rows that have a looks grade.
- **WORKS_REAL %**, **works_mock %**, **works (combined) %** = sum(weight x grade) / sum(weight) over all rows. Report the
  real one as the works figure and the others beside it.
- **Heaviest gaps** = the weight not yet done, against the real target: `weight x (1 - mean of looks and works_real)`
  (works_real alone for a row without a look).
- **Unweighted** = rows with looks >= 0.75, rows with works_real >= 0.5, rows with combined works >= 0.5, as counts.
- **Size** (`--size`) = Swift lines under `apple/swift/Sources`, per module, without `WormholeBindings` (generated by
  UniFFI), against the app crate's 121,651 production lines. The naive projection divides by the works share: it
  divides by the combined share and assumes the remaining features cost as much per GPUI line as the finished ones, which flatters the plan because the
  cheap, visible rows are done first. Treat it as a floor.

## When to regrade

After every merge into the experiment's main branch (`exp/swift-ui-metal` today) and before every status report.
Change the grades in the same commit as the feature, with the evidence that justifies them, and run `--check`.
A row's grades only go down when a regression is found. When a feature is split further, keep the weights adding up.

## Grading history

2026-11-07, first grade, of `exp/swift-ui-metal` at 871971ee2 (before the gap-filling worktrees): looks 33.2 %, works
35.8 % (one works column).

2026-11-07, second grade, at f01a8a642 (the gap-filling modules merged, the window running on the core's workspace through
the FFI): works split into mock and real, `wired` added. The Studio, the folder picker, global hotkey, notifications,
minimap, command blocks, text zoom, print/export and agent card are merged but nothing in the app opens them.

2026-11-07, third grade, at 27b545ef3 (the backends wired): the git panel, Files/Project/Usage/Notes, Settings, the aux
windows, the keymap menu bar, palette matcher and words, the workspace's status machine and git feed, the hook socket and
the Theme Studio launcher are real in a window started with a program (`--shell`) on the FFI build; looks 63.5 %,
works_real 69.3 %, works_mock 65.5 %, combined 69.9 %. Real grades come from the code and its tests, not from a live
run of every row. Still merged but unwired: minimap, command blocks, print/export (T10, T13, T22).
