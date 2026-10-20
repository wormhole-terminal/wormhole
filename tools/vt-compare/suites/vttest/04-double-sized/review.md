# Menu 4: test of double-sized characters

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `main.c, `tst_doublesize``) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | DECSWL, DECDWL and DECDHL, 80 columns, a normal line below | pass | the capture's line sizes: row 9 double-width, rows 11 and 14 the top halves and 12 and 15 the bottom halves of double-height lines, every other row normal (the last one, DECSWL, is normal) |
| 002 | the same with the last line changed to double width | pass | row 21 is double-width now |
| 003 | DECDWL and DECDHL, 132 columns | pass | as 001 |
| 004 | the same with the last line changed to double width, 132 columns | pass | as 002 |
| 005 | a frame of line-drawing characters and blinking bold text in double-height double-width size | pass | the right border is at column 40, the last column of a double-width line, so the frame is square as on a VT100; bold and blink are carried; rows 8 to 17 alternate top and bottom halves of double-height lines (the capture's line sizes). The text row's blink run is one column longer on the top row than on the bottom: vttest writes a trailing blank in bold blink there (`main.c`) |
| 006 | half of the box stays after the scroll (DECSTBM, RI) | pass | rows 20 to 24 hold the top five rows of the box. The box's rows keep their sizes when scrolled: rows 20 to 24 are top, bottom, top, bottom, top halves |
