# Input

Output is what the program prints. **Input** is what the host sends back: keys, mouse reports, pasted
text and focus changes. The engine has the modes that decide how those are encoded, so it also has the
encoders. They are the `input` module.

## Pure functions of the event and the modes

An encoder takes an event and the modes the program switched on (`InputModes`, made from the terminal's
`Modes`) and appends the bytes to write to the PTY to a `Vec<u8>` you keep, returning whether there were
any: nothing is written when the modes say the program does not want that event.

```rust
use std::io::{self, Write};
use wormhole_vt::input::{InputModes, Key, KeyEvent, Mods, encode_key};
use wormhole_vt::Terminal;

fn send_key(term: &Terminal, pty: &mut impl Write, event: &KeyEvent) -> io::Result<()> {
    let modes = InputModes::from(term.modes()); // read when the event happens
    let mut bytes = Vec::new(); // keep this between events
    if encode_key(event, &modes, &mut bytes) {
        pty.write_all(&bytes)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut term = Terminal::new(80, 24);
    let mut pty = Vec::new();

    // Ctrl+Up, to a program that has not asked for anything special.
    let event = KeyEvent::new(Key::Up).with_mods(Mods::CTRL);
    send_key(&term, &mut pty, &event)?;
    assert_eq!(pty, b"\x1b[1;5A");

    // The program turns on application cursor keys (DECCKM): the plain key changes.
    term.feed(b"\x1b[?1h");
    pty.clear();
    send_key(&term, &mut pty, &KeyEvent::new(Key::Up))?;
    assert_eq!(pty, b"\x1bOA");
    Ok(())
}
```

They hold no state and need no terminal; calling them takes no lock beyond the one you used to read
`modes()`. Read the modes when the event happens, because a program can change them at any time (a
program turns the kitty keyboard protocol on when it starts and off when it quits). Encoding allocates
nothing once the buffer has grown.

Nothing here touches the PTY. You decide what to do when an encoder returns `false`: usually nothing. A
key that writes nothing may be a shortcut for your application (Cmd on macOS) or a key with no text and
no code.

## Keys

A key event says which key, which modifiers, and the text the key produced with the active keyboard
layout. Keep these apart: the key is a physical or logical key (`Up`, `F5`, `a`), the text is what the
layout makes of it (`å` for Option+A on a US layout). The encoders need both.

### The legacy encoding

Without any kitty flag set, keys follow xterm:

- Printable keys send their text. Control combinations send C0 bytes (`Ctrl+A` is `0x01`), and Meta
  (Alt, or Option when you set it up that way) sends `ESC` first. Whether Option is Meta is a setting of yours: when it is not, Option+L on a
  German layout sends `@`, as the layout composes it.
- Arrow keys and Home/End send `CSI A`, `B`, `C`, `D`, `H`, `F`, or `SS3 A`... when the program set
  application cursor keys (mode 1, DECCKM). Insert, Delete, Page Up and Page Down send `CSI n ~`.
  Function keys: F1 to F4 send `SS3 P` to `S`, F5 and up send `CSI n ~`.
- Modifiers on those keys go in a parameter: `CSI 1 ; m A`, where `m = 1 + shift + 2·alt + 4·ctrl`.
- Backspace sends `DEL` (`0x7f`), and Ctrl+Backspace `BS`; a program that set DECBKM (mode 67) gets them
  the other way round. Shift+Tab sends `CSI Z`.
- Return (and the keypad's Enter, unless the program set the application keypad) sends `CR`, or `CR LF`
  while the program has new line mode set (LNM, `CSI 20 h`; `InputModes::new_line`). The VT52 has no such
  mode.
- DECKPM (key position mode, `CSI ? 81 h`, where a VT510's keys report their position instead of their
  character) is not implemented: it is not one of xterm's modes either, no program asks for it, and the
  engine answers DECRQM for it as an unknown mode.

### modifyOtherKeys

A program that wants to tell Ctrl+I from Tab, or Alt+x from a typed `x`, without the kitty protocol
can ask for xterm's modifyOtherKeys with `CSI > 4 ; 1 m` or `CSI > 4 ; 2 m` (and `CSI > 4 ; 0 m`, or
`CSI > 4 m`, to turn it off; `CSI ? 4 m` asks and the engine answers `CSI > 4 ; Pv m`). `Modes` carries
the level (`MODIFY_OTHER_KEYS`, `MODIFY_OTHER_KEYS_ALL`, read together as `modify_other_keys()`), and
`InputModes::from(modes)` hands it to the encoder, which then writes `CSI 27 ; modifiers ; code ~`
(xterm's `formatOtherKeys` 0) for ordinary keys with a modifier held; the modifier is
`1 + shift + 2·alt + 4·ctrl + 8·meta` and the code is the character the key types (`A`, 65, for
Shift+a). The rules follow xterm's table of modified keys:

- **Level 1**: Shift alone still types the shifted character, and Control on a key that has a control
  code (letters, Space, `@ [ \ ] ^ _ /`, the digits 2 to 8, `~`) still sends it. Alt and Meta on any
  key, Control on the other characters (`! ? - 0 1 9`, …), Enter and Tab with any modifier (but Shift+Tab
  stays `CSI Z`) and Alt+Escape are sequences.
- **Level 2**: every key with a modifier is a sequence, Shift+Tab (`CSI 27 ; 2 ; 9 ~`) and Shift+letters
  among them. xterm's table keeps Control+Backspace as BS and the symbol Shift types on a digit.
- Cursor, function and keypad keys are not "other" keys and are not changed. xterm's level 3 and the
  modifier mask of `CSI > 4 : Pm m` are not implemented; the other resources of XTMODKEYS (modifyCursorKeys
  and friends) are neither kept nor answered.
- The kitty flags take precedence when any is set. DECSTR and RIS reset the level.

A host that rewrites keys before it calls the encoder (Shift+Enter as Alt+Enter, for programs that do not
know better) should not do so for a program that asked for modifyOtherKeys: it asked to be told.

### VT52 mode

A program can put the terminal in VT52 mode with `CSI ? 2 l` (DECANM reset) and leave it with `ESC <`.
`Modes::VT52` says it is on, and `InputModes` carries it: the encoder then sends the VT52's keys. The
cursor keys are `ESC A` to `ESC D` (DECCKM makes no difference), F1 to F4 are `ESC P` to `ESC S` (the
VT52's PF keys), the keypad sends its characters or, after `ESC =`, `ESC ? p` to `ESC ? y` and the
other letters of its table (xterm's ctlseqs, "VT52-Style Function Keys"). Characters, Enter, Tab,
Backspace and Escape are as in the legacy encoding without the `CSI` forms. Keys that exist only as
`CSI` sequences (Home, End, Insert, Delete, Page Up and Down, F5 and up) send nothing, and so do the
mouse, focus and paste reports, which need a `CSI` (pasted text is sent without markers).

### The kitty keyboard protocol

A program asks for it with `CSI > flags u`. The five flags are *disambiguate escape codes* (1), *report
event types* (2), *report alternate keys* (4), *report all keys as escape codes* (8) and *report
associated text* (16). The engine keeps a stack of flags per screen, so a program that leaves the
alternate screen without popping does not leave its flags on the main screen. The stack is 32 deep; a
push on a full stack drops the oldest entry.

With flag 1 or more set, the encoder switches to `CSI code ; modifiers u` (or the `~` and letter forms
for keys that have them) so that `Escape`, `Ctrl+I` and `Tab` are no longer the same byte. Even then, some
keys stay in their legacy form until the program asks for everything: an unmodified Enter, Tab and
Backspace stay `\r`, `\t` and `0x7f`, so a shell is still usable after a program crashes without
resetting the flags.

Flag 2 makes press, repeat and release distinguishable (`:1`, `:2`, `:3` after the modifiers). Your toolkit
has to deliver repeats and releases for this to mean anything.

`Modes` carries the flags in force (`KEYS_*`). `CSI ? u` asks for them, and the engine answers that itself.

## Mouse

A program turns the mouse on with a mode: 1000 (clicks), 1002 (clicks and drags) or 1003 (all motion).
It picks an encoding with 1005 (UTF-8), 1006 (SGR) or nothing (the classic X10 form).

`encode_mouse` takes a `MouseEvent`: the button, the kind (press, release, motion), the cell (0-based
column and row of the visible screen) and the modifiers. It appends nothing, and returns `false`:

- when no mouse mode is on, when the mode does not report this event (motion with a button held
  needs 1002 or 1003; motion with none needs 1003), or when the event has no encoding (a wheel release).

Otherwise it writes:

- `CSI < b ; x ; y M` or `m` in SGR mode. This is the one to rely on: it has no coordinate limit and it
  says which button was released.
- In the classic form, `CSI M` and three bytes. Coordinates go up to 223; farther cells cannot be
  expressed and the event is dropped. A release cannot say which button went up. UTF-8 mode raises the
  limit to 2015.

```rust
use wormhole_vt::Terminal;
use wormhole_vt::input::{InputModes, MouseButton, MouseEvent, MouseKind, encode_mouse};

let mut term = Terminal::new(80, 24);
let mut out = Vec::new();
let click = MouseEvent::new(MouseButton::Left, MouseKind::Press, 9, 4); // column 9, row 4

// Nobody asked for the mouse: it is yours.
assert!(!encode_mouse(&click, &InputModes::from(term.modes()), &mut out));

// The program asks for clicks (1000) in the SGR encoding (1006).
term.feed(b"\x1b[?1000h\x1b[?1006h");
assert!(encode_mouse(&click, &InputModes::from(term.modes()), &mut out));
assert_eq!(out, b"\x1b[<0;10;5M"); // SGR counts from 1
```

Wheel buttons are 64 to 67 (up, down, left, right). Motion adds 32. Shift, Alt and Control add 4, 8 and
16. Many hosts keep Shift for their own selection, so that a program that grabs the mouse can still be
selected from; that choice is yours, not the encoder's.

**SGR with pixels (1016).** The report has the form of SGR's with the pointer's position in pixels, for
the program that wants more than a cell. Give the event the pixel inside the text area, counted from 0
(`MouseEvent::with_pixel(x, y)`); the engine writes it 1-based, like every number of the SGR family, as xterm
does (its `EmitMousePosition` adds 1 to the cell, the row and the pixels alike). kitty writes the 0-based
pixel, a difference of one that only shows at the edge of a cell. An event without a pixel is reported
in cells.

When the program has not turned on a mouse mode, the mouse is yours: select text, scroll the history.
On the alternate screen with mode 1007 (alternate scroll) set and no mouse mode, a host sends arrow keys
for the wheel instead of scrolling history, because there is none.

**The DEC locator.** A program can ask for the pointer as a DEC locator (DECELR) instead. The two are
alternatives: switching on one switches off the other. Tell the engine about the pointer with
`locator_moved` and `locator_button`, and it queues the report as a reply.

## Paste

When the program set bracketed paste (mode 2004, `Modes::BRACKETED_PASTE`), pasted text is wrapped:

```text
ESC [ 200 ~  text  ESC [ 201 ~
```

so the program can tell typed input from pasted input (and a shell can show it without running it).
`input::paste(text, &modes, &mut out)` does the wrapping, and it **removes what could end the bracket
early** from the text: ESC (so the end marker `ESC [ 201 ~` loses its first byte) and the C1 controls
U+0080 to U+009F (U+009B is CSI to a terminal in 8-bit mode). Without that, pasted text could contain
`ESC [ 201 ~` and the rest of it would arrive as typed input. Line breaks (CRLF or LF) are sent as CR,
the byte Return sends, in both modes.

```rust
use wormhole_vt::Terminal;
use wormhole_vt::input::{InputModes, paste};

let mut term = Terminal::new(80, 24);
let mut out = Vec::new();

// Without bracketed paste the text is sent as it is, a line break as Return.
paste("ls\n", &InputModes::from(term.modes()), &mut out);
assert_eq!(out, b"ls\r");

// With it, wrapped, and a smuggled end marker cannot close the bracket early.
term.feed(b"\x1b[?2004h");
out.clear();
paste("a\x1b[201~b", &InputModes::from(term.modes()), &mut out);
assert_eq!(out, b"\x1b[200~a[201~b\x1b[201~");
```

Without bracketed paste the text is sent as it is, and each line break then runs as Return. If you
paste multiple lines into a program that did not ask for bracketed paste, consider asking the user first.

The encoder does not decide what to paste. If your clipboard text has control characters you do not want
to send, filter it before you call the encoder.

## Focus

If the program set mode 1004, `input::focus(focused, &modes, &mut out)` writes `CSI I` for focus in and
`CSI O` for focus out and returns `true`; otherwise it writes nothing. A program can use it to redraw
or to pause. Send it whenever the window's or the pane's focus changes, not only the window's.

```rust
use wormhole_vt::Terminal;
use wormhole_vt::input::{InputModes, focus};

let mut term = Terminal::new(80, 24);
let mut out = Vec::new();
assert!(!focus(true, &InputModes::from(term.modes()), &mut out));

term.feed(b"\x1b[?1004h");
assert!(focus(true, &InputModes::from(term.modes()), &mut out));
assert!(focus(false, &InputModes::from(term.modes()), &mut out));
assert_eq!(out, b"\x1b[I\x1b[O");
```

## Application keypad and other modes

Mode 66 (application keypad) and its older forms (`ESC =`, `ESC >`) change what the keypad's keys send.
Build the encoders' `InputModes` with `InputModes::from(term.modes())` (or from `&Modes`); you do not need
to track the mode yourself. Modes that only exist in the
VT510's hardware set-up (keyboard usage, key position) are recorded and reported by DECRQM but have no
effect on the encoders.
