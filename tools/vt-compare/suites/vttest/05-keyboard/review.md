# Menu 5: test of keyboard

Each screen below is the engine's screen at one `Push <RETURN>` of vttest 20251205, checked against what vttest says
the screen should show (its text is on the screen, its source is `keyboard.c`) and, where that is not enough, against
the VT100/VT220 manuals and xterm's behaviour. Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.

| screen | test | verdict | notes |
|--------|------|---------|-------|
| 001 | keyboard LEDs, all four on (DECLL 1 2 3 4) | unsupported: DECLL | the engine keeps no LED state (they are keyboard hardware); the screen is only vttest's own text |
| 002 | LEDs 2, 3 and 4 on | unsupported: DECLL |  |
| 003 | LEDs 2 and 3 on | unsupported: DECLL |  |
| 004 | LEDs 1 and 2 on | unsupported: DECLL |  |
| 005 | LED 1 on | unsupported: DECLL |  |
| 006 | all LEDs off | unsupported: DECLL |  |

Not run, because they read the keys a person presses (the typing itself is the test): 5.2 auto repeat, 5.3 keyboard layout, 5.4 cursor keys,
5.5 numeric keypad, 5.6 editing keypad, 5.7 function keys, 5.8 the answerback message and 5.9 control keys. The engine's key encoder
(`wormhole_vt::input`) has its own tests; vttest's keyboard menu would need the host to press every key, which this host does only
for the few screens in the non-VT100 menu that wait for one (11.3.4).
