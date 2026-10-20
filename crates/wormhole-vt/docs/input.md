<!--
API names assumed here (the `wormhole_vt::input` module, D-12, built in phase 2a; today these live in
wormhole-term's input module as keys::{KeyInput, encode, focus_report} and mouse::{MouseInput, MouseButton,
MouseAction, encode}): input::{KeyEvent, Key, Modifiers, MouseEvent, MouseButton, MouseAction},
input::{encode_key, encode_mouse, encode_paste, encode_focus} as pure functions of (event, Modes) returning
Option<Vec<u8>> (or writing to &mut Vec<u8>); Modes from Terminal::modes(); Terminal::{locator_moved,
locator_button}.
-->

# Input

Output is what the program prints. **Input** is what the host sends back: keys, mouse reports, pasted
text and focus changes. The engine has the modes that decide how those are encoded, so it also has the
encoders. They are the `input` module.

## Pure functions of the event and the modes

An encoder takes an event and the terminal's `Modes` and returns the bytes to write to the PTY, or
nothing when the modes say the program does not want that event.

```rust,ignore
use wormhole_vt::input::{self, KeyEvent, Key, Modifiers};

let modes = term.modes();

let event = KeyEvent::new(Key::Up).with_modifiers(Modifiers::CONTROL);
if let Some(bytes) = input::encode_key(&event, modes) {
    pty.write_all(&bytes)?;
}
```

They hold no state and need no terminal; calling them takes no lock beyond the one you used to read
`modes()`. Read the modes when the event happens, because a program can change them at any time (an
editor turns on the kitty keyboard protocol when it starts and off when it quits).

Nothing here touches the PTY. You decide what to do with `None`: usually nothing. A key that returns
`None` may be a shortcut for your application (Cmd on macOS) or a key with no text and no code.

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

`encode_mouse` takes the button, the action (press, release, motion), the cell (0-based column and row of
the visible screen), and the modifiers. It returns:

- `None` when no mouse mode is on, when the mode does not report this event (motion with a button held
  needs 1002 or 1003; motion with none needs 1003), or when the event has no encoding (a wheel release).
- `CSI < b ; x ; y M` or `m` in SGR mode. This is the one to rely on: it has no coordinate limit and it
  says which button was released.
- In the classic form, `CSI M` and three bytes. Coordinates go up to 223; farther cells cannot be
  expressed and the event is dropped. A release cannot say which button went up. UTF-8 mode raises the
  limit to 2015.

Wheel buttons are 64 to 67 (up, down, left, right). Motion adds 32. Shift, Alt and Control add 4, 8 and
16. Many hosts keep Shift for their own selection, so that a program that grabs the mouse can still be
selected from; that choice is yours, not the encoder's.

**SGR with pixels (1016).** The report has the form of SGR's with the pointer's position in pixels, for
the program that wants more than a cell. Give the event the pixel inside the text area, counted from 0
(`MouseEvent::with_pixel`); the engine writes it 1-based, like every number of the SGR family, as xterm
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
`encode_paste` does the wrapping, and it **removes the end marker from the text**. Without that, pasted
text could contain `ESC [ 201 ~` and end the bracket early, so that the rest of the text arrives as
typed input.

Without bracketed paste the text is sent as it is. Newlines in it then run as Enter. If you paste
multiple lines into a program that did not ask for bracketed paste, consider asking the user first.

The encoder does not decide what to paste. If your clipboard text has control characters you do not want
to send, filter it before you call the encoder.

## Focus

If the program set mode 1004, `encode_focus(focused, modes)` returns `CSI I` for focus in and `CSI O` for
focus out; otherwise `None`. Editors use it to refresh buffers and to pause. Send it whenever the window's
or the pane's focus changes, not only the window's.

## Application keypad and other modes

Mode 66 (application keypad) and its older forms (`ESC =`, `ESC >`) change what the keypad's keys send.
Build the encoders' `InputModes` with `InputModes::from(term.modes())` (or from `&Modes`); you do not need
to track the mode yourself. Modes that only exist in the
VT510's hardware set-up (keyboard usage, key position) are recorded and reported by DECRQM but have no
effect on the encoders.
