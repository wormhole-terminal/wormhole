# Menu 8: test of VT102 features (insert and delete character and line)

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `main.c, `tst_insdel``) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | screen accordion before the test, 80 columns | pass | each row filled with its letter |
| 002 | insert and delete line (IL, DL) with a scrolling region and origin mode | pass | top row A, bottom row X, the text row, nothing else |
| 003 | insert mode (IRM) | pass | `A***...***B` |
| 004 | delete character (DCH) | pass | `AB` |
| 005 | right column staggered by one (DCH on single-width lines) | pass |  |
| 006 | the same on double-width lines (DECDWL) | pass | all 24 rows are double-width (line sizes) and each is one shorter than the one above it: the right column is staggered by one |
| 007 | insert character (ICH) | pass | both lines `A B C ... Z` |
| 008 | screen accordion, 132 columns | pass |  |
| 009 | insert and delete line, 132 columns | pass |  |
| 010 | insert mode, 132 columns | pass |  |
| 011 | delete character, 132 columns | pass |  |
| 012 | right column staggered, 132 columns | pass |  |
| 013 | the same on double-width lines, 132 columns | pass | as 006 |
| 014 | insert character, 132 columns | pass |  |
