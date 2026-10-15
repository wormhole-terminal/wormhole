# esctest2 in each terminal

- date: 2026-10-15T11:48:57Z
- machine: Mac17,8, Apple M5 Pro, macOS 27.0.1
- checkout: 5e2f529c (with uncommitted changes)
- asked for: 80 x 25 cells, Menlo 13 pt (an app that can't be told from the command line has its own)
- load average at the start: 4.01 3.53 3.48
- suite: esctest2 2798f12149a1 (Thomas Dickey, GPL-2.0), run by tools/vt-compare/suites/esctest_run.py
- judged against the behaviour esctest2 describes (xterm without its bugs), not against xterm's known-bug annotations; "empty" and "blank" cells count as one
- 80 x 25 cells, the size esctest2 resets to

## What esctest2 needs from the terminal

| terminal | cursor position (DSR 6) | DA1 | text area size (CSI 18 t) | CSI 14 t | CSI 16 t | checksum of a rectangle (DECRQCRA) | checksum form |
|---|---|---|---|---|---|---|---|
| Wormhole | yes | yes | yes | yes | yes | yes | positive |
| Ghostty | yes | yes | yes | yes | yes | no | - |
| Alacritty | yes | yes | yes | yes | no | no | - |
| kitty | yes | yes | yes | yes | yes | no | - |
| WezTerm | yes | yes | yes | yes | yes | yes | positive |
| iTerm2 | yes | yes | yes | yes | no | no | - |
| Terminal | yes | yes | yes | yes | no | no | - |

DECRQCRA is how esctest2 reads the screen back, one cell at a time. A terminal that does not
answer it cannot be asked what is on the screen: a test that gets as far as a screen check is
`unmeasurable` for it and is left out of its pass count (tests that fail before that point are real
failures). Without CSI 18 t the size is taken from the tty instead, and only the tests of CSI 18 t
itself fail for it.

## Result

| terminal | version | pass | fail | unmeasurable | blocked | skipped | measured | pass of measured | pass on the 323 tests every measurable terminal ran |
|---|---|---|---|---|---|---|---|---|---|
| Wormhole | 5e2f529c (this checkout) | 538 | 28 | 0 | 0 | 1 | 566 | 95.1% | 295/323 (91.3%) |
| Ghostty | 1.3.1 | 141 | 185 | 240 | 0 | 1 | 326 | 43.3% | 141/323 (43.7%) |
| Alacritty | 0.17.0 | 121 | 215 | 230 | 0 | 1 | 336 | 36.0% | 121/323 (37.5%) |
| kitty | 0.49.2 | 135 | 194 | 237 | 0 | 1 | 329 | 41.0% | 135/323 (41.8%) |
| WezTerm | 20240203-110809-5046fc22 | 340 | 226 | 0 | 0 | 1 | 566 | 60.1% | 174/323 (53.9%) |
| iTerm2 | 3.7.4 | 186 | 140 | 240 | 0 | 1 | 326 | 57.1% | 186/323 (57.6%) |
| Terminal | 2.15 | 111 | 217 | 238 | 0 | 1 | 328 | 33.8% | 111/323 (34.4%) |

- `unmeasurable`: the test needed a screen check and the terminal does not answer DECRQCRA.
- `blocked`: esctest2's own set-up (reset) failed, so the test did not run.
- `skipped`: needs a VT level above the one asked for, or overwrites the clipboard.
- "pass of measured" compares terminals that were measured on different tests; the last column
  compares them on the same ones.

## By area

Cells are passes / tests measured in that area.

| area (tests) | Wormhole | Ghostty | Alacritty | kitty | WezTerm | iTerm2 | Terminal |
|---|---|---|---|---|---|---|---|
| Cursor movement (132) | 132/132 | 75/95 | 57/104 | 72/95 | 131/132 | 84/95 | 67/96 |
| Erasing and editing (129) | 129/129 | 4/8 | 1/8 | 4/8 | 79/129 | 8/8 | 3/8 |
| Rectangular operations (32) | 32/32 | 4/4 | 3/4 | 4/4 | 8/32 | 4/4 | 4/4 |
| Margins (9) | 9/9 | 1/1 | 1/1 | 1/1 | 9/9 | 1/1 | 1/1 |
| Modes and resets (120) | 120/120 | 29/89 | 24/90 | 25/92 | 63/120 | 48/89 | 22/90 |
| Tabs (6) | 6/6 | 3/6 | 5/6 | 5/6 | 6/6 | 5/6 | 5/6 |
| Cursor save and restore (22) | 22/22 | 12/14 | 9/14 | 8/14 | 20/22 | 10/14 | 8/14 |
| Reports (30) | 28/30 | 5/30 | 1/30 | 4/30 | 3/30 | 14/30 | 0/30 |
| Colors, titles, selection (52) | 30/51 | 8/51 | 20/51 | 12/51 | 14/51 | 7/51 | 0/51 |
| Strings (APC, DCS, PM, SOS) (7) | 7/7 | - | - | - | 7/7 | - | - |
| Window operations (28) | 23/28 | 0/28 | 0/28 | 0/28 | 0/28 | 5/28 | 1/28 |

## Commonest failure reasons

- Wormhole: TestFailure x28
- Ghostty: TestFailure x105; no reply x80
- Alacritty: TestFailure x154; no reply x61
- kitty: TestFailure x132; no reply x62
- WezTerm: TestFailure x88; ChecksumException x75; no reply x63
- iTerm2: TestFailure x111; no reply x29
- Terminal: no reply x155; TestFailure x61; InternalError: Read [ (0x5b), expected P (0x50) x1

Every test, per terminal: esctest-tests.md. esctest2's own log and the traceback of each failed
test are in esctest/<terminal>.log and .log.trace.

