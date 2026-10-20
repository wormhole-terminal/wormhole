<!--
API names assumed here: Policy { window_ops, title_report, screen_checksum } (today WindowOps::{Deny, Allow} and
ScreenChecksum::{Deny, Allow}), Terminal::{clipboard_reply, image_decoded}, Event variants, input::encode_paste.
Title reports are `Policy::title_report` (D-24), cleaned when the title is set (D-25).
ALSO: OSC 1337 that is not an inline image reaches the host raw as Event::Iterm (cap: 64 MiB). Listed under
"What the host must do"; decide whether the engine should cap it lower or drop it (D-9 "no file/exec sequences").
-->

# Security

A terminal reads whatever a program prints. That includes `cat` of a file from the internet, the banner of
a server you `ssh` into, the output of a build in a repository you just cloned, and text an AI agent was
tricked into printing. All of it is untrusted. This chapter says what the engine guarantees about that
input and what the host still has to do.

## Threat model

**The attacker controls the bytes fed to `Terminal::feed`.** Nothing else. They do not control the host
process, the PTY's other end or the keyboard.

What they want:

| Goal | How a terminal gets it wrong |
|---|---|
| Run a command as the user | The terminal answers a question with text the attacker chose, and the answer lands in the input of a shell (**report injection**). Or a feature that starts programs, opens files or loads fonts |
| Read something | The clipboard, the window title, the screen, a file |
| Take over the window | Move, resize, minimise or full-screen it; raise a notification that looks like something else |
| Hang or crash the host | A panic, an endless loop, or memory that grows without limit |
| Fool the user | Text that looks like a prompt or a dialog; a link whose text differs from its target |

The engine's job is to make the first four hard from inside the engine. The last one cannot be solved in a
terminal: a program may draw anything on the screen. That is why the host must show link targets and
sanitise text it takes out of the terminal.

## What the engine guarantees

### It does not panic

The parser accepts every byte string and always makes progress. Counts are clamped. Positions outside the
screen do nothing. The guarantee is tested, not proven: four fuzz targets (`parser`, `split`, `term` and
`graphics`) run with overflow checks on and check the terminal's invariants after every step, and the
crate's tests run the same checks on generated streams on every build. The crate has no `unsafe` code
(`#![forbid(unsafe_code)]`), so a mistake in arithmetic is a panic or a wrong cell and not a memory
corruption. A panic on any input is a bug: report it (`SECURITY.md`).

### Memory is bounded

A hostile stream cannot make a terminal grow without limit. Strings are kept up to a limit that depends on
what they are. A string over its limit is **dropped whole**: half a clipboard or half an image is worse than
none.

The numbers below are the built-in **ceilings**, which are what a terminal uses unless the host says
otherwise. A host that has no use for them can lower the ones with a name in the last column, with
`Limits` (`Builder::limits`, `Terminal::set_limits`). A value above its ceiling is cut to it, so
lowering is all a host can do and this table stays true whatever it asks. The rest are fixed.

| What | Ceiling | `Limits` field |
|---|---|---|
| Parameters in a CSI | 32 values, each saturating at 65,535; 4 intermediate bytes. A sequence over the limit is reported with an overflow flag | |
| OSC string (titles, folders, colours, links, notifications, marks) | 8 KiB | `osc` |
| OSC 52 (clipboard) | 8 MiB | `clipboard` |
| OSC 1337 (iTerm2): a picture | 64 MiB | `osc_image` |
| OSC 1337 (iTerm2): anything else | 64 KiB | `osc_iterm` |
| Title (and icon name) kept | 4,096 bytes (cut at a character); the title stack is 16 deep | `title` |
| Folder (OSC 7) kept | 4,096 bytes (cut at a character) | `cwd` |
| Hyperlink target | 8 KiB (a longer one is ignored, like one with a control character) | `link_uri` |
| DECRQSS request | 32 bytes | |
| XTGETTCAP request | 4,096 bytes (a capability name is at most 32) | |
| Sixel data | 4 MiB | `sixel` |
| DECRSPS and DECRSTS data | 32 KiB | |
| kitty graphics: control data | 2,048 bytes | |
| kitty graphics: one transmission, and one iTerm2 file | 32 MiB | `image_payload` |
| One picture | 10,000 pixels a side and 16 million pixels | |
| Pictures kept | 1,024 per screen | `image_count` |
| Pictures kept, as RGBA | 128 MiB per screen; least recently used leave first | `image_bytes` |
| Placements | 4,096 per screen | |
| Uploads waiting for the host to decode | 256 MiB in all; past that new pictures are refused | `image_uploads` |
| Replies the host has not collected | 1 MiB; past that new replies are dropped whole | `replies` |
| Events the host has not collected | 16,384; past that new ones are dropped, except picture uploads, picture releases, resets and dropped history | `events` |
| Dropped history lines the host has not collected (only when it asked for them) | 64 MiB of lines; past that the newest are counted, not kept | `archive_backlog` |
| Replies held behind a picture decode | 1 MiB and 64 answers waiting; then the oldest is given up | |
| Screen size | 2 to 16,384 columns, 1 to 16,384 rows | |
| Characters after a cell's first | 32 | |
| Kitty keyboard flag stack | 32 per screen; XTSAVE entries 64 | |
| XTPUSHSGR and XTPUSHCOLORS stacks | 10 entries each (xterm's depth); a push on a full stack does nothing | |
| Scrollback | What the host asks for, in bytes; the oldest page is dropped first | `Builder::scrollback` |

When a host lowers a cap while the terminal runs, it applies to the next sequence: a string being read keeps
the cap it started with, and events, replies and a title already kept stay as they are. Pictures are the one
exception: those held over a lowered count or budget are evicted at once, the least recently used first, and
each is announced with `Event::ImageFree`.

An unfinished string costs nothing beyond the bytes seen so far, and a string whose number the engine
ignores is never stored. A picture-heavy stream can still make a terminal hold a large amount (up to the
picture budgets above, per screen, plus queued uploads; lower them with `Limits` if that is too much); drain events promptly, since uploads leave the
queue when you take the event, and decode with limits of your own.

### Time is bounded

Each sequence costs time proportional to its length and to a count the engine has clamped. A repeat count
(REP) is at most what one parameter can say (65,535). Rectangle operations are cut to the screen. A
hyperlink's reach is followed over at most 16 rows each way. Sixel decoding has a work budget. A
program cannot move or resize the window unless `Policy` allows it, and then only up to the screen. The one
exception is DECCOLM, which changes the engine's width to 80 or 132 columns (see below) and asks the host
to follow.

There are no timers in the engine, so there is nothing a program can make wait.

### Replies are built from state

Every reply is formatted from the engine's own state: the cursor position, a mode's number and value,
the colours, the size. **No reply contains text a program chose**, with two exceptions that the host
controls: title reports, which are behind `Policy` (below), and the clipboard answer, which the host
supplies. The few places a request's bytes appear in an answer are limited:

- An OSC 52 reply repeats the selection name, but only the letters the protocol defines (`c p q s 0`
  to `7`); anything else is dropped, and a missing name is `s0`.
- XTGETTCAP repeats the hex name of a capability only when the name is in the engine's own table.
  Unknown names are answered `DCS 0 + r ST`, with no echo.
- DECRQSS replies are built from state. An unknown request is answered `DCS 0 $ r ST` with no echo.
- A kitty graphics reply carries the numeric ids and a fixed message.
- A request id (the `Pi` in DECRQCRA) is a number.
- XTREPORTSGR answers the rendition the cells of a rectangle have in common, which is attributes and
  colours and never text; XTQMODKEYS, XTREPORTCOLORS and the in-band resize report are numbers.

The reason is the history below: most of those vulnerabilities are an answer that contained attacker
text.

### Title reports

Programs can ask for the window title and the icon name (XTWINOPS 20 and 21). The answer would put text
the previous program chose into the input of the next program. The engine answers only when
`Policy::title_report` is on, a switch of its own that `window_ops` does not turn on. Leave it off unless
you have a reason; the engine's own default is off. Cleaning the controls out is not enough to make a
reported title safe, since the text can still be a command line. Setting the title (OSC 0, 1, 2) is not
restricted: it is an event and a string you can read, and it is cleaned when it is set: C0 and C1
controls and DEL are dropped, as they are from the icon name and the folder of OSC 7.

### Window operations and reading the screen back

| Request | Default | With `Policy` |
|---|---|---|
| Move, resize, minimise, raise, lower, maximise, full-screen (XTWINOPS 1 to 10, DECSLPP, DECSNLS) | Ignored | `window_ops`: queued as `Event::WindowOp`; sizes in cells also take effect in the engine, up to the screen |
| Title and icon name reports (XTWINOPS 20, 21) | Not answered | `title_report`, see above |
| DECCOLM (80/132 columns) | Needs mode 40, which the program sets itself, so `Policy` does not cover it | The engine takes the new width, clears and homes the screen, and queues `Event::ColumnsRequested(n)`. The host makes the window match, or calls `resize` with the size it has to say no |
| DECRQCRA (checksum of a rectangle) | Ignored | `screen_checksum`: answered |
| Window state, position, pixel size, cell size and text-area reports (XTWINOPS 11 to 19) | Always answered | They say nothing the program could not learn from the tty |

DECRQCRA is off because the checksum of one cell tells a program what is in it, so any program, also one
on the far end of an ssh connection, could read the screen back cell by cell. Turn it on only for test
harnesses.

### Clipboard and notifications are requests

The engine has no clipboard and no notification centre. OSC 52 becomes `ClipboardStore` or
`ClipboardLoad`; OSC 9, 99 and 777 become notification events. The text is the program's, as sent. The
engine does not decide anything about them, and a clipboard read is never answered unless the host calls
`clipboard_reply`.

### Sequences that are not implemented, on purpose

No sequence in the engine starts a program, reads or writes a file, or loads a font.

- **OSC 50** (set the font) is not read. An unknown OSC is not even stored.
- **tmux control mode** (`DCS 1000 p`) and every other DCS the engine does not know is read to its
  end and thrown away.
- kitty graphics **files and shared memory** (`t=f`, `t=t`, `t=s`) are refused. Every such failure is
  answered the same way (`EBADF`), so a program learns nothing about files it cannot read.
- **Printing**, **user-defined keys**, **soft fonts** (DRCS) and **ReGIS** are not implemented. The
  VT52's printer controls are ignored in VT52 mode.
- A program can put the terminal in **VT52 mode** (`CSI ? 2 l`), where no `CSI` is read, so a stray
  sequence can leave a terminal that seems not to answer. `ESC <` leaves it, so does `ESC c` (an
  extension: the VT52 has no reset, but `reset` sends one), and a host's `Terminal::reset` does.
- **Colour names** and device-independent colour specs are not read.

### Control characters in text you take out

OSC 8 targets with control characters are refused. Other strings are not filtered: a title, a folder, a
notification and a clipboard store arrive as the program sent them, after UTF-8 repair (invalid bytes
become U+FFFD) and, for titles and folders, a length cut. The host cleans them for its own use.

## Why each rule exists

Each row is a published vulnerability in another terminal that the rule exists to prevent. The CVE numbers
are linked to the National Vulnerability Database; the descriptions are paraphrased from its records.

| Rule | Vulnerability | What happened |
|---|---|---|
| Replies are built from state | [CVE-2008-2383](https://nvd.nist.gov/vuln/detail/CVE-2008-2383) (xterm) | A DECRQSS escape sequence with line feeds around a command name, in a text file, let attackers run commands (CRLF injection) |
| | [CVE-2022-41138](https://nvd.nist.gov/vuln/detail/CVE-2022-41138) (Zutty before 0.13) | DECRQSS in text written to the terminal could achieve arbitrary code execution |
| | [CVE-2022-45872](https://nvd.nist.gov/vuln/detail/CVE-2022-45872) (iTerm2 before 3.4.18) | A mishandled DECRQSS response |
| | [CVE-2021-33477](https://nvd.nist.gov/vuln/detail/CVE-2021-33477) (rxvt-unicode 9.22, rxvt, mrxvt, Eterm) | Potentially remote code execution through an escape sequence (`ESC G Q`) whose response is terminated by a newline |
| Title reports are behind `Policy`, and cleaned | [CVE-2003-0063](https://nvd.nist.gov/vuln/detail/CVE-2003-0063) (xterm in XFree86 4.2.0 and earlier) | An attacker sets the window title and then reports it back into the command line, to run commands when a user views a file |
| | [CVE-2024-38396](https://nvd.nist.gov/vuln/detail/CVE-2024-38396) (iTerm2 3.5.x before 3.5.2) | An unfiltered title report, combined with tmux integration, injected code |
| | [CVE-2024-38395](https://nvd.nist.gov/vuln/detail/CVE-2024-38395) (iTerm2 before 3.5.2) | The setting that controls title reports was not honoured |
| No font, file or exec sequences | [CVE-2022-45063](https://nvd.nist.gov/vuln/detail/CVE-2022-45063) (xterm before 375) | A response to OSC 50 (font) could contain Ctrl-G and lead to command execution in zsh's vi line-editing mode |
| Control-mode DCS is never read | [CVE-2019-9535](https://nvd.nist.gov/vuln/detail/CVE-2019-9535) (iTerm2 up to 3.3.5) | iTerm2's tmux integration let malicious terminal output execute arbitrary commands |
| Notifications are host requests, not actions | [CVE-2022-41322](https://nvd.nist.gov/vuln/detail/CVE-2022-41322) (kitty before 0.26.2) | Insufficient validation in the desktop notification escape sequence could lead to arbitrary code execution after the user clicked the notification |
| Clipboard access is the host's decision | [CVE-2026-48725](https://nvd.nist.gov/vuln/detail/CVE-2026-48725) (Warp before 0.2026.05.06.15.42.stable_01) | Terminal output could read and write the system clipboard through OSC 52 with no separate confirmation |
| Window operations are behind `Policy` | [CVE-2000-0476](https://nvd.nist.gov/vuln/detail/CVE-2000-0476) (xterm, Eterm, rxvt) | Escape characters that force the window to resize caused a denial of service |
| | [CVE-2024-37535](https://nvd.nist.gov/vuln/detail/CVE-2024-37535) (GNOME VTE before 0.76.3) | A window resize escape sequence caused unbounded memory use |
| Counts are clamped; no unbounded loops | [CVE-2012-2738](https://nvd.nist.gov/vuln/detail/CVE-2012-2738) (VTE before 0.32.2) | An escape sequence with a large repeat count caused a long loop and CPU use |
| | [CVE-2003-0071](https://nvd.nist.gov/vuln/detail/CVE-2003-0071) (xterm in XFree86) | A DEC UDK escape sequence made the terminal enter a tight loop |
| Bounded, checked picture protocols; no `unsafe` | [CVE-2026-33633](https://nvd.nist.gov/vuln/detail/CVE-2026-33633) (kitty up to 0.46.2) | A single kitty graphics command with a PNG payload larger than twice the initial buffer overflowed the heap |
| | [CVE-2026-33642](https://nvd.nist.gov/vuln/detail/CVE-2026-33642) (kitty up to 0.46.2) | Unsigned 32-bit wraparound in a bounds check of the compose command led to out-of-bounds heap access |

These are other programs' bugs. Each rule above exists because of a real failure, and the same failure
is easy to build again in a new terminal. Two further sources shaped the list: the
OSC 8 specification's own remarks on hyperlinks (it says terminals might consider showing the target before
the user opens it, and opening links on a deliberate action such as Ctrl+click or a menu), and David
Leadbeater's talk [*Houdini of the Terminal*](https://2023.everythingopen.au/schedule/presentation/6/)
(Everything Open 2023) on escape-sequence flaws that chain with
bugs in other programs.

## What the host must still do

The engine hands you untrusted data and requests. The remaining checks are yours.

**Clipboard (OSC 52).**
- Treat `ClipboardLoad` as a request to read the user's clipboard and **ask the user**, or refuse. If you
  never call `clipboard_reply`, nothing is sent. A secret on the clipboard (a password, a one-time code)
  is exactly what a hostile program is after.
- `ClipboardStore` overwrites what the user copied. Decide whether to allow it, ask, cap the size (the
  engine allows up to 8 MiB; `Limits::clipboard` lowers it), or limit it to sessions you trust. Show
  that it happened.

**Hyperlinks (OSC 8).**
- Show the target (a status line, a tooltip) before it is opened. The text of a link and its target are
  independent.
- Open on a deliberate action, not a plain click, and restrict the schemes: `http`, `https` and `mailto`
  are usual; a `file:` link only for a local file; nothing else.

**Notifications (OSC 9, 99, 777).**
- Strip control characters and cap the length before they reach the operating system. Do not treat the
  text as markup, as a command or as a path.
- Rate-limit them. A program can send thousands.
- Never run an action a notification asks for. `KittyNotification` carries kitty's raw metadata, which
  can describe actions and buttons; parse it with limits or ignore it.

**Titles and folders.**
- Strip control and bidirectional-override characters from titles and icon names before showing them.
  Never put one into a command line, a file name or a shell.
- OSC 7 gives `file://host/path`, percent-encoded, as the shell sent it. Check the host part, decode
  with care, and treat the path as untrusted: do not use it to run anything or to read outside what the
  user trusts.

**Pictures.**
- Decode with limits on size, pixels and time, in a process or thread you can afford to lose. A small PNG
  can decode to a huge bitmap. The engine limits what it counts; your decoder must limit what it allocates.
- Answer every `ImageData` with exactly one `image_decoded`, and free pixels on `ImageFree`.

**Raw events.**
- `Event::Iterm` carries the parts of OSC 1337 that are not inline images, unparsed, up to 64 KiB
  (a longer one is dropped; inline images keep their own, larger limits). iTerm2's protocol has features that act on the host. Ignore the event unless you implement it
  carefully, and never act on its content without validation.
- `Event::UserVar` is iTerm2's `SetUserVar`, decoded: a name of at most 256 printable bytes and a value of
  at most 64 KiB, and at most 1 MiB of values wait in the queue. The value is a program's bytes: treat it
  as text you did not write (do not run it, do not put it in a title without cleaning it).
- `Event::PointerShape` is a CSS cursor keyword from a fixed list, never the program's own text, so it
  can only choose among pointers you draw. A pointer that stays after the program that asked has gone is
  yours to prevent (reset it when the alternate screen ends or the program exits).
- `ProgramStatus` (the [program status protocol](https://mitchellh.com/writing/program-status-osc7501), OSC 7501)
  and the strings you asked for with a `Passthrough` (`Event::Osc`, `Event::Dcs`, `Event::Apc`) are protocols of
  your own or someone else's: validate them.

**Policy.** Leave `window_ops`, `title_report` and `screen_checksum` off. If you turn `window_ops` on, clamp
what a program asks for (position, size) to what you would allow. Do not turn `title_report` on for sessions
to remote machines, or for any program whose output you did not write.

**Input.**
- Strip the bracketed-paste end marker from text you paste (the `input` module's paste encoder does it),
  and warn before pasting multiple lines into a program that did not ask for bracketed paste.
- Write replies only to the PTY of the terminal that produced them. A reply written to the wrong PTY is
  an injection.

**Flow.**
- Collect replies and events regularly. The queues are bounded, but a dropped request is a request the
  program never gets an answer to.
- Feed bytes in slices and keep your lock short, so a flood of output cannot freeze your UI.

**Log with care.** If you log what a program printed, escape control characters in the log. Otherwise the
log is a place to run the same attack later.

**Spoofing stays possible.** Any program can draw a convincing prompt or a fake dialog, and hidden text
(SGR 8) is still copied when it is selected. If your application shows a trusted message (a permission
prompt, an update dialog), draw it outside the terminal's grid.

## Reporting a problem

See `SECURITY.md` in the crate. What counts for the engine: a panic, a hang or unbounded memory growth on
some input; a reply that contains bytes a program chose; a request that reaches the host without going
through `Policy` or an event; a sequence that reads a file, starts a program or loads a font; and any
case where the engine's behaviour differs from this chapter.
