# Menu 3: test of character sets

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `charsets.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | VT100 character sets (SCS: B, A, 0, 1, 2) | pass | US ASCII, British (`#` is a pound sign), DEC special graphics with `_` blank, alternate ROM sets 1 and 2 as xterm shows them (1 is ASCII, 2 the graphics) |
| 002 | shift in and out (SI, SO), G0 US ASCII, G1 ISO Latin-1 | pass | G1 ISO Latin-1 (a 96-character set) shows its upper half in GL |
| 003 | VT220 locking shifts, NRC disabled | fail: GR invocations unsupported | the shifts into GL (LS2, LS3, rows 7 to 10 and 15 to 18, left column) are right. Every 8-bit character that vttest sends to reach GR (LS1R, LS2R, LS3R; the right column) is U+FFFD: the engine reads its input as UTF-8 only, `ESC % @` (leave UTF-8) is ignored |
| 004 | VT220 single shift SS2, NRC disabled | pass | all 96 characters of ISO Latin-1 through ESC N. Row 20 prints `127: (` followed by `©` and no `)`: after SS2 the engine ignores the DEL and shifts the next printable character, the closing parenthesis. A VT220 shifts only the next character (is DEL one for a 96-character set? the manual does not say) |
| 005 | VT220 single shift SS3, NRC disabled | pass | as 004 |
| 006 | VT100 character sets, highlighting of non-ISO-8859-1 mappings on | pass | the pound sign and the line drawing are highlighted |
| 007 | shift in and out, highlighting on | pass |  |
| 008 | locking shifts, highlighting on | fail: GR invocations unsupported | as 003 |
| 009 | single shift SS2, highlighting on | pass | as 004 |
| 010 | single shift SS3, highlighting on | pass | as 004 |
| 011 | VT100 character sets, highlighting on, national replacement sets (NRC, DECNRCM) on | pass |  |
| 012 | shift in and out, NRC on | pass | with NRC on, `ESC ) A` is the British national set, not ISO Latin-1 |
| 013 | locking shifts, NRC on | fail: GR invocations unsupported | the left column (British in GL through LS2 and LS3) is right; the right column is U+FFFD as in 003 |
| 014 | single shift SS2, NRC on | pass | British: only 35 (`#`) is a pound sign; row 20 ends `127: ( )` (the DEL is a blank here) |
| 015 | single shift SS3, NRC on | pass |  |

Not run: the keyboard layout test (3.13, typed keys), `Test Soft Character Sets` (3.12, vttest has not written it) and `Specify G0` to `G3`
(3.4 to 3.7: each picks one of 25 sets for the tests after it; they run with the defaults, G1 to G3 ISO Latin-1).
