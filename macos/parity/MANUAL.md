# The five-minute check for what no script can do

Everything a `--background` instance can show was driven (`DRIVING.md`, `evidence/s2-overlays/`). What is left needs a person at the
Mac with the real menu bar, Dock, notification centre and keyboard. One numbered step per check; each says what to expect.
Do them in this order, in one session, about five minutes.

**Setup (once).** Build with the FFI (`macos/scripts/build-ffi.sh macos/swift`, then `swift build -c release`) and start the app
from a terminal as a normal (not `--background`) instance, so the menu bar, Dock and status item are made:

```sh
mkdir -p /tmp/Wormhole-swift-manual /tmp/manual-ctl
cp /bin/cat /tmp/manual-claude && codesign --force -s - /tmp/manual-claude      # a program named like an agent: it is detected as one
WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-manual WORMHOLE_DEVCTL=/tmp/manual-ctl WORMHOLE_SHELL=/tmp/manual-claude \
  macos/swift/.build/release/Wormhole
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

## Added in round S3 (platform lane)

18. Edit menu: open it. **Expect** AppKit's own items under Edit as GPUI's `menus` lists them (Writing Tools ▸, AutoFill ▸, Start Dictation…,
    Emoji & Symbols): a script's menu dump holds only the app's items, the system adds these.
19. Right-click a group head in the sidebar. **Expect** one row of coloured dots (grey, blue, red, yellow, green, pink, purple, cyan, orange; the
    group's own ticked) between Rename… and Other colour…/No colour, not a Colour submenu; clicking a dot sets it and closes the menu.
    (`control right-click` lists the items; whether the palette row is drawn as dots is a SwiftUI menu's, seen only in a real menu.)
20. Turn on the Folder picker flag, click in a terminal, press ⌘J. **Expect** the picker's card at the terminal's cursor cell (below the prompt
    line when there is room), not at a fixed point; pick a folder: it is pasted at the cursor of THAT terminal; switch terminal with the picker open:
    it closes. Open it twice from a folder you never `cd`'d into: the folder shows under Recent the second time.
21. With VoiceOver on, open Settings → Fonts, open a font picker and press ↓: **Expect** it says the family it lands on. Rename a file in the Files
    tab to a name that exists: **Expect** the chip and VoiceOver say "Not renamed: something with that name exists already".
22. Copy a commit's hash from the History tab's menu, an object id from a binary diff's note, a setting's key with ⌘C in the palette and a folder path
    with ⌃↩ ▸ Copy path. **Expect** the chip to say "Copied commit hash", "Copied object id", "Copied setting key", "Copied path".
23. With Wormhole in front, finish three agents within ten seconds (see `evidence/s3-platform/PL03.txt`): **Expect** one "3 terminals finished" banner.
    A needs-input banner is never grouped.

## Themes lane (round S wave 3; SD01, SD03, X14)

Everything the control folder can reach was driven (`evidence/s3-themes/`); these need the real menu bar, a pointer on a SwiftUI menu, or a
system panel. Start a normal (not `--background`) instance as in the setup above, with a throwaway `themes/` folder.

24. Press ⌥⌘, (View ▸ Theme Studio) with the Studio closed, then again with it open. **Expect** the window to open and then to close (with the
    "Do you want to save the changes" sheet when there are edits), not to come forward.
25. Click the theme's name in the Studio's title row. **Expect** the menu: Your Themes (the one being edited checked), This Project (only when
    the terminal in front is in a project with a `theme.json`), New Theme…, New Theme From ▸ (yours, then Wormhole ▸ and the families, a family
    with a light and a dark one as a submenu), Duplicate “name” (⇧⌘S), Rename…, Move to Trash… (own file only), Add Light or Dark Appearance
    (a single theme only), Update from X… (greyed "Up to date with X" or "X is unavailable" when there is nothing to do), Revert to Saved, Revert To ▸ Browse All Versions….
26. Change a token's value, open the Edit menu. **Expect** "Undo Change “primary”"; ⌘Z undoes it and the item then reads "Redo Change “primary”"
    (inside a text field ⌘Z is the field's own).
27. Export JSON… and save into a new folder, then Export JSON… again. **Expect** the second panel to start in that folder, also after
    quitting and starting the app again.
28. Open a theme that is not in use (New Theme From ▸ Nord, with Wormhole Dark picked). **Expect** a "Preview" chip with Use beside the title:
    Use saves (it asks for a name first) and makes the theme the pick of its appearance in Settings → Appearance.
29. Put a Zed icon theme folder in the profile's `icon-themes/`, turn on the Icon themes feature flag and pick it under Settings → Appearance →
    File icon theme. **Expect** the Files tab and the Changes rows to draw its icons, "None" to draw the system's symbols, and the bundled
    Wormhole Icons again with the flag off.

## Themes lane: icon motion

What a `--background` instance cannot show: hover does not fire in it, a SwiftUI menu is never opened by an injected click, and a
motion lasts a few frames. Settings → Appearance → Animate icons is on for all of these except steps 9 and 10. This section's steps
are numbered on their own.

1. Show terminals as tabs (Settings → Terminals & sidebar → Show terminals in ▸ Tabs) and click the **+** at the end of the strip.
   **Expect** the + to turn a quarter (about a fifth of a second) and look the same after it.
2. Hover a group head in the sidebar and click its **+** (New terminal in group). **Expect** the same quarter turn on the + as the new
   terminal appears.
3. In the right panel's Changes tab click the refresh icon in the band. **Expect** the arrows to go once round clockwise (half a second).
4. Open a diff, hover a commit's detail or a diff note and click its copy icon. **Expect** the front sheet to fill, the icon to become a
   check for about a second and then the copy icon to come back.
5. Click Split right and Split down in the title row (with room for a half). **Expect** the right half (or the bottom half) of the icon's
   frame to fill from the divider outward and let go, as the new pane opens.
6. In a split press ⌘⇧↩ (or click the zoom button in a pane's header). **Expect** the corners icon to shrink away and the other one to grow in;
   press the key again: the same, in reverse. The key and the button show it alike.
7. Click the small chevron next to Open in…. **Expect** it to turn over to point up while the menu is open and back down when it closes
   (also on Esc and on picking an app).
8. Click each right-panel tab in turn (Changes, History, Notes, Usage, Project; Files has none). **Expect** the glyph of the tab you
   switched to to play once: Changes' two circles fill one after the other, History's dot fills as its lines draw out from the ring, Notes'
   lines are written, Usage's needle sweeps up and settles, Project's folder flashes full. None plays when the window opens or when the
   panel slides back in.
9. Turn Animate icons off (Settings → Appearance) and repeat 1 to 8. **Expect** no icon to animate, and the chevron of step 7 to flip
   without turning; turn it on again and click nothing: **expect** nothing to play on its own.
10. Turn Reduce Motion on in System Settings → Accessibility → Display and repeat 1 to 8. **Expect** the same as step 9.
11. Turn Increase Contrast on (System Settings → Accessibility → Display), give a group a colour (its menu) and select a terminal in it.
    **Expect** a 2 pt line in the group's colour along the bottom of the title row's rounded bar, and the Settings theme cards, the
    segmented controls and the diff preview card to wear a visible outline.

## Launch, opening files, updates (round S3, lane launch: PL07, PL09, A03, A04, X15)

L1. In a bundled build (not a `--background` instance) make Wormhole the default terminal (Settings → Terminal), then double-click a `.command`
    file in Finder. **Expect** Wormhole to open a new terminal in that file's folder that runs it. `open -a Wormhole <folder>`, Open With on a
    folder and a folder dropped on the Dock icon each **expect** a new terminal in that folder. (The handler itself is proved in a test instance
    by `control open-url`; only Launch Services' choice of this app needs the real bundle.)
L2. From a terminal in the bundled app run a command that asks for the microphone, then one that lists `~/Desktop`. **Expect** macOS's own
    question naming Wormhole with the sentences of `Info.plist` (`A program running in Wormhole wants to ...`), not a silent refusal.
L3. A release build with its feed baked in (`WORMHOLE_UPDATE_URL` set when the FFI was built): Settings → Updates → Check now against the
    real feed, Install, Restart Now with a command running (**expect** the question), Restart When Idle then Don't Restart, and switch the
    channel to Unstable and back after an Unstable install (**expect** the note under the status). A security release shows the severity line
    and its GHSA links, which open in the browser.
L4. Replace the running bundle from outside (`brew upgrade --cask wormhole-terminal`, or copy a newer `Wormhole.app` over it) while it runs.
    **Expect** Settings → Updates to say the new version is installed and the ready chip to appear in the window in front within a minute.
L5. With two windows open, wait for the ready chip, switch windows, close it with its ×, switch back. **Expect** it to follow the window in
    front, and to stay closed in every window for that version.
L6. On a Mac whose default terminal is still the old codename's app (bundle id `dev.hyperficial.andromeda`) and a build that opens the real data
    folder: the first launch takes the role over (X15; the Swift app refuses any folder but `Wormhole-swift*` today, so this cannot be tried yet).
L7. With VoiceOver on, open Help → Session Hosts and move over a card. **Expect** it read as a button with the host's standing, id and note; Return or
    Space opens and folds it.
