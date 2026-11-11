# Driving a background instance

What a script can do to a `--background` Swift instance through its control folder, and what still needs a person. For the
lanes checking their rows and for the verification wave. The commands follow GPUI's devctl
(`crates/wormhole/src/devctl/mod.rs`) where the Swift app has the same thing.

## Setup

```sh
WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-<x> WORMHOLE_SHELL=/bin/zsh WORMHOLE_AUTH=cancel WORMHOLE_DEVCTL=<ctl> \
  apple/swift/.build/release/Wormhole --background &
printf 'click 150 66\nrows\n' > <ctl>/cmd.tmp && mv <ctl>/cmd.tmp <ctl>/cmd     # results are appended to <ctl>/out.txt
```

Lines run in order. A line that takes time (`drag`, `scroll`, `wait-idle`, `perf-profile`) holds the rest of its batch and any
batch written meanwhile. Points are the window content's, from its top left, in points (a `window-shot` at 1x). Persistent
terminals need `flags-unlock`, `set feature_flags={"persistent_terminals":true,"unlocked":true}`, a restart and
`WORMHOLE_HOST_BIN=<a wormhole-host built from the same tree>`.

## How the pointer and keys get there

Events are made in the process and given to the window (`NSWindow.sendEvent`, `NSApp.sendEvent` for keys), which routes them
as the window server's: hit testing, gesture recognizers, SwiftUI gestures, menus' key equivalents, the window's own event
monitors. The real pointer, the real keyboard focus, the clipboard and the window's place behind other apps are untouched
(the transcripts print the window's z-order and the front app before and after). A background app is never active, so while
an injected event is routed the window acts as the key window of an active app (`Driving/InFront.swift`); outside it every
answer is AppKit's. Entering and leaving views goes to their tracking areas. A view that follows a press in a loop of its own
(the window's resize edge, a text field) gets the rest of the gesture from AppKit's queue, so `click` and `drag` queue it ahead
of the press (`Driving/Injector.swift`).

## Commands

| Command | Example | Does |
|---|---|---|
| `click x y [mods]` | `click 150 66` | press and release (a sidebar row selects; `cmd`, `shift-alt`, … as mods) |
| `double-click x y` | `double-click 352 400` | two clicks, the second a double-click (a column handle's reset) |
| `right-click x y` | `right-click 150 66` | the menu there (`menu(for:)`, SwiftUI's `.contextMenu` too), listed and kept unshown |
| `menu` / `menu-pick <n\|title\|A > B>` / `menu-close` | `menu-pick Pin to top` | the last menu again; press an item as a click; drop it |
| `hover x y` / `hover-cmd x y` | `hover-cmd 389 51` | move the pointer there (with ⌘: a link underlines) |
| `mouse-down x y` / `mouse-move x y` / `mouse-up x y` | `mouse-down 150 182` | a held drag, a line each, so a `window-shot` can catch it mid-way |
| `drag x1 y1 x2 y2 [steps]` | `drag 351 400 300 400 10` | pressed, moved in steps a frame apart, released (reorder, column edges, window resize) |
| `scroll x y dy` | `scroll 700 400 -400` | the wheel there, dy points down (negative up), in steps of 20 a frame apart |
| `key <keystroke>` | `key shift-f6`, `key ctrl-c` | a key as the keyboard sends it (menu equivalents run their action) |
| `text <text>` | `text Build` | typed into whatever has the keys, a key per character (a field being edited, the terminal) |
| `pointer-window [title]` | `pointer-window Settings` | the window the pointer acts on (none: the workspace's in front) |
| `bounds x y [levels]` | `bounds 700 400 2` | the views under a point, then the accessibility element there and its parents |
| `a11y [title]` / `a11y said` | `a11y said` | the window's accessibility tree; the announcements made lately (`!` interrupts) |
| `wait-idle [ms]` | `wait-idle 3000` | until no terminal draws and the window looks the same three times in a row |
| `perf-profile [seconds] [name]` | `perf-profile 2 flood` | forced draws for that long: each span's avg/p50/p99/max and allocations per draw, to `<name>.txt`/`.json` |
| `hosts-list` / `host-kill <id>` | `host-kill lj2u40zqop3h` | the session hosts as the Hosts window lists them; end one from outside |
| `quit-keep` | | quit, hosted terminals left running (the same as `quit`) |
| `window-shot <path> [title]` | `window-shot /tmp/s.png Settings` | a window as drawn (GPUI's `settings-shot` is this with the Settings title) |

Landed by other lanes on `exp/swift`: `question`, `question confirm|cancel|also` (a background instance never shows a
question sheet; the control folder answers it), `studio-out`, `intro`, `welcome`, `onboarding-step`, `tear-off`, `tear-off-at`,
`profile-menu`, `minimise`, `place`, `hold-cmd`, `release-cmd`, `look-keys`. Injected clicks never need a sheet on screen.

Proof runs: `apple/parity/evidence/s-tooling/` (`pointer.txt`: row click, sidebar drag reorder, a held drag shot mid-way,
column edge, row context menu, rename typed into a SwiftUI field, the wheel; `T15.txt`; `T18.txt`; `hosts.txt`;
`keys-a11y-perf.txt`).

## Tips

- A window behind others may get no display-link tick: send `frames 2` before a terminal `shot` that must show a change, and
  prefer `shot` (the terminal's own render) to `window-shot` for terminal content.
- `type` writes bytes to the program; `text` and `key` go through the window like typing.
- `key cmd-…` runs the keymap's action as the menu bar would; inside a field it is that action, not the field's own (⌘A).
- Never `click` a link with ⌘ or pick Open Link: it reaches the system's browser.

## What still needs a person

- **VoiceOver and the SwiftUI accessibility tree.** SwiftUI builds its accessibility nodes only for an assistive app (an AX
  client the system trusts); in-process asking, AppKit's own entry points and its request flags do not wake it, and an AX
  client needs a Privacy grant no script has. `a11y` shows AppKit's part (window, buttons, the terminal's text area, any
  NSView); the SwiftUI controls' labels need Accessibility Inspector or VoiceOver.
- **Shown menus and popovers**: `right-click` reads a menu without showing it; how AppKit draws it, and the menu bar, the
  status item, the Dock tile and the global hotkey (PL02, PL04 to PL06) are never made in a background instance.
- **Notifications** (PL03): banners are logged, not shown.
- **Drag and drop across apps** (Finder files, a tear-off onto another app's space) and anything that needs the real pointer's
  position on screen (`NSEvent.mouseLocation` stays the real pointer's).
- **The input method's own UI** (marked text, the candidate window): `text` applies the key bindings directly.
- **Signed bundle behaviour** (update install, Time Sensitive entitlement).
