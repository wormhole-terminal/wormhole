# Menu 6: test of terminal reports

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `reports.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | ENQ and the answerback message | pass | the answerback is empty (the engine has none, as xterm by default): the reply line is blank |
| 002 | LNM, mode set: the screen waits for Return | prompt | not a result: the host presses Return (`encode_key`) at this screen; see 003 and 004 |
| 003 | LNM set, Return pressed | fail: LNM ignored by the key encoder | with LNM (ANSI mode 20) set, Return must send CR LF (`<13><10>`); the engine's `encode_key` has no LNM input and sent `<13>` |
| 004 | LNM reset, Return pressed | pass | `<13>` is right. (The set case failed: 003) |
| 005 | device status reports (DSR 5 and 6) | pass | `CSI 0 n` terminal OK, cursor position `CSI 5;1 R` twice, both right |
| 006 | primary device attributes (DA) | pass | the engine answers `CSI ? 65;1;4;6;9;15;16;17;18;21;22;28;29;52 c`, VT500 family. vttest does not know extension 52 (xterm's clipboard) and prints `BAD VALUE`: that is vttest's list, not the engine. The engine claims 17 (terminal state reports), 18 (user windows) and 21 (horizontal scrolling); see 11.2.5.4.2 and the pan tests for what is behind them |
| 007 | secondary device attributes | pass | `CSI > 64;777;0 c`: VT520, firmware 777, keyboard ok |
| 008 | tertiary device attributes (unit ID) | pass | `DCS ! | 00000000 ST` |
| 009 | request terminal parameters (DECREQTPARM, arguments 0 and 1) | fail: no report | the engine sends nothing for `CSI 0 x` and `CSI 1 x`; a VT100 (and xterm) answers `CSI 2;1;1;112;112;1;0 x` and `CSI 3;...`. vttest says `Bad format` |
