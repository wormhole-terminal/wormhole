# Menu 7: test of VT52 mode

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `vt52.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | VT52 cursor addressing and graphics (DECANM reset, `ESC Y`, `ESC F`, ...) | pass | the screen is cleared and has the centred rectangle of `*` with `!` inside, and "Only this, and nothing more." as the text says: reverse line feed scrolls down, an address past the screen moves only the other axis |
| 002 | VT52 normal and special graphics character sets | pass | the normal set, then the special one (`ESC F`). The graphics are the VT100's (DEC special graphics) as in xterm; the VT100 User Guide says the VT52's own differ in some characters, and no published table of them is at hand |
| 003 | IDENTIFY (`ESC Z`) and returning to ANSI mode, waiting for Return | prompt | `ESC Z` is answered `ESC / Z` ("OK (means VT100 emulating VT52)"). After `ESC <` the terminal is at the VT100 level, where DECRQSS (a VT320 function) is not recognised: vttest's request for DECSCL gets no answer and "you should have to press return to continue" holds; the host presses Return (a screen of its own, not a result) |
| 004 | the same test, after the Return | pass | DECRQSS unanswered at the VT100 level (the trace shows vttest's "Response was <10> ok"), and S8C1T is not recognised there either, as vttest wants. vttest clears its results before the screen the host sees, so this one shows only the next prompt. Before, DECRQSS was answered at every level ("DECSCL recognized -- failed") |
