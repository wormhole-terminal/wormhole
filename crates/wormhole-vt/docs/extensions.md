<!--
THIS CHAPTER DESCRIBES THE API BEING BUILT (phase 3, D-14). Every name below is assumed:
ext::{Extension, Claims, Handled, Context, ExtensionId}, Extension::{claims, osc, dcs, apc, csi_private},
Claims::{osc(number, max_payload), dcs(intermediates, final), apc(introducer, max_payload), csi_private(final)},
Handled::{Yes, Pass}, Context::{reply, emit, cursor, size, pin}, Builder::extension(Box<dyn Extension + Send>),
Terminal::{add_extension, remove_extension}, Event::Custom(..) carrying the extension's own event type.
Reconcile with the real names before release.
-->

# Extensions

> **This chapter describes the API as designed. It is being built.** Names and signatures may differ in
> the first release; the behaviour described here is the contract the implementation is held to.

The terminal speaks a long list of sequences, and programs invent more. A shell prompt wants to say what
command ran. A build tool wants to show a progress bar. An application wants to report its own status. An
**extension** is a way for the host to add its own sequences without forking the engine, and without
giving up the engine's guarantees about hostile input.

## The model

An extension is an object that implements the `Extension` trait. The host registers it on the builder or
at runtime. The extension declares what it wants to see, its **claims**:

- an **OSC number**, with a **payload cap** (how many bytes of payload it accepts);
- a **DCS** or **APC** introducer, with a cap;
- a **private CSI final** (a final byte after a private marker like `?` or `>`).

The engine caches the claims of all extensions in bitsets. When a sequence arrives, one check on the
bitset decides whether any extension wants it. If none does, the sequence takes the engine's normal path
and the extensions cost one branch. This is why the trait can be a trait object (`Box<dyn Extension +
Send>`): only claimed sequences ever call it, so the dynamic call is not on the path of ordinary text.

```rust,ignore
use wormhole_vt::ext::{Claims, Context, Extension, Handled};

/// Our own OSC 7777: the program says what it is working on.
struct WorkingOn;

impl Extension for WorkingOn {
    fn claims(&self) -> Claims {
        Claims::new().osc(7777, 1024) // OSC 7777, payloads up to 1 KiB
    }

    fn osc(&mut self, number: u32, payload: &[u8], cx: &mut Context<'_>) -> Handled {
        if number != 7777 {
            return Handled::Pass;
        }
        let Ok(text) = std::str::from_utf8(payload) else {
            return Handled::Yes; // ours, but malformed: swallow it
        };
        cx.emit(WorkingOnChanged { text: text.to_owned() });
        Handled::Yes
    }
}

let term = Terminal::builder().extension(Box::new(WorkingOn)).build();
```

## Handled and Pass

Extensions form a stack. The **newest registered is asked first**. A handler returns:

- `Handled::Yes`: the extension dealt with the sequence. Nobody else sees it, including the engine.
- `Handled::Pass`: the extension declined. The next older extension that claimed it is asked, and after
  the last one, the built-in handler runs.

So an extension can add a sequence the engine does not know, or take over one it does (for example to
log OSC 8, or to pass OSC 52 through a policy of its own), and it can wrap the built-in behaviour: look,
then `Pass`.

Claims are matched before the handler runs. A claim for a number does not mean the extension must handle
every payload; it may `Pass` on a payload it does not understand.

## Payloads

The parser streams strings in pieces and never buffers them. For a claimed sequence, the engine collects
the payload and delivers it **whole**, up to the claim's cap, in one call. Over the cap, the payload is
**dropped whole and the extension is not called**. A string is never cut in the middle: half a message is
worse than none, and the extension would have to guess whether it was cut.

This is deliberately different from an interface that hands over a stream of chunks (every extension would
need its own state machine) and from one that buffers without a limit (every extension is a memory
exhaustion bug waiting for a hostile program). The cap is part of the claim, so a reviewer can see in one
line how much memory an extension lets a program use. Choose the smallest number that works. Titles use
8 KiB in the engine; an image protocol is megabytes; a status message is a few hundred bytes.

An unclaimed sequence is never collected. A claim costs memory only while a claimed string is arriving.

## Context

The handler gets a narrow `Context`, not the terminal. It can:

- `reply(bytes)`: queue bytes for the program's input. The engine applies its reply cap and ordering
  (`concepts.md`). Build the reply from your own state, never from the payload; see `security.md`.
- `emit(event)`: queue a custom event for the host. It appears in the event stream in order with the
  built-in events. This is how an extension talks to the host's UI.
- `cursor()` and `size()`: where the cursor is and how big the screen is.
- `pin(point)`: pin a position so the host or the extension can refer to that text later, through
  scrolling and reflow (`concepts.md`).

It cannot change cells, modes or the cursor, and it cannot read the screen. An extension is a *reader of
sequences*, not a way into the engine's state. If a feature needs more (decorations over text, for
example), build it on pins and events in the host.

## Why there is no async

Some designs let a handler return a future so it can wait for the host. This engine does not. A handler
that needs the host does this:

1. It emits an event and returns `Handled::Yes`.
2. The host does its work, however long it takes, on its own thread.
3. The host calls back into the terminal with the answer (a reply to write, a state change).

Reasons: the engine has no executor and no lock to hold across an await point. While a handler waits,
the parser would have to stop, because the bytes behind a sequence may depend on it, and a stalled parser
stalls the whole terminal. xterm.js, whose handler model this design follows, lets a handler return a
promise and pauses parsing until it settles; this engine keeps the model and drops that part. With
events, the engine never waits, and the order of replies stays what `concepts.md` says: the order of the
questions.

The clipboard read and the picture decode in the built-in engine work the same way (`embedding.md`).

## Registering and removing

On the builder, extensions are present from the first byte. At runtime, `add_extension` pushes onto the
stack and `remove_extension` takes one off by the id it returned. Claims are recomputed on each change.
Registration is for the host, from the thread that owns the terminal; extensions are `Send` so the terminal
stays `Send`.

## Rules for writing one

- **Treat the payload as hostile.** It is what a program printed. Validate it, bound anything you build
  from it, and never use it as a path, a command or a format string.
- **Never echo it.** A reply that contains request bytes is the oldest bug in terminal security
  (`security.md`). Put a number or a fixed word in the reply.
- **Keep it small.** A handler runs on the parsing path, under the host's lock. Do not do I/O, take
  locks or allocate without need. `emit` and let the host do the work.
- **Return `Pass` when it is not yours.** A claim for a number is a claim for every sequence with that
  number.
- **Pick numbers carefully.** OSC numbers are shared by every terminal. Use a number nobody uses, and
  document it. Wormhole's own program-status protocol (OSC 7501) is the first extension to use this API:
  it lives outside the engine, in the app's terminal layer.
