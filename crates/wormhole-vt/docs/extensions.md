# Your own sequences

Programs invent sequences, and so do hosts. A shell prompt wants to say what command ran, a build tool
wants to show a progress bar, an application wants to report its own state. The engine reads every
sequence it knows and drops the rest. **Passthrough** is how a host asks to be given the ones it
wants: the engine collects the string whole and hands it over as an event. The engine does not run
the host's code, and it does not interpret the string.

## Asking

A [`Passthrough`](crate::passthrough::Passthrough) lists what the host wants and how much of it:

```rust,ignore
use wormhole_vt::passthrough::Passthrough;
use wormhole_vt::Terminal;

let term = Terminal::builder()
    .passthrough(
        Passthrough::new()
            .osc(51234, 1024)        // OSC 51234 ; payload ST, up to 1 KiB of payload
            .dcs(b"!", b'z', 64 * 1024) // DCS params ! z data ST, up to 64 KiB of data
            .apc(b'Q', 4096),        // APC Q... ST, up to 4 KiB, the Q included
    )?
    .build();
```

`Terminal::set_passthrough` changes the choice while the terminal runs. A refused value
(`PassthroughError`) changes nothing.

Each claim carries its **limit**. A string over its limit is **dropped whole**: nothing is queued and
the string after it is read as usual. A string is never cut, because half a message is worse than none
and the host would have to guess whether it was cut. The limit is part of the claim so that a reviewer
can see in one line how much memory a program can make the terminal hold for it. Choose the smallest
number that works: titles take 8 KiB in the engine, an image protocol takes megabytes, a status message a
few hundred bytes. No limit goes above `MAX_BYTES` (1 MiB); a larger one is clamped.

An unclaimed string is never collected. A claim costs memory only while a claimed string is arriving.

## Reading

The strings arrive in the event stream, in order with the engine's own events:

- `Event::Osc { number, payload, terminator, replies_before }`: `OSC number ; payload`, the payload being
  what follows the first `;` (empty without one).
- `Event::Dcs { params, intermediates, final_byte, payload, replies_before }`: the header of the DCS and
  its data. A DCS with a private marker (`DCS = …`) is not matched.
- `Event::Apc { payload, replies_before }`: the whole string, its first byte included (the byte that was
  claimed), so one handler can tell its claims apart.

The payload is what a program printed. Validate it, bound anything you build from it, and never use it as
a path, a command or a format string.

## Replying, in order

A program that asks a question reads its answers in the order it asked. Your answer may come later, from
another thread, but it still has to be written to the program's input *between* the engine's replies to
what came before it and to what came after. `replies_before` is how many bytes of
`Terminal::replies()` the engine had queued when the string ended, exactly as `Event::ClipboardLoad`
carries it. A host that answers at once writes `replies()[..replies_before]`, then its answer, then the
rest. A host that answers later holds the place (as a host does while it asks the user about a clipboard read). The reply ends the
way the request did (`Terminator::as_str`).

```rust,ignore
for event in term.events() {
    if let Event::Osc { number: 51234, payload, terminator, replies_before } = event {
        if payload == b"ping" {
            let answer = format!("\x1b]51234;pong{}", terminator.as_str());
            input.splice(replies_before..replies_before, answer.into_bytes());
        }
    }
}
```

`examples/custom_osc.rs` is this, whole. Never build the answer from the request's bytes: a reply that
contains what the program sent is the oldest bug in terminal security (`security.md`). Put a number or a
fixed word in it.

## Why the host's code does not run in `feed`

The engine does not call back into the host when a claimed string ends; it queues an event. There are
three reasons.

- **No foreign code in the parser.** A handler running inside `feed` would run on the parsing path, under
  the host's lock, and could be slow, block, take a lock of its own or panic. With events, the engine's
  time per byte is the engine's own, and a panic in the host's handling is the host's, outside the
  parser.
- **No waiting.** A handler that needs the host (a clipboard, a prompt, a network call) would have to
  stall the parser, because the bytes behind the string may depend on the answer, and a stalled parser
  stalls the terminal. An event never makes the engine wait; the host answers when it can, at the place
  `replies_before` names.
- **Nothing to get wrong about the screen.** A handler could reach into the engine's state in the middle
  of a sequence. An event is data. The host reads the screen from the terminal as it always does, between
  calls.

The price is that the host cannot change how the engine treats a sequence, only get a copy of one the
engine would have dropped. That is deliberate.

## Why the engine's own sequences cannot be claimed

A built-in stays built-in. Claiming OSC 52 would let a host replace the clipboard policy with one of its
own, and then every embedder would have to know whose OSC 52 it is talking to; a claim on the title would
let a bug in a host's handler change what a terminal does for every program. If you want to see the
clipboard or the title, the engine already queues an event for it (`Event::ClipboardStore`,
`Event::ClipboardLoad`, `Event::Title`), and if you want to refuse one, `Policy` is the place.

So a `PassthroughError` is returned for:

- the OSC numbers the engine acts on: 0, 1, 2, 4, 5, 7, 8, 9, 10 to 19, 52, 99, 104, 105, 110 to 119,
  133, 777, 1337 and 7501 (the [program status
  protocol](https://mitchellh.com/writing/program-status-osc7501), which arrives as
  `Event::ProgramStatus`);
- the DCS strings it streams into decoders and tables it cannot replay to a handler: Sixel (`q`),
  DECRQSS (`$ q`), XTGETTCAP (`+ q`), DECAUPSS (`! u`), DECRSPS (`$ t`) and DECRSTS (`$ p`);
- the kitty graphics protocol (`APC G`);
- numbers and bytes that no sequence can carry (an OSC number of more than five digits, an APC that
  starts with a control).

Control sequences (CSI) cannot be claimed at all: their parameters are already parsed and a host that
needs one can add it to the engine, with a test.

## Rules for choosing a sequence

- **Pick numbers carefully.** OSC numbers are shared by every terminal. Use a private-use style number
  nobody uses (the examples use 51234) and document it, or use a published one and implement its spec.
- **Return the same answer to the same question.** A program that probes for a feature should get a
  reply whose shape does not depend on what it sent.
- **Do the work outside `feed`.** Handle the event after you release the terminal's lock.
