# Swift standards

The rules for the Swift code in `macos/swift`. Small enough to remember; the cheap ones are checked by
`macos/scripts/quality.sh` (no build). A rule that is not checked is still a rule: review holds it.

## Rust decides, Swift draws

Behaviour with a rule in `wormhole-core` (git, settings, keymap, search, status, words, trust) stays in Rust and
arrives over the FFI. Swift holds pure view logic (geometry, animation, focus, presentation) and a small
protocol (`...Backend`) wherever the core is not linked yet. Never re-implement a core rule in Swift to be fast
to ship; put a seam in and let the core fill it.

## Layers

Each target has a kind and a rank (`macos/scripts/layers.py` holds the table; `Package.swift` and `import`
lines are both checked). A module uses only lower modules of the kinds it may use:

| Kind | Modules | May use |
|---|---|---|
| found | `WorkspaceModel`, `WordsModel`, `AppLogging`, `TerminalCore`, `DesignSystem` | nothing of the package |
| model | the `...Model` modules: plain values, pure logic, Foundation only | found |
| ffi | `WormholeBindings`, `UniFFISource` | found, ffi |
| ui | `...UI`, `AuxWindows`, `TerminalMetal`, `FileIconTheme`, then `StudioUI`, then `WindowUI` | found, model, a lower ui |
| bridge | the `...Bridge` modules (built only with the FFI linked) | found, model, ffi, a lower bridge |
| app | `Wormhole` | everything |

The rule: **dependencies point down, never sideways, and a ui never sees a bridge** (a bridge fills a model's
protocol; the app wires them). A new target is placed in the table on purpose, or the check fails.

## Minimal code

- Delete before adding. Reuse a component before writing one; one component, one job.
- No protocol, generic or wrapper without a second user or a real seam (a backend the FFI fills later is one).
- A file holds one concept: under 400 lines (soft), never over 600 (hard). A function: under 60 lines.
- Over a budget today? It sits in `scripts/standards-allow.json`; the list only shrinks.
- No dead code, no commented-out code, no duplicated logic. Comments say why, not what.

## Apple's guidance first

Xcode ships Apple's own guidance for SwiftUI as plain text, written for coding agents and as good for people. Read
the parts that fit the change before writing or reviewing a view; they are newer than anything remembered:
`Xcode.app/Contents/PlugIns/IDEIntelligenceChat.framework/Versions/A/Resources/`, `swiftui-specialist.idechatprompttemplate`
(the index) with its `swiftui-specialist-ref-*.md.packaged` references (view structure, data flow, the environment,
`ForEach` identity, modifiers, localization, animations, soft-deprecated APIs), and `swiftui-whats-new-27` with its
references for what the macOS 27 SDK added. They are reference knowledge about the frameworks, not this project's rules:
where the two differ, these standards hold. The package targets macOS 26, so an API new in 27 needs an availability
check and a fallback. Look an API up in Apple's documentation before using one from memory.

## Swift 6 concurrency

- UI types are `@MainActor`. Model modules are `Sendable` value types (`struct`, `enum`), not classes.
- `@unchecked Sendable` and `nonisolated(unsafe)` need a `// why:` on the line or the two above (what guards it).
- No `DispatchQueue.main.async`, no `Task.detached` without a reason; await, do not block.
- Cross-actor hand-offs are values, closures are `@Sendable`/`@MainActor` on purpose.

## Events, not polling

An idle window sleeps until something happens. A change arrives as an event: the core's `wake()` then a drain, an
observer, a notification. No `Timer.scheduledTimer`, `Timer.publish` or polling `Task.sleep` loop (`rules.py`
fails them). A one-shot deadline (a debounce, a hover delay, a "copied" label going away) is allowed when a
`// one-shot: <what it waits for>` comment sits on its line or the two above.

## Access control and API

- `internal` by default; `public` only for what another module uses. A `public` type or function has a `///`
  doc comment saying what it is for (`standards.py` counts the missing ones; the count only goes down).
- Names follow the Swift API Design Guidelines: clarity at the call site, no abbreviations, methods read as
  phrases, Bool as assertions (`isEmpty`).
- No `AnyView`; use `@ViewBuilder`, generics or `some View`.

## Look: tokens, spacing, words

- Colour comes from `@Environment(\.tokens)`, spacing from `Space` (4-pt grid: no half steps), radius and type
  from `Metric` and `TextRole`. No hard-coded colour or number; a layout constant taken from the GPUI app is a
  named constant with its source in a comment.
- Words go through the `WordsModel` seam (a key per message, named like the Fluent key), never a literal in a
  view. Logs, the CLI and config keys stay English.
- Prefer a native control (`Form`, `List`, `Menu`, `NSAlert`, `NSPopover`) where it gives the GPUI app's behaviour.

## Cut text

Text is shown whole whenever the place it is in can hold it; the layout cuts it from the real width, never from a count of
characters or a width fixed in advance (a fixed width is fine where the design fixes the thing, like a tab, and then the text
uses all of it). When a row is too narrow the less important text gives way first (`layoutPriority`: the folder before the
name, the value before the token's name, a detail before a title), and what is cut is cut where it loses least: a name at
its end, a path or folder at its front, an identifier whose two ends matter (a dotted token, a hash, a file name with its
extension) in its middle. A sentence (a message, an empty state, a hint, an error) wraps and is never cut. Whatever is cut can
still be read whole: the tooltip says it, and VoiceOver reads a `Text` whole whatever is cut.

One helper does it: `CutText(whole, at: .end | .start | .middle)` in `DesignSystem` (the font and the colour are the
environment's), or `cutProbe` where a view draws the cut itself (a fade). `rules.py` fails `Elide.` and `.prefix(…)` joined to
an "…" in a view. The core cuts nothing a shell can cut by width: a text it must shorten for the GPUI app comes beside its whole
form (`SecondLine.place`).

## Tests

Swift Testing (`@Test`, `#expect`), deterministic: no `Task.sleep`, no wall clock, no network, no real user
data. Inject a clock or a stream and drive it. Models are tested without UI; a view is checked by an offscreen
render. A test never sets the app's isolation variables by hand.

## Accessibility checklist (every screen)

- VoiceOver label, value and role on every control; decorative images hidden.
- Keyboard: Tab order, arrows in lists, Return, and Esc as the GPUI app's ladder does.
- Reduce Motion: cuts or fades, loops stop. Motion is interruptible.
- Light, dark, Increase Contrast and Reduce Transparency each look right.

## What the checks do

One command runs them all: `macos/scripts/quality.sh [--strict]`.

| Script | Checks | Result |
|---|---|---|
| `rules.py` | file over 1,000 lines, bucket names, `unsafe`, polling (`Timer`, `Task.sleep` without `// one-shot:`), spawning a process, cutting text by a count of characters in a view | error |
| `layers.py` | an upward, sideways or forbidden edge; an unplaced target (`--selftest`) | error |
| `parity.py --check` | `features.csv` is valid; `--size` prints Swift lines per module | error / info |
| `standards.py` | file 400/600 lines, function 60 lines, unchecked escape without `why:`, `AnyView`, undocumented `public` | warning |

`standards.py` is warn-only: it compares counts per file to `standards-allow.json`, like
`scripts/structure.py`. `--strict` fails on a count above its entry, `--update` rewrites the file and only ever
lowers an entry (the first run wrote it). A new file starts with an entry of zero.
