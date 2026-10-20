# Menu 7: test of VT52 mode

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `vt52.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | VT52 cursor addressing and graphics (DECANM reset, `ESC Y`, `ESC F`, ...) | unsupported: VT52 mode | `CSI ? 2 l` is ignored (the engine is always in ANSI mode, by design), so every VT52 sequence is shown as text: the screen is the program's bytes drawn as characters |
| 002 | VT52 normal and special graphics character sets | unsupported: VT52 mode | as 001 |
| 003 | IDENTIFY (`ESC Z`) and returning to ANSI mode | unsupported: VT52 mode | the screen has only the prompt: the VT52 specific questions are not shown. (`ESC Z` in ANSI mode is answered as DECID) |
