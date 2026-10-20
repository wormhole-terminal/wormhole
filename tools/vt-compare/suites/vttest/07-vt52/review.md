# Menu 7: test of VT52 mode

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `vt52.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | VT52 cursor addressing and graphics (DECANM reset, `ESC Y`, `ESC F`, ...) | pass | the screen is cleared and has the centred rectangle of `*` with `!` inside, and "Only this, and nothing more." as the text says: reverse line feed scrolls down, an address past the screen moves only the other axis |
| 002 | VT52 normal and special graphics character sets | pass | the normal set, then the special one (`ESC F`). The graphics are the VT100's (DEC special graphics) as in xterm; the VT100 User Guide says the VT52's own differ in some characters, and no published table of them is at hand |
| 003 | IDENTIFY (`ESC Z`) and returning to ANSI mode | fail: DECRQSS is answered at VT100 level | `ESC Z` is answered `ESC / Z` ("OK (means VT100 emulating VT52)") and `ESC <` leaves VT52 mode at VT100 level (S8C1T is then not recognised, as vttest wants), but DECRQSS for DECSCL is answered, where a VT100 has none: vttest says "DECSCL recognized -- failed". The screen shown is the prompt |
