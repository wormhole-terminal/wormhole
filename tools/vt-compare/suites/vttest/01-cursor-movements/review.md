# Menu 1: test of cursor movements

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `main.c, `tst_movements``) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | screen border and frame, 80 columns (CUF CUB CUU CUD CUP HVP IND NEL RI ED EL DECALN) | pass | unbroken border of `*` and `+`; the frame of 60 `E`s is centred with one free cell around the text |
| 002 | the same in 132 columns (DECCOLM) | pass | the engine resized to 132 columns, the frame centred |
| 003 | autowrap with control characters, 80 columns (DECAWM, DECSTBM, DECOM, BS, TAB) | pass | letters in order down both margins; `y` and `Z` end up where a VT100 puts them |
| 004 | the same in 132 columns | pass |  |
| 005 | cursor control characters inside escape sequences (BS, CR and VT inside CSI) | pass | four identical lines |
| 006 | leading zeros in escape sequences | pass | the sentence is on the row it should be |
