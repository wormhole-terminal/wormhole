# Menu 10: test of reset and self-test

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `reset.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | RIS, before | pass |  |
| 002 | after RIS (the screen is cleared, the cursor at home) | pass | auto repeat is back on (`+AUTO_REPEAT`), as after a reset |
| 003 | DECTST, before | pass |  |
| 004 | after DECTST (`CSI 4;1 y`, the confidence test) | unsupported: DECTST | ignored, as xterm does: nothing to show and no error code |
| 005 | DECSTR, before | pass |  |
| 006 | after DECSTR | pass | nothing is drawn by the test itself (the effects are tested by 11.1.6 and esctest2) |
