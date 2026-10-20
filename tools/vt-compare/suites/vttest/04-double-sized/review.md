# Menu 4: test of double-sized characters

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `main.c, `tst_doublesize``) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | DECSWL, DECDWL and DECDHL, 80 columns, a normal line below | unsupported: DECDWL, DECDHL | ESC # 3, 4, 5 and 6 are ignored: the double-width line and both halves of the double-height lines are drawn in normal size. Expected: 40 columns of double-width characters per line |
| 002 | the same with the last line changed to double width | unsupported: DECDWL | the row is not widened |
| 003 | DECDWL and DECDHL, 132 columns | unsupported: DECDWL, DECDHL | as 001 |
| 004 | the same with the last line changed to double width, 132 columns | unsupported: DECDWL |  |
| 005 | a frame of line-drawing characters and blinking bold text in double-height double-width size | pass (text) | the right border is at column 40, the last column of a double-width line, so the frame is square as on a VT100; bold and blink are carried. The capture holds text only, so the double height itself is checked by the engine's `line-size.vtt` cases and the app's screenshots |
| 006 | half of the box stays after the scroll (DECSTBM, RI) | pass | rows 20 to 24 hold the top five rows of the box. The box's rows keep their double width when scrolled |
