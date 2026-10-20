# Menu 11: non-VT100 terminals (VT220, VT320, VT420, VT520, ISO 6429, xterm)

Each screen below is the engine's screen at one `Push <RETURN>` (or wait for a key) of vttest 20251205, checked against what
vttest says the screen should show (its text is on the screen, its source is `vt220.c`, `vt320.c`, `vt420.c`, `vt520.c`,
`nonvt100.c`, `color.c`, `xterm.c`) and, where that is not enough, against the VT220/VT420/VT520 manuals and xterm's behaviour.
Verdicts: `pass`, `fail: <why>`, `unsupported: <feature>`; `prompt` is a screen that waits for the key the host then presses.
The golden files are the engine's behaviour today, whatever the verdict: the gate keeps them from changing by accident.
The path of each screen is in its header (`path:`); the tests repeat under switches (origin mode, left and right margins) and
in the colour menu, so the same test comes up more than once.

| screens | test | verdict | notes |
|---------|------|---------|-------|
| 001 | DSR keyboard status (`CSI ? 26 n`) | pass | `CSI ? 27;1;0;0 n`: North American, ready, LK201 |
| 002 | DSR operating status | pass | `CSI 0 n` |
| 003 | DSR printer status | pass | `CSI ? 13 n`: no printer |
| 004 | DSR user-defined key status | pass | `CSI ? 20 n`: unlocked |
| 005 to 006 | visible and invisible cursor (DECTCEM) | pass | the cursor is hidden (`(hidden)`, `-SHOW_CURSOR`) and then shown |
| 007 | erase characters (ECH) | pass | E's with a gap before the diagonal, the lower right cleared |
| 008 to 010 | protected areas: DECSED and DECSEL, then ED and EL, then ECH, ICH and DCH, ignoring protection (DECSCA) | pass | a solid box of `*` each time, nothing else |
| 011 | 8-bit controls (S7C1T, S8C1T) | pass | both reports `ok` |
| 012 to 015 | media copy: assign the printer, printer to host, printer extent (DECPEX), form feed (DECPFF) | unsupported: printer (media copy) | nothing to show, and nowhere to print: the engine has no printer (DSR says so, 003) |
| 016 | auto-print mode (`CSI ? 5 i`) | unsupported: printer (media copy) | the screen is as asked (the pattern is visible); nothing is printed |
| 017 | printer controller mode (`CSI 5 i`) | fail: printer controller mode | the pattern should be invisible, as it goes to the printer; it is drawn on the screen |
| 018 to 021 | print page, print composed display, print all pages, print the cursor line (`CSI i`, `CSI ? 1 i`, ...) | unsupported: printer (media copy) | the screen is as asked; nothing is printed |
| 022 | soft character sets (DECDLD) | not exercised | vttest needs a font file (`-f`); none is given. The engine's DA does not claim soft characters either |
| 023 to 024 | soft terminal reset (DECSTR) | pass | nothing is drawn by the test itself |
| 025 | user-defined keys (DECUDK), waiting for keys | prompt | the host presses `q` |
| 026 | user-defined keys after `q` | unsupported: DECUDK | only `q` was pressed: the test needs F-keys defined by `DCS ... | ... ST` to echo their labels, and the host does not press them; the engine has no key definitions |
| 027 | pan down (SU) | pass | a row of `*` on the top row |
| 028 | pan up (SD) | pass | a row of `*` just above the message |
| 029 to 031 | set columns per page, 80 (DECSCPP) | pass | 80 of 80 columns filled |
| 032 | set columns per page to 132 (DECSCPP) | fail: DECSCPP unsupported | `CSI 132 $ |` is ignored: the screen stays 80 columns (80 of 132 filled). The same request is answered by DECRQSS (62) as 80 |
| 033 | back to the original width | pass |  |
| 034 to 035 | set lines per page, 24 and 25 (DECSLPP) | pass | the engine is 80x24 then 80x25 (the host resizes its window) |
| 036 to 037 | set lines per page, 36 and 48 | pass |  |
| 038 to 039 | set lines per page, 72 and 144 | pass | the host window allows 54 lines, so the engine is 80x54: the limit is the fake window's |
| 040 | back to 24 lines | pass |  |
| 041 to 043 | DSR keyboard, printer and user-defined key status (the VT320 menu) | pass | as 001 to 004 |
| 044 | DSR locator status (`CSI ? 55 n`) | pass | `CSI ? 53 n`: no locator (the host has none to offer, although DA claims a locator port) |
| 045 | DSR identify locator | pass | `CSI ? 57;1 n`: the locator is a mouse |
| 046 | extended cursor position (DECXCPR) | pass | `CSI ? 2;1;1 R` |
| 047 to 048 | cursor information report (DECCIR), then with national replacement sets | pass | cursor (3,10), page 1, GL G0, GR G2, sets ISO Latin-1 then British |
| 049 to 051 | tab stop report (DECTABSR): default, after reset, after setting every 7th | pass | the report lists the stops the rulers show. The default list starts at column 1 (`1/9/17/...`) as xterm's does |
| 052 | ANSI mode report (DECRPM, `CSI Ps $ p`) | pass | GATM, SRTM, VEM, HEM, PUM, FEAM, FETM, MATM, TTM, SATM, TSM, EBM permanently reset, KAM IRM LNM reset, SRM set; CRM (3) is `unknown` |
| 053 to 054 | DEC mode report (DECRPM, `CSI ? Ps $ p`) | pass | states match the modes (DECAWM set, DECCOLM reset ...); modes the engine does not have (DECEDM, DECLTM, DECSCFDM, DECTEM, DECEKEM, DECGEPM, DECGPCM, DECGPBM, DEC131TM, DECKKDM) answer `unknown`. DECBKM is reset (the backarrow key sends DEL) |
| 055 to 056 | restore presentation state, cursor (DECRSPS): renditions, flags, modes, character sets | pass | all 25 checks `OK` |
| 057 | restore presentation state, tab stops | pass | the original and four modified rulers |
| 058 | DECRQSS: protected fields attributes (DECPRO) | unsupported: DECRQSS DECPRO | answered as an invalid request (`DCS 0 $ r`) |
| 059 to 066 | DECRQSS: DECSASD, SGR, DECSCA, DECSCPP, DECSCL, DECSLPP, DECSSDT, DECSTBM | pass | each valid and the value current (conformance level 65, 7-bit controls) |
| 067 to 068 | DECRQSS: transmit termination character (DECTTC), transmission line termination (DECTLTC) | unsupported: DECRQSS DECTTC, DECTLTC | invalid requests |
| 069 | terminal state report (DECRQTS, DECTSR) | pass |  |
| 070 | assign the user-preferred supplemental set (DECAUPSS) | pass | ISO Latin-1 assigned; nothing is drawn |
| 071 | DECRQUPSS | pass | `DCS 1 ! u A ST`: ISO Latin-1 |
| 072 | window report (DECRPDE) | pass | `CSI 24;80;1;1;1 " w` |
| 073 to 081 | status line (DECSSDT, DECSASD): simple text, graphic rendition, cursor movement | unsupported: status line | the engine has no status line (DECRQSS 065 says none), but it does not drop what is written to it: the text is drawn on the main screen at the cursor (073: `TEXT IN THE STATUS LINEThere should be ...`) |
| 082 to 083 | back index and forward index (DECBI, DECFI) | pass | the top row is numbered 1 through 20 |
| 084 to 085 | cursor movement and other movement (CR, HT, LF, FF) within margins, no margins set | pass | the box of `*` is overwritten entirely |
| 086 to 089 | the same two and DECBI, DECFI with origin mode, left/right margins and top/bottom margins on | pass | numbered 1 through 10, the box of `*` overwritten inside the margins |
| 090 | DECRQM for DECLRMM (left and right margins mode) | pass | reset |
| 091 to 092 | DECRQSS for DECSTBM and DECSLRM | pass | `1;24 r` and `1;80 s` |
| 093 to 094 | insert and delete column (DECIC, DECDC) | pass | letters a to t on column 80, then on column 1 |
| 095 to 098 | vertical scrolling (IND, RI) at the ends of the screen | pass | `abcd...` at the top, `0123...` at the bottom |
| 099 to 102 | insert and delete line (IL, DL) | pass |  |
| 103 to 104 | insert and delete character (ICH, DCH) | pass | letters a to t on column 80, then on column 1 |
| 105 | ASCII formatting (BS, CR, TAB) | pass | the repeating pattern falls within the `-` and `+` margins |
| 106 to 121 | the editing tests again with origin mode and the left/right margins on (DECRQM set, DECSTBM `1;12`, DECSLRM `1;40`, column 40, scrolling within the margins) | pass | every result is as in the unmargined run, inside the margins |
| 122 | backarrow key (DECBKM), waiting for the key | prompt | the host presses Backspace |
| 123 to 124 | backarrow key: with the mode set, then reset | pass | set sends `<8>` (BS), reset `<127>` (DEL): the key encoder follows DECBKM. (Before `InputModes` had `backarrow_bs` both sent DEL: vttest found it) |
| 125 | numeric keypad (DECNKM), waiting | prompt |  |
| 126 | keypad 5 with the mode reset | pass | `5` |
| 127 | waiting, keypad mode set | prompt | `+APP_KEYPAD` |
| 128 to 129 | keypad 5 with the mode set (DECNKM, or `ESC =`) | pass | the application keypad sends `ESC O u`. (Before `InputModes` had `app_keypad` it sent `5`: vttest found it) |
| 130 to 134 | keyboard usage (DECKBUM), a letter pressed in each mode | pass | a letter is the same in data-processing and typewriter mode; the effect (shift lock) cannot be seen with it |
| 135 to 139 | key position (DECKPM), a letter pressed in each mode | fail: DECKPM unsupported | with position reports on, a key must report its position (`CSI ... , }`); the engine sent `a` in both |
| 140 to 141 | change attributes in a rectangle (DECCARA): rectangle, then stream extent | pass | an open rectangle of reverse-video E's; with DECSACE stream it wraps at the screen's edges as the stripes require |
| 142 to 145 | copy a rectangle (DECCRA), overlapping included | pass |  |
| 146 | erase a rectangle (DECERA) | pass |  |
| 147 to 148 | fill a rectangle (DECFRA) and erase it | pass |  |
| 149 to 150 | reverse attributes in a rectangle (DECRARA): rectangle, stream | pass | as DECCARA |
| 151 to 154 | selective erase of a rectangle (DECSERA): border, then inside | pass | protected cells stay, unprotected are erased |
| 155 | DECCARA in a rectangle with the margins on | pass | the rectangle is inside the margins |
| 156 | DECCARA, stream extent, margins on | fail: stream extent does not wrap at the margins | vttest: "combined with wrapping at the margins"; the engine wraps at the screen's edges (the first stripe runs to column 80 though the right margin is 40). Unsure what xterm does with the margins in a stream; the VT510 manual's wording is not at hand |
| 157 to 160 | DECCRA with the margins on | pass |  |
| 161 to 163 | DECERA and DECFRA with the margins on | pass |  |
| 164 | DECRARA with the margins on | pass |  |
| 165 | DECRARA, stream extent, margins on | fail: stream extent does not wrap at the margins | as 156 |
| 166 to 169 | DECSERA with the margins on | pass |  |
| 170 to 172 | ANSI and DEC mode reports (the VT420 menu) | pass | as 052 to 054 |
| 173 to 178 | DECRQSS: DECELF, DECLFKC, DECSACE, DECSMKR, DECSLRM, DECSNLS | pass | valid, with the current values |
| 179 to 181 | DSR printer, user-defined key and keyboard status | pass | as 001 to 004 |
| 182 to 185 | macro space (DECMSR), memory checksum (DECCKSR), data integrity, multiple session status | pass | `CSI 0 * {`, `DCS 1 ! ~ 0000 ST`, `CSI ? 70 n`, `CSI ? 83 n` |
| 186 | checksum of a rectangle (DECRQCRA), GL characters | fail: not the VT420 checksum | every character's checksum is highlighted as mismatching: vttest expects the two's complement of the sum with the video attributes added (`CHK(n) = -n`), the engine answers xterm's XTCHECKSUM variant on purpose: the plain sum of the code points, no attributes (`checksum.rs`; esctest2 reads the screen with it). `All: 4273` against `EAED` expected |
| 187 | checksum of a rectangle, GR characters | fail: not the VT420 checksum | as 186; the 8-bit characters are also U+FFFD (as 003) |
| 188 | extended cursor position | pass |  |
| 189 to 191 | select number of lines per screen (DECSNLS): 36, 48, 24 | pass | the engine is 80x36, 80x48, 80x24; DECRQSS answers `36 *|`, `48 *|`, `24 *|` |
| 192 to 200 | VT520 cursor movement: HPA, CBT, CHA, CHT, HPR, VPA, CNL, CPL, VPR | pass | boxes closed, rulers and numbers in sequence (CNL numbers 1 to 19, CPL 1 to 20) |
| 201 to 209 | the same with origin mode and margins | pass | inside the margins; with origin mode on, CPL from row 24 starts at the bottom margin |
| 210 to 220 | DECRQSS for the VT520's settings: DECATC, DECAC, DECCRTST, DECDLDA, DECSEST, DECARR, DECSTGLT, DECSZS, DECSPMA, DECTME, DECUS | unsupported: DECRQSS for these settings | eleven invalid requests |
| 221 to 222 | no clear on column change (DECNCSM) off: 80 and 132 columns | pass | the screen is cleared |
| 223 to 224 | DECNCSM on | pass | not cleared (`+NO_CLEAR_ON_COLUMNS`); the 80 columns of E's are kept in 132 |
| 225 | DECNCSM off again | pass | cleared |
| 226 to 233 | cursor styles (DECSCUSR 0 to 6, and back) | pass | the headers show the style the program chose: block blinking, default, block steady, underline blinking, underline steady, beam blinking, beam steady, block steady |
| 234 | alternate text colour (DECATC) display | pass | the sixteen attribute combinations are drawn (bold, reverse, underline, blink); no colours were assigned, so DECATC itself is not exercised (DECRQSS 210 is invalid) |
| 235 to 243 | ISO 6429 cursor movement (HPA, CBT, CHA, CHT, HPR, VPA, CNL, CPL, VPR) | pass | identical to 192 to 200 |
| 244 | colour test pattern: 64 foreground and background combinations, bright off and on | pass | every cell carries its pair (`fg=0 bg=0` ... `fg=7 bg=7`); bright is bold |
| 245 | SGR 0 resets the colours | pass | black on black and white on white are shown as such |
| 246 to 249 | background colour erase (BCE) with ED, EL, ECH and indexing; blue then black | pass | every cell of the screen has the background (`bg=4`, then `bg=0`), the box in yellow |
| 250 to 284 | the cursor movement, screen features and insert/delete tests again with colours on (BCE) | pass | the text of every screen is identical to the uncoloured run (menus 1, 2 and 8) and the colours are carried. Not every cell has the blue background: the cells DECALN fills with E (250, 251) and the rows DECCOLM clears or the test never writes (252, 253, 255, 267, 268) keep the default; vttest says "no cells with the default background" only for 246 and 248. Unsure what xterm gives those |
| 285 | protected areas as ISO 6429 guards (SPA, EPA), colours on | unsupported: SPA/EPA | vttest: "There may be a solid box"; there is none |
| 286 | repeat (REP), colours on | fail: REP repeats after a control sequence | as 301 |
| 287 to 290 | scroll down, left, right, up (SD, SL, SR, SU), colours on | pass |  |
| 291 to 294 | scrolling regions (the menu 2 tests) with colours | pass |  |
| 295 to 296 | graphic rendition pattern with colours, then in reverse video | pass |  |
| 297 to 299 | graphic rendition pattern with the ISO 6429 codes SGR 22 to 27, 39, 49 and 8 | pass | hidden text, default colours and the resets are carried; row 20 intentionally has no background |
| 300 | protected areas as ISO 6429 guards (SPA, EPA) | unsupported: SPA/EPA | vttest says "may"; no box (DEC terminals do not implement ERM either) |
| 301 | repeat (REP) | fail: REP repeats after a control sequence | vttest: a diagonal of 2 `+`; each row shows 12. The second `CSI 10 b` right after a REP should be ignored ("since a control sequence precedes", xterm forgets the character); the engine repeats the `+` ten more times |
| 302 to 305 | scroll down, left, right, up (SD, SL, SR, SU) | pass | a row (SD, SU) or a column (SL, SR) of `*` where vttest says |
| 306 | report version (XTVERSION) | pass | both forms answer `Wormhole 1.1.0` |
| 307 to 310 | xterm's DECRPM list | pass | states match; xterm-only modes the engine has not (tek4014 38, toolbar 10, logging 46, fast scroll 1014, `XT_ALTS_OK` 1046, rxvt 2001 to 2006 but 2004) say `unknown` |
| 311 to 318 | DECRQSS for xterm's key modifier options and title modes | unsupported: DECRQSS (modifyKeyboard, modifyCursorKeys, modifyFunctionKeys, modifyOtherKeys, title modes) | eight invalid requests |
| 319 | title stack position (XTTITLEPOS) | unsupported: XTTITLEPOS | no report |
| 320 | set window title (OSC 0) | pass | the host typed a title; nothing to see on the screen |
| 321 to 322 | modify font (OSC 50) | unsupported: OSC 50 (font) | a window feature: nothing to show |
| 323 | report fonts (OSC 50 query) | unsupported: OSC 50 (font) | no reply: the lists are empty |
| 324 to 332 | alternate screens, modes 47, 1047, 1049 | pass | the screen switches (`+ALT_SCREEN`) and the original is restored, cursor save and restore `ok` |
| 333 to 341 | window modify operations (XTWINOPS: move, resize in pixels and in characters) | pass | played by the fake window of `window_host`: the size follows each request, up to what it allows (54 rows, 240 columns) |
| 342 to 343 | lower and raise the window | pass | nothing to show |
| 344 to 345 | window report operations (XTWINOPS 11, 13, 14, 16, 18, 19, 20, 21) | pass | all ten `OK` |

Not run: 11.1.2.1 send/receive mode (SRM) waits for keys to show local echo; 11.8.5.3 to 11.8.5.8 the mouse tests wait for clicks;
11.8.6 the Tektronix 4014 tests (DECSET 38 opens xterm's second window; the engine has no Tek mode, and the tests wait for keys in it).
Entries vttest has not written (marked `*` on its menus) are in `../notes.txt` with the entries the later models' menus repeat.
