# The five-minute check for what no script can do

Everything a `--background` instance can show was driven (`DRIVING.md`, `evidence/s2-overlays/`). What is left needs a person at the
Mac with the real menu bar, Dock, notification centre and keyboard. One numbered step per check; each says what to expect.
Do them in this order, in one session, about five minutes.

**Setup (once).** Build with the FFI (`apple/scripts/build-ffi.sh apple/swift`, then `swift build -c release`) and start the app
from a terminal as a normal (not `--background`) instance, so the menu bar, Dock and status item are made:

```sh
mkdir -p /tmp/Wormhole-swift-manual /tmp/manual-ctl
cp /bin/cat /tmp/manual-claude && codesign --force -s - /tmp/manual-claude      # a program named like an agent: it is detected as one
WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-manual WORMHOLE_DEVCTL=/tmp/manual-ctl WORMHOLE_SHELL=/tmp/manual-claude \
  apple/swift/.build/release/Wormhole
```

Notifications (steps 5 to 7) need a signed app bundle with the app's bundle id (`app.getwormhole.terminal.swift`): an unbundled
binary has no notification centre and the app says nothing instead of showing a banner.

Send commands with `printf 'cmd\n' > /tmp/manual-ctl/cmd.tmp && mv /tmp/manual-ctl/cmd.tmp /tmp/manual-ctl/cmd`.

## Menu bar (PL01)

1. Open the menus one after another: Wormhole, Shell, Edit, View, Terminal, Window, Help. **Expect** the lists `control menus` prints
   (`evidence/s2-overlays/PL01-menu-keys.txt` lists each item with its key), the same words and keys, separators where `menus`
   shows `---`, and Shell ▸ New Terminal With ▸ listing the shells, Shell ▸ Open Recent ▸ the projects worked in, Window ▸ the numbered terminals.
2. Press ⌃⌥] and ⌃⌥[ with a window that has two groups. **Expect** the selection to move to the next and previous group (a script cannot
   press them: they have no ⌘, so only the real menu bar or the app's own key handling sees them).
3. In the sidebar press F6 until the list has the keys, arrow to a row and press ⌃↩. **Expect** a menu under the row with
   Rename…, Add terminal to new group, Add terminal to group ▸, Split right, Close terminal, ---, Show beside… ▸, Pin to top, the two
   privacy items, ---, Reveal in Finder, Copy path; Rename… starts the name field on that row. (The same items were picked one by
   one through `row-menu` and `menu-pick`: `evidence/s2-overlays/O04-rowmenu.txt`. What only a person sees is the menu's place under the row.)
4. Right-click a row, a group head, a file in the Files tab, a panel item and the terminal itself. **Expect** AppKit to draw each menu
   with the items `control right-click` lists (`evidence/s2-overlays/O04-menus.txt`), the group menu with its colour dots.

## Notifications (PL03)

5. Make a terminal wait: `printf 'hook 1 claude UserPromptSubmit\n' > …` then `hook 1 claude Notification:permission_prompt Needs permission to run npm test`
   (as the command lines above), with another app in front. **Expect** a banner "claude needs input" with the question as its body, a
   sound, and Time Sensitive in System Settings → Notifications → Wormhole.
6. Click the banner's Reply…, type `ok`, press Return. **Expect** `ok` typed into that terminal (`control screen` shows it) and the
   banner gone.
7. End the agent's turn (`hook 1 claude Stop`). **Expect** a finished banner with Open and Show Changes; Open brings the terminal's window forward.

## Status item, quick panel, hotkey (PL04, PL05, PL02)

8. Look at the menu bar extra with the waiting agent. **Expect** its icon in the mode chosen in Settings → Notifications (a count, a dot
   or the animation), and its menu: Needs input with the terminal, its Reply row, Working, Do (New terminal, Open folder…, the project's actions).
9. In Settings → Hotkey record ⌃⌥Space. **Expect** it taken; try ⌘Space. **Expect** "macOS uses ⌘Space for Spotlight" under the field and the old key kept.
10. With another app in front press ⌃⌥Space. **Expect** the quick panel over that app: Needs input first, then Working; ↓ moves, ⌘↩ on a
    Needs input row opens Reply on it, typing and Return send the text to that terminal, Esc closes it and gives the keys back.
11. Press ⌃⌥Space again with the panel open. **Expect** it to close. Quit the other app's own use of the key if it owns it: **expect** the
    registration error under the recorder row (not shown yet; the log says why).

## Dock (PL06) and bell (T14)

12. Look at the Dock tile with the waiting agent. **Expect** the badge `1` (Settings → Notifications → Dock badge decides needs-you or needs-you-and-done);
    right-click the tile: the menu lists New Terminal, New Window and the terminals as `control ambient` prints under `dock`.
13. With Wormhole behind another app make the agent wait again. **Expect** the tile to bounce once if Dock bounce is on, and not for a bell.
14. In a terminal run `printf '\a'`. **Expect** the bell sound and a 150 ms glow in the theme's attention colour round the pane's edge when
    Settings → Terminal → Bell visual is on; **no** Dock bounce (GPUI bounces only for notifications).

## Sheets and the rest (W08, O05, windows)

15. ⌘Q with a terminal that runs a program. **Expect** the native sheet with the program named; Cancel keeps everything.
16. With the update feed fixture (`evidence/s-platform/a03-fixture.sh`) run to "ready — Restart", click the notice chip's ⓘ.
    **Expect** a popover with the error's words, Open Logs Folder and Copy reason.
17. Switch light and dark, then Reduce Transparency and Increase Contrast in System Settings, with the palette (⇧⌘P) and the sheet (⌥⌘/) open.
    **Expect** both to follow without a restart; the blur behind the window is the only thing a screenshot never shows.
