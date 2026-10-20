<!--
API names assumed here: Terminal::{new, builder, feed, take_replies, replies, clear_replies, events, resize,
set_scrollback_lines, set_cell_size, set_window, synchronized_output, clipboard_reply, image_decoded},
Builder::{size, scrollback_lines, policy, cell_size}, Policy, Event variants, Terminator.
-->

# Embedding

The engine is the middle of a terminal. The host supplies everything around it: a PTY (or a pipe, a
socket or a file), threads, a clipboard, a window and a renderer. This chapter shows the loop and the
decisions that are yours.

## The loop

```text
read bytes from the PTY --> feed --> write replies to the PTY --> handle events --> draw
```

```rust,ignore
use wormhole_vt::{Event, Terminal};

let mut term = Terminal::builder()
    .size(cols, rows)
    .scrollback_lines(10_000)
    .cell_size(cell_w, cell_h) // pixels; used for a few reports and for pictures
    .build();

let mut buf = [0u8; 16 * 1024];
loop {
    let n = pty.read(&mut buf)?;
    if n == 0 {
        break;
    }
    term.feed(&buf[..n]);

    // 1. Answer the program. Do this before anything slow: a program may be blocked on it.
    let replies = term.take_replies();
    if !replies.is_empty() {
        pty.write_all(&replies)?;
    }

    // 2. Do what the program asked for.
    for event in term.events() {
        match event {
            Event::Bell => ring_bell(),
            Event::Title => set_window_title(term.title()),
            Event::ClipboardStore { text, .. } => clipboard.set(text), // after your own checks
            _ => {} // Event is #[non_exhaustive]
        }
    }

    // 3. Draw, if something changed.
}
```

Points that are easy to get wrong:

- **Write the replies even when you do not draw.** A program that asks for the cursor position and gets
  no answer waits, or times out after seconds.
- **Collect events.** The queues are bounded, so an ignored queue does not eat memory, but an event you
  never read is a request you never answered. Pictures and clipboard reads in particular stay open
  until the host answers.
- **Keep one terminal per PTY.** The terminal holds the state of one program's output. A second PTY
  needs a second `Terminal`.
- **Feed everything, in order.** Never drop or reorder bytes, and never feed a sequence's bytes to two
  terminals. You may cut the stream anywhere (read sizes do not matter).

### Answers that wait for the host

Three requests cannot be answered by the engine alone:

| Event | The host | If it never does |
|---|---|---|
| `ClipboardLoad` (OSC 52 read) | Writes the replies owed before the request (`replies_before` bytes of `replies()`), then the answer from `clipboard_reply(selection, text, terminator)` | Nothing is sent. The program gets no answer |
| `ImageData` (a picture to decode) | Calls `image_decoded(handle, Ok(()))` or `Err(..)`. The engine holds later replies behind the answer, so the program reads them in order | After 1 MiB of held replies, or 64 waiting answers, the oldest waiting answer is given up |
| `ColumnsRequested`, `WindowOp` | Resizes the window and calls `resize`, or calls `resize` with the size it has | The engine keeps the size the sequence gave it, which may not match your window |

For a clipboard read the order is yours to keep: the program expects its answers in the order it asked, so
write the bytes it was owed before the read first, then the clipboard's answer.

Answer an `ImageData` with exactly one call, in any order. Decoding is the host's job because the engine
has no image decoder; a decode failure is reported with `ImageError`. If the host decides a clipboard
read is not allowed, it does not call `clipboard_reply`; see `security.md`.

## Threads and locking

The engine has no locks, no threads and no global state. `Terminal` is `Send`: build it on one thread
and move it to another. Two threads that touch it at once need a lock of yours.

The usual shape is a reader thread that owns the PTY and a UI thread that draws:

```text
reader thread:  lock; feed a slice; take replies and events; unlock; write replies
UI thread:      lock; copy what changed into its own buffers; unlock; draw from the copies
```

Rules of thumb:

- **Keep the lock short.** `feed` costs time in proportion to the bytes. Feed in slices (Wormhole feeds
  16 KiB at a time) so that the UI never waits behind a flood.
- **Use a fair lock** if the reader is busy. A plain `Mutex` can let the reader take the lock again and
  again while the UI thread starves. Wormhole uses a lock that hands over in the order threads asked.
- **Copy under the lock, draw outside it.** The engine's row copy writes into buffers you keep, so the
  lock is held for the copy and not for the drawing (`rendering.md`).
- **Write to the PTY outside the lock.** A full PTY buffer can block the write; do not hold the
  terminal's lock while it does.
- Do not share a `Terminal` between a reader and a writer without thinking about replies: `take_replies`
  returns the bytes and empties the queue, so only one thread should call it.

The same loop works single-threaded, in an async runtime (feed from the task that reads) or in a test
with a byte slice instead of a PTY.

## Resizing

Call `Terminal::resize(cols, rows)` whenever the window's cell grid changes, and set the PTY's window
size (`TIOCSWINSZ`) so the program gets `SIGWINCH`. The order does not matter much, but do both.

What happens:

- The size is clamped: 2 to 16,384 columns and 1 to 16,384 rows. Calling with the current size does
  nothing.
- **The main screen is reflowed.** Soft-wrapped lines are rewrapped to the new width, the cursor and the
  pins move with their text, and history is rewrapped too. Hard line breaks stay.
- **The alternate screen is cropped or padded.** Full-screen programs redraw after `SIGWINCH`.
- A resize ends any scrolling region and left/right margins (the screen they were set on is gone) and
  gives new columns the default tab stops.
- All rows count as damaged (`rendering.md`).

DECCOLM (`CSI ? 3 h/l`, 132 and 80 columns) works only when the program has set mode 40. The engine then
takes the new width itself (it clears and homes the screen, unless mode 95 is set) and queues
`Event::ColumnsRequested(n)`. Resize your window to `n` columns, or call `resize` with the size you have to
put the terminal back.

If you allow window operations (`Policy::window_ops`), a program can ask for a size in cells; the engine
takes the size at once (so a program that asks right after gets the new answer) and queues
`Event::WindowOp`. Make the window match, or call `resize` with the size you can give. Tell the engine
where the window is with `set_window` so that XTWINOPS position and size reports are right.

## Scrollback sizing

The history is capped in bytes. `scrollback_lines(n)` asks for at least `n` lines at the current width.
Each cell is 8 bytes and each row record 16, so a line of 80 columns costs about 656 bytes and 10,000
lines about 6.5 MB. The engine drops whole pages (up to 128 KiB each), oldest first, and keeps room for
three pages of slack, so the history can hold a little more or less than `n` lines. Do not rely on the exact
count.

A change in width changes how many lines fit: wider rows are bigger. The cap is recomputed on resize.
`set_scrollback_lines` changes it at runtime and drops what no longer fits at once. The alternate screen
never has history.

Memory is allocated as needed and a dropped page is reused for the next, so a full history does not keep
growing.

## Synchronized output

Mode 2026 lets a program bracket a redraw: `CSI ? 2026 h` begins an update, `CSI ? 2026 l` ends it.
The idea is that the host does not draw half a frame. The older iTerm2 form `DCS = 1 s` and `DCS = 2 s`
sets and clears the same flag.

The engine only records the flag. It never holds output back, and it feeds the screen as bytes come.
`Terminal::synchronized_output()` tells you an update is in progress. What to do is a host decision:

- Skip drawing while it is true, and draw once when it turns false.
- **Give up after a short time.** A program can crash between begin and end, or a hostile one can set the
  mode and never clear it. Pick a limit (100 ms to 1 s is common) and draw anyway when it passes. The
  engine has no timer, so the clock is yours.
- Remember to draw at the end: the end of an update is a reason for a frame, even if no more bytes come.

## Policy

`Policy` holds the switches for the risky requests. The default is the safe one, and you turn on only
what your application needs:

```rust,ignore
use wormhole_vt::{Policy, Terminal};

let mut policy = Policy::default();
policy.window_ops = true;      // moves, resizes
policy.title_report = false;   // reading the window title back (it is typed into the shell)
policy.screen_checksum = false; // DECRQCRA (reading the screen back)

let term = Terminal::builder().size(80, 24).policy(policy).build();
```

`security.md` says what each switch exposes.

## Persistence and attach

To rebuild a terminal elsewhere (a detached session, a replica, a checkpoint), the repaint writer
serializes a terminal's state as VT bytes that a fresh terminal parses into the same state:
`repaint_ready(&parser)` tells whether the terminal is between sequences, and `repaint` writes the bytes.
It reports what VT cannot say (pictures and a few corner cases) as `Gaps`. Counters that no sequence can
set (block numbers, generated link ids, picture handles) are `Seeds` that you apply before parsing the
repaint.
