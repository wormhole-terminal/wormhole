# Menu 9: test of known bugs (of the VT100)

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `main.c, `tst_bugs` and `bug_*``) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | bug A, the introduction (scroll while toggling smooth scroll) | pass |  |
| 002 to 012 | bug A, ten times: one line more at the bottom, no lines lost or repeated, the cursor stays | pass | DECSCLM toggled between line feeds does not disturb scrolling |
| 013 | bug B, scrolling region and double-width line, before the test | pass | line 11 is double-width (line sizes) |
| 014 | bug B: RI at the top margin, then A to P written down the region | pass | F to M on the `E` rows, then N, O and P on blank rows: what a terminal without the bug shows. Line 11 stays double-width above the region |
| 015 | bug C: wide to narrow screen | pass | only the one line is left |
| 016 | bug D: narrow to wide screen, the screen waiting for 0 or 1 | pass | the host answered 0. The text is double-width (rows 1, 3, 4 and 24 are) and blinking, with bold, blink and reverse carried |
| 017 | bug D, the five lines after | pass |  |
| 018 | bug E: cursor move from a double- to a single-wide line | pass | the `X` is at line 3 column 100, above the `+`. (Line 1 is double-wide, line 3 is not) |
| 019 | bug F, part 1: reverse and 132 columns set together | pass | `+COLUMNS_132 +REVERSE_VIDEO` |
| 020 | bug F, part 2: `CSI ? 6;5;3 l` resets all three (the VT100 forgot the rest after origin mode) | pass | 80 columns, not reversed |
| 021 | wrap around with cursor addressing (the wrap around bug) | pass | a row of 79 `+` and `*` all down the last column |
| 022 | erase right half of double-width lines: a line of 80 characters | pass |  |
| 023 | the same line set to double width | pass | row 15 is double-width: its first 40 characters show |
| 024 | the same line back to single width | pass | 80 characters |
| 025 | the same test in 132 columns, single width | pass |  |
| 026 | set to double width, 132 columns | pass | row 15 is double-width: its first 66 characters show |
| 027 | back to single width, 132 columns | pass |  |
| 028 | funny scroll regions: `DECSTBM 20;10` is no region | pass | twenty lines, no scrolling |
| 029 | `DECSTBM 0;1` is no region either | pass |  |
