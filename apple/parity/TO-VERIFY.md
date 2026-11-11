# Built, not yet checked

From 2026-11-11 the lanes build first and check at the end (the owner's call, for speed). Whatever a lane builds without
driving it in an instance or testing it goes here, one line per behaviour, so the final verification wave can run all of
it: what to do, and what must happen. A lane adds a section of its own and never edits another's.

Format: `| row | what to do | what must happen | where the code is |`


## Lane terminal, wave 3 (round S)

Instance help: `--background`, `WORMHOLE_DEVCTL=<ctl>`; `launches` prints what the instance would have opened (file `opened.txt`: URLs, Finder, and the core's editor command lines); `pasteboard` / `pasteboard-set` as before; `size-chip` prints each shown pane's chip; `triple-click x y` added. A shell that prints scenes from a fifo is the easiest program; do not use `shift-ctrl` click (it pops a real menu and holds the instance).
Already run in this lane before fast mode (transcripts were not kept, the lines are re-checks): Copy Link / Open Link (recorded) / Copy Output / Copy Command / Copy / Paste from the context menu, Cmd-click of a file rule at its line and of an open-url rule, find count on a fold marker, fold click open and close, Show Hidden Lines key, never_hide_lines live, the person's own Fold Output, chips during live-resize-sim in both modes, split chips, window zoom with a new terminal, hollow cursor, no cursor after exit, word / block / Cmd-A / autoscroll selections. The built binary after them added only the items below the line marked NEW.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| T07 | real keyboard: Settings → Terminal → Option as Meta off, then Left; type Option-2, Option-L (an @ or | layout) and Option-B in a shell | off: the Option characters are typed; left: only the left Option makes Meta (ESC b); the setting works without a restart. Also the old boolean `true` in config.json reads as both | `Wormhole/Look/TerminalLook.swift use(optionAsMeta:)`, `Settings/SettingsApplier.swift`, `TerminalMetal/Keyboard/KeyMap.swift OptionAsMeta(setting:)` |
| T07 | NEW: ⇧⌘V, ⌥⌘C in a terminal | paste and copy, as ⌘V / ⌘C, unless the key is an app key (⇧⌘E etc. stay the menus') | `TerminalMetal/Pointer/TerminalShortcuts.swift shortcut(for:)`, `TerminalLook.isAppKey` |
| T07 | NOT BUILT: ⌘K should also erase the terminal's saved text (GPUI `window/panes/mod.rs:83`) | after ⌘K and a crash restore, the cleared lines do not come back | needs the store's clear_scrollback over the FFI |
| T03 | Run `printf '\e[5 q'`, switch to another pane or the sidebar (F6) | the blinking stops in the pane without keys, the cursor is hollow; back in the pane it blinks again; Reduce Motion keeps it steady | `TerminalMetal/CursorBlink.swift updateBlink(focused:)`, `TerminalFocus.swift hasFocus` |
| T03 | a theme with a cursorText colour (Tokyo Night, Catppuccin), block cursor over a letter | the letter is drawn in the theme's cursor-text colour; a theme without one keeps the cell's background | `TerminalCore/Palette.swift` slot 268, `RowBuilder.colors` |
| T03 | NOT BUILT here: OSC 12 cursor colour (engine lane's override table) | | |
| T05 | start `printf '\e[?1000h'`, move over the pane, then `printf '\e[?1000l'` | the pointer is an arrow while the program has the mouse and an I-beam after, without moving | `MetalTerminalView.refreshPointerShape` |
| T05 | a mouse with back/forward buttons over a mouse-reporting program | those buttons send nothing; middle still reports | `TerminalPointer.swift otherMouse*` |
| T08 | ⌘-click in the 4 pt padding beside a link at the edge | nothing opens | `TerminalPointer.swift press` |
| T04 | ⌘C typed in a terminal with a selection | the chip says "Copied N characters" (also for the menu's Copy and Copy Link) | `TerminalOverlay+Clipboard.swift` (`onCopied`), `AppController+Open.swift copySelection` |
| T12 | select a word, ⌘F; and right-click ▸ Find… with a selection | the bar opens with the selection's first line as the query | `FindModel.selectionSeed`, `WindowHub+Overlays.swift` |
| T13 | ⌘↑ to a block, then type a key, paste or scroll the wheel, then Edit ▸ Copy Output | it acts on the newest block again (the jump target was forgotten) | `MetalTerminalView.onInput`, `TerminalOverlay` |
| T10 | View ▸ Show Minimap in a terminal with a closed fold | a thumb-coloured tick at the minimap's left edge for the fold's lines; a press on the minimap gives the terminal the keys | `ffi/src/minimap.rs minimap_folds`, `MinimapView.swift`, `TerminalMarks.swift` |
| T15 | in each state (selection, link, OSC 133 block, alternate screen, hosted terminal) right-click | the items and enablement of GPUI's menu; Paste works in a background instance now (it went through the responder chain); the words come from the catalog | `PlatformModel/Context/TerminalMenu.swift`, `AppController+Context.swift` |
| T15 / T25 | right-click a finished block with output in the history, Fold Output flag on | "Fold Output" is listed after Put Command at Prompt; picking it makes a marker without a Probe name; with the feature off it is not listed | `TerminalMenu.swift canFold`, `WindowHub+Blocks.swift foldOutput`, `ffi/src/probes/terminal.rs fold_block_output` |
| T16 | a program in an unfocused pane runs `printf '\e]52;c;aGk=\a'` | the chip says "<program> copied 2 characters"; for a terminal that is not shown no chip; with Notices off the text appears in that terminal's own bar | `TerminalOverlay+Clipboard.swift`, `Notice.swift programLines(_, program:)` |
| T17 | ⌘+ in a window of two or more terminals, then open or move a terminal into it | all terminals zoom; the new or moved-in one takes the zoom without easing; ⌘0 resets all | `WindowHub+Zoom.swift`, `TerminalViews.zoom` |
| T18 | drag a window corner for real, grid and smooth_resize | the chip "cols × rows" shows while dragging (my run used `live-resize-sim`; a scripted drag never sets AppKit's live flag) | `WindowUI/Overlay/TerminalHost.swift layout` |
| T21 | drag a file over a pane | the pane tints (`state.highlight`) while it hovers and clears on leave or drop | `MetalTerminalView.showDropTint`, `TerminalLook.followTheme` |
| T24 | Cmd-click a rule's file and URL links in a real instance | the editor opens at the line; the browser opens the URL; the readout names the Probe | `Wormhole/AppController.swift` (diverts in tests only), `ffi/src/open_file.rs` |
| T25 | Export Text As… with a fold, in a test instance | the file lands in `<data dir>/exports/` (it was the person's Documents before: fixed) with all lines, folded ones too | `TextExporter.swift directory:`, `ActionRouter+Output.swift` |
| P05 | make a worktree from the sheet, in the new terminal run `env \| grep WORMHOLE_PORT` | WORMHOLE_PORT and WORMHOLE_PORTS are set; a terminal in the repository itself has none; the same for a hosted terminal | `ffi/src/git/worktrees.rs env_for`, `ffi/src/terminal.rs spawn_options` |
