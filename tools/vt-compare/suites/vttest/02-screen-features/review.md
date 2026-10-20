# Menu 2: test of screen features

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `main.c, `tst_screen``) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | wrap around mode (DECAWM) | pass | three identical rows of `*`, none empty between |
| 002 | tab setting and clearing (HTS, TBC 0, 1, 2, 3) | pass | both lines have a `*` every six columns, starting at 7 |
| 003 | 132 column mode, light background (DECCOLM, DECSCNM) | pass | size 132x24; reverse video is a screen mode (`+REVERSE_VIDEO`), no cell carries it |
| 004 | 80 column mode, light background | pass |  |
| 005 | 132 column mode, dark background | pass |  |
| 006 | 80 column mode, dark background | pass |  |
| 007 | soft scroll down, region 12 to 13 (DECSCLM, DECSTBM, DECOM, RI) | pass | smooth scrolling is a mode the engine only records (`+SMOOTH_SCROLL`); the lines are where a VT100 leaves them |
| 008 | soft scroll down, region 1 to 24 | pass |  |
| 009 | jump scroll down, region 12 to 13 | pass |  |
| 010 | jump scroll down, region 1 to 24 | pass |  |
| 011 | origin mode with a region at the bottom (DECOM) | pass | the first line is on row 23, above the last row, the second on the last row |
| 012 | origin mode off | pass |  |
| 013 | graphic rendition test pattern, dark background (SGR 0, 1, 4, 5, 7 in every combination) | pass | every combination is carried, including the empty parameter in `1;4;;5;7` (a reset, so no bold and no underline) |
| 014 | the same, light background (DECSCNM) | pass |  |
| 015 | save and restore cursor (DECSC, DECRC) with graphics character sets and attributes | pass | ten characters of each kind (attributes and character set come back), a 5 x 4 rectangle of `A`s |
