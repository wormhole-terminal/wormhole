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
| T07 | BUILT (wave 3, T30): ⌘K also erases the terminal's saved text at once (`Touch::Cleared` to `erase_saved_text`) | after ⌘K and a crash restore, the cleared lines do not come back | ffi `interaction/pointer.rs` `clear_history`, `workspace_texts.rs` `erase_saved_text` |
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

## Platform lane (S3): PL01 to PL06, O01 to O05, X03, X07, T14, T16, W08, A03

Already run in an instance before the switch to fast mode (evidence in `evidence/s3-platform/`): the menu dump against GPUI's (PL01-menus-*.txt: only the flag-gated
rows, AppKit's own Edit items and the Window list differed), the banner matrix through the log seam (PL03.txt), the quick panel's keys (PL05-quick-panel.txt).
Everything below was built after that and has not been run.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| PL01 | Start with the `persistent_terminals` flag on, open `control menus` | Wormhole ▸ lists "Quit and End All Terminals" and Help lists "Session Hosts" (the menu state never set `persistent`) | WindowUI/Menu/MenuStateMaker.swift (`menu.persistent`) |
| PL04/PL05/PL06 | Put a terminal in a folded group, make its agent wait (`hook`), `control ambient` | The status item, Dock menu, badge and quick panel list it (they listed only visible rows) | Wormhole/Ambient/AmbientController.swift `ambientTerminals` |
| PL05 | `control quick-open`, `quick-key up\|down\|enter\|escape\|cmd-enter`, `quick-text <t>`, `quick-state` | Rows, selection mark, Reply in place and its hints as the panel's model has them; Esc steps out of Reply then closes | PlatformUI/QuickPanel/QuickPanelModel.swift `press`, Wormhole/AppController+ControlPlatform.swift `driveQuickPanel` |
| O04 | Right-click a group head in the sidebar (real window, real menu) | One row of nine colour dots (palette row) instead of a Colour submenu, the group's own ticked; a click sets the colour and closes the menu; Other colour… and No colour follow | SidebarUI/Menu/GroupMenu.swift (SwiftUI `ControlGroup` with `.palette`: whether macOS draws coloured dots is the open question) |
| O04 | Right-click rows of Changes, History and Files in a repository | Items as GPUI's (not read before); Files rows still lack "Open in ▸" unless the panel lane added `openIn` | PanelUI/Files/FileRowMenu.swift, PanelUI/History/CommitMenu.swift |
| O05 | Copy a commit hash/short hash/message/subject/author in History, an object id from a binary diff's note, a setting key with ⌘C in the palette | Chip says "Copied commit hash" / "short hash" / "commit message" / "subject" / "author" / "object id" / "setting key"; a cut message says "Copied the start of the message, which is cut" | PanelModel/Backends/PanelHost.swift `PanelCopied`, WindowUI/SystemPanelHost.swift `copy(_:as:)`, TerminalUI/Notice/Notice.swift, Wormhole/Platform/PaletteSourcing.swift |
| O05 | Copy a commit's message with git unable to read it (a repo made unreadable) | Chip "Couldn't copy: git could not read the commit" | PanelUI/History/CommitMenu.swift, CommitDetailsView.swift (`host?.failed(.copyCommit)`) |
| O05 | Files tab: rename to a name that exists / to an empty name / of a file that is gone | Chip and field say "Not renamed: something with that name exists already" / "that isn't a usable name" / "it isn't there any more" | PanelUI/Stores/FilesStore+Rename.swift, Notice.Failed.rename* |
| O05 | HistoryKeptByHost | Not built: needs a session host older than the clear-history message (a seam on the wire version); `control notice history-kept` posts the chip only | Wormhole/AppController+ControlNotice.swift |
| T16 | A program writes the clipboard (`printf '\\e]52;c;aGk=\\a'`) in a non-typing terminal | Chip names the foreground program ("ssh copied 2 characters"); nothing for a terminal not on screen; with the Notices flag off the words appear in that terminal's own bar for 4 s | WindowUI/Overlay/TerminalOverlay+Clipboard.swift, TerminalUI/Notice/Notice.swift `.named*` |
| O01 | ⌘⌫ on a running terminal in the palette | The close question is asked (it was a straight close); folded-group terminals and parked windows are listed/skipped as GPUI | Wormhole/Platform/PaletteSourcing.swift `closeTerminal`, PaletteSourcing+Listed.swift |
| O01/O05 | ⌘C on a palette setting; ⌃↩ ▸ Copy path | Chip "Copied setting key" / "Copied path" | PaletteSourcing.swift `copySettingKey`, Wormhole/Menus/RowMenuKeyboard.swift (the SwiftUI row menu's Copy path in SidebarUI/Menu/RowMenu.swift still posts nothing: sidebar lane) |
| O03 | Folder picker flag on, ⌘J in a terminal | Card at the cursor cell (bottom-left line of the pane before a first frame); a pick is pasted into that terminal only; the picker closes when another terminal is selected; the folder you opened it from shows under Recent next time | Wormhole/Platform/FolderPicking.swift, TerminalMetal/Keyboard/InputClient.swift `cursorFrameInWindow` |
| O03 | Turn the flag off, change folders in a terminal, turn it on | The stored recent list was emptied once; recents are read from and written to the data folder's database row `recent_folders` (the GPUI app's own), not `recent-folders.json`; other profiles' lists are not read (one list for the default store) | crates/wormhole-ffi/src/folder_picker/mod.rs `folder_recents_load/save`, PlatformBridge.swift |
| X-test | `--background` instance: open Settings, Theme Studio, About; ring the bell | The windows open behind the others, nothing comes forward; no beep | DesignSystem/TestInstance.swift and its callers (SettingsWindowController, StudioWindowController, AboutPanel, TerminalBell) |
| X03 | VoiceOver on the terminal | The node reports the cursor's line, the selection (or the cursor as an empty range), the line ranges and the character count, as GPUI's text node | TerminalMetal/TerminalAccessibility.swift |
| X03 | VoiceOver in a font picker: ↓ | It says the family it lands on | FontPickerUI/FontSearchField.swift |
| X03 | Settings "Changed" page copy button | Not built: Swift has no copy button on that page (GPUI announces "Copied") | SettingsUI (settings lane) |
| X07 | `WORMHOLE_LANGUAGE=en-XA` start, then look at: the sidebar and its menus, the palette, the shortcut sheet, Settings (every page), the panel (Changes, History, Notes, Project, Files), notices and banners, questions (close, quit, restart), About, Hosts, Updates, Trust, the Studio, the font picker | Every word is bracketed and accented except user data, core-sent words (keymap labels, palette row words, settings schema labels: Rust's, change with `text_set_language`) and the `// text:` lines | `python3 macos/scripts/words.py` lists what is left; `Words.text(id, args, fallback:)` in WordsModel/Words.swift, `text_format` in crates/wormhole-ffi/src/words.rs |
| X07 | `python3 -I macos/scripts/words.py` | Exits 0 (quality.sh runs it as the `words` row) | macos/scripts/words.py, words-allow.json |
| X07 | Switch language at run time (`control language en-XA`) | Static `let` words were made computed so they follow; windows already open keep their text until redrawn | modules' `static var` conversions |
| PL03 | Burst of three finished agents in a real bundled app | One "terminals finished" banner; needs-input banners never grouped; with the group setting off each keeps its own (in the log run, `notify_group_bursts=false` showed only one finished banner of three: check this first) | core notify `Bursts`, Wormhole/Notices/NoticeFeed.swift |
| PL03 | Real banners | Manual steps 5 to 7 in MANUAL.md | |
| A03 | Not built here: Settings "Restart Now" asking when something runs, the channel note, the security line, bundle-replaced detection, the ready chip following the front window | the launch and settings lanes own the updater | Wormhole/Aux/UpdateFollower.swift, SettingsUI/Panels/UpdateStatus.swift |
| X07 | The words seam now also covers the platform modules (context menus, status item, Dock, quick panel, notification buttons, shortcut recorder, folder picker): `python3 -I macos/scripts/words.py` is clean; compile checked once, tests not run (MenuBarExpected and the Platform tests compare English strings: run PlatformTests first) | | swift-platform.ftl, PlatformModel/*, PlatformUI/* |
| O03 TEST | Recent folders are written to the data folder database (`meta` row `recent_folders`) and cleared when the flag is off | no list lost on a profile switch | crates/wormhole-ffi/src/folder_picker/mod.rs |

## Lane engine (round S wave 3: T19, T20, T27 to T31)

Instances for these: no `WORMHOLE_SHELL` and no `--shell` (the core decides the program), `HOME` pointed at a scratch folder with
a `.zprofile` (`export PROFILE_READ=yes`) and a `.bash_profile`, `SHELL=/bin/zsh`. `--sample` (or `--mock`) now asks for the sample
workspace; a build with the FFI otherwise always runs real terminals. Partly driven before the mode changed (login shell,
`default_shell`, New terminal: zsh kept across a relaunch, saved text above a restored terminal, OSC 10/11/12/4 and `CSI ? 996 n`
answers, mode 2031 on a theme switch, CSI 13/14/18/19 t reports): run them again on the merged build.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| TEST T27 | start the app as above; in the terminal `echo $PROFILE_READ $0 $WORMHOLE_PROFILE; [[ -o login ]] && echo LOGIN` | `yes /bin/zsh default` and `LOGIN` (not the sample workspace) | ffi `launch.rs`; Wormhole `Options+Real.swift`, `WorkspaceHost.swift` |
| T27 | `set default_shell="/bin/bash"`, ⌘T, `shopt -q login_shell && echo L; echo $0` | `L` and `/bin/bash` | ffi `launch.rs` (`shells::for_terminal`) |
| T27 | palette `New terminal: zsh` (key ""), quit, relaunch | that terminal runs zsh again while the others run `default_shell`; `tabs.shell` is `''` for it | core `workspace/terminals.rs` `new_terminal_with_shell`, `saved.rs`; Swift `PaletteSourcing+Listed.swift` |
| TEST T27 | a trusted project with `.wormhole/config.json` `{"env":{"X":"1"},"shell":"/bin/bash"}`: open a terminal in it; untrust it and open another | trusted: `$X` is 1 and bash runs; untrusted: neither | ffi `launch.rs` `project_adds` |
| TEST T27 | a profile with environment, git identity and a start folder (Settings → Profiles); new terminal | `env` shows them, `WORMHOLE_PROFILE` is its id, it starts in the folder | ffi `launch.rs` (`apply_profile`) |
| T27 | `--shell /bin/cat`: ⌘T, then palette New terminal: zsh | ⌘T runs cat; the shell chosen in the palette still runs zsh | ffi `launch.rs` (program unless `shell`) |
| T28 | `set scrollback_lines=1000`, ⌘T, print 5000 lines, scroll to the top | the oldest line kept is about line 4000; terminals opened before keep their old size | ffi `launch.rs` |
| T28 | `printf '\e]11;?\a'` (and 10, 12, `4;1`), `printf '\e[?996n'` in dark and light themes; with `printf '\e[?2031h'` switch the theme | answers are the drawn theme's colours; `997;1` dark, `997;2` light; the 2031 program hears `997;2` unasked | ffi `engine_look.rs`; Swift `Look/EngineColors.swift`, `TerminalLook.followTheme` |
| TEST T28 | `printf '\e]11;#203040\a\e]12;#ff8000\a\e]4;1;#00ff00\a\e[31mX'`, then `printf '\e]111\a\e]112\a\e]104\a'` | the background behind the grid and the padding turn #203040, the cursor orange, red text green; the resets bring the theme's back | ffi `frame_colors.rs`; Swift `ProgramColors.swift`, `MetalRenderer.drawPalette` |
| T28 | `perf-profile` on a flood, before and after | no new allocations per frame (FrameWorkGuardTests) | `MetalRenderer.apply(frame:)`, `FrameSections.read` |
| T29 | `set allow_window_ops=true`, ⌘T, `printf '\e[8;30;100t'`, `printf '\e[3;100;80t'`, `printf '\e[2t'`, `printf '\e[1t'`, `printf '\e[9;1t'`, `printf '\e[9;0t'`, `printf '\e[10;2t'` | the window resizes so this pane is 100x30, moves to 100,80 (kept on screen), minimises and comes back, zooms and back, full screen toggles; the grid ends at the window's size | ffi `window_ops.rs`; Swift `Platform/WindowOps.swift` |
| TEST T29 | the same with `allow_window_ops` off | nothing moves or resizes (a test holds the engine's side) | ffi `launch_tests.rs` |
| T29 | `printf '\e[13t'`, `'\e[14t'`, `'\e[19t'`, after moving and resizing the window | position, size in points, and the most cells reported, changed with the window; `\e[11t` says 2 while minimised | `WindowOps.share` |
| T29 | `printf '\e[5t'` in a `--background` instance | the window stays behind (a raise brings a normal instance forward) | `WindowOps.carry` |
| TEST T30 | type a marker, quit, relaunch (Persistent terminals off) | the terminal shows the marker dimmed above a `restored …` divider (zsh's start may push it into the history: scroll up) | ffi `workspace_texts.rs`; Swift `RustWorkspaceBackend+Hosting.swift` (`startSaving`, `finishQuit`) |
| T30 | wait over a minute after typing, kill -9 the app, relaunch | the text of that minute comes back | `startSaving` |
| TEST T30 | ⌘K, then kill -9 the app before the next minute, relaunch; separately the row's "Don't keep this terminal's text", quit, relaunch | nothing of the cleared or private terminal comes back | ffi `interaction/pointer.rs` `clear_history`, `workspace.rs` `close_what_closed` |
| TEST T30 | set `restore_terminal_text` off | `scrollback.db` has no rows; nothing is saved while off; turning it off in `config.json` while the app is closed erases at the next launch | `SettingsApplier`, `RustWorkspaceBackend.init`, ffi `erase_saved_text_if_off` |
| T20 | Persistent terminals on with hosts that keep checkpoints, then turn Restore after restart (or the text setting) off | the hosts stop writing and their files under `sessions/` go | ffi `hosting/texts.rs` `stop_checkpoints_if_off` |
| T20 | turn Persistent terminals on while the app runs, ⌘T; off, ⌘T | the first new terminal is hosted (`hosts-list`), the second runs in the app | `RustWorkspaceBackend+Hosting.swift` `hosting` |
| T19 | a program that finished unseen in a hosted terminal, quit keeping hosts, relaunch | its row says Done again; a terminal restarted from its text does not | core `status_feed.rs` `attached_again`; Swift `connected` |
| T19 | a TUI with DECSET 1004 that redraws on focus in/out; switch windows | its row stays Idle and no "finished" banner comes (a test holds the FFI side) | ffi `interaction/pointer.rs` `focus_changed` |
| T19 | run claude until its gauge shows, quit it, run it again | the old model and context % are gone until its first status line | `WindowHub.followGauges`, `GaugeBoard.forgetGone` |
| T31 | `printf '\e]9;hi\a'`, `printf '\e]777;notify;T;B\a'` in a terminal not looked at; `notices` | banners in the program's words (`B` under title `T`); a second identical one is posted again | core `status/mod.rs` `on_notice`; Swift `NoticeFeed.noticeTabs` |
| T31 | `set notify_command_after=2`; `sleep 3`; then with shell integration a command that fails after 3 s | "finished" and "failed" banners with the command's name (needs OSC 133 marks for the failure) | ffi `workspace_notices.rs`, `notify/watch.rs` |
| T31 | a script reporting OSC 7501 status (`brew`-like); make it ask and finish | banners, the status item and the quick panel name the program, not a plain terminal | `NoticeFeed.namedByReports`, `noticeTabs` |
| T31 | `wh notify --title A --body B` from a terminal, while looking at it and with notifications off | no banner while looked at or off; otherwise one in the caller's words; the row's status does not change | ffi `hook_server/control/listing.rs` |

Known unfinished: none of the two this line once named. The minimap does draw the colours a program set (it reads the drawn
palette, `TerminalOverlay` `CoreMinimap` palette from `MetalRenderer`); the exited bar's shell name is now the core's per terminal (wave 6).

## Settings lane (s3-settings): ST01, ST02, ST03, ST09, STX1, STX2, STX3, PL02

Tools added for this pass (control folder, `pointer-window Settings`): `settings-rows` lists every control on show with its frame (a row's control, `<row>.reset`, panel buttons such as `sheet.create`, `import.<source>`, `agent.<id>.hooks`, `flag.<key>`, `project.*`), `settings-control <row>` gives a segmented control's segment centres, `settings-pick <row> <item words>` picks from a system pop-up (it takes no injected click), `reset <keys>` is the reset arrow's action, `panel-pick <path>` answers the next open panel, `settings-search <q>` also lists the row ids and per-page counts, `key` and `text` go to the `pointer-window` window, `pointer-window popover` targets a popover's window. Drivers: `evidence/s3-settings/drive.py <x> <page>|all` (every toggle, segment, pick, slider and checkbox of a page: write, file, reset), `recorder.py`, `imports.py` + `fake_home.py` (FAKE settings of the nine terminals in the instance's scratch HOME). Seams: test instances never touch the Mac: Trash goes to `<data>/trash` (`DesignSystem.Trash`), what would open goes to `<data>/opened.txt` (`DesignSystem.ExternalOpen`), open panels answer from `panel-pick` (`DesignSystem.PanelPick`).

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| ST01 | Change three settings, open Changed | Card says "3 settings changed", the rows follow core's list (`Values::changed_rows`: a Feature flags row shows when flags are unlocked), nav badge matches | ChangedPage.swift `ChangedSummary`, SettingsModel.changedEntries, ffi settings/handle.rs `changed_rows` |
| ST01 | Changed → Copy as JSON | Button says Copied for 1.5 s, clipboard holds `{"blur": false, …}` of the listed keys only, VoiceOver says Copied | ChangedPage.swift, ffi `copy_changed_json`, core `Values::json` (Rust test `copy_as_json_is_core_s_snippet…`) |
| ST01 | Changed → Reset All… | A question sheet (not a native alert) with the count; Reset puts every listed key back; Cancel changes nothing | SettingsQuestion.swift, ChangedPage.resetAll |
| ST01 | Right-click any row | Menu: Copy Key, Copy as JSON, Reset to Default (off at the default); each does what it says (clipboard `minimap`, `{"minimap": true}`) | SettingRow.swift `.contextMenu` |
| ST01 | Advanced → Show config keys on | Each row shows its key; a click copies it and says Copied for 1.5 s. Unfinished: GPUI shows it only under the pointer or while ⌥ is held, here always | SettingRow.swift `keyButton` |
| ST01 | Write `{ "blur": ` into config.json | Advanced: warning with the parse error and an Open button (opens the file); every other page: "config.json can't be read, so changes made here aren't saved…" | SettingsPage.swift, RustSettingsBackend.unreadable |
| ST01 | Corrupt profiles.json, relaunch, Profile page | Banner "The list of profiles couldn't be read, so it was set aside and rebuilt: …" | ffi profile.rs `list_in`/`recovered`, page_text.rs `settings_profiles_problem`, SettingsPage.swift |
| STX1 | Add a folder with a probe.toml to the Probes folder while Settings is behind; bring Settings forward on Extensions | The list shows it without Refresh | ProbesPanel.swift `didBecomeKey` |
| ST02 | Profile → New profile…, type a name, Create | Profile exists in profiles.json AND Wormhole is in it (the GPUI Create switches); the Settings window follows | NewProfileSheet.swift, RustSettingsHost+Lists.makeProfile, SettingsPlatform.profileCreated → switchProfile |
| ST02 TEST | In a non-Default profile with a running terminal, Delete profile… → Delete | Question sheet names the profile; after: Wormhole is in Default, the deleted profile's windows and terminals are gone, its folder is gone, no crash (RISKY: data safety; `AppController.end(profile:)`) | PanelControl.deleteProfile, ProfileChipBinding.swift `end(profile:)`, RustSettingsHost+Lists.removeProfile |
| ST02 | Profile page: name, letter, each colour, start folder (Choose… with `panel-pick`), Add a folder…, env, git name/email, login folders | profiles.json takes each on blur or Return; a refused value shows core's sentence under the fields | ProfileControl.swift (frames `profile.*`) |
| ST02 | Appearance → Theme Studio (beside Import theme) | The Theme Studio opens | ThemeFileActions.swift, AppSettings `openStudio` |
| ST02 | Appearance → Import theme with `panel-pick /tmp/x.json`, then Move to Trash… on it, `question confirm` | Theme listed and picked; trash goes to `<data>/trash`, picks go back to the defaults | ThemeFileActions.swift, Trash.swift, RustSettingsHost+Themes.swift |
| ST02 | Terminal → Open folders in → Choose app… with `panel-pick /System/Applications/Utilities/Terminal.app` | `open_in` becomes `com.apple.Terminal` | OpenInControl.swift, PanelPick.swift |
| ST02 | Every system pop-up (Scrollback, Image memory, Open folders in, Default shell, Icon theme, motion choices, notification and update selects) with the real pointer | Menu opens; a pick writes its key; the reset arrow (hover the row, press it) puts it back. A script cannot click them (`settings-pick` does what the binding does and was run for the static ones) | OptionPicker.swift |
| ST02/ST03 | Every row's reset arrow with the real pointer | Shows under the pointer on a changed row, press puts it back, gone at the default (the arrow's action was run through `reset`, its press cannot be injected: SwiftUI hover follows the real pointer) | SettingRow.swift |
| ST03 | Advanced → Reset tips | Off while no tip was closed with Don't show again; on after one; off again after the press | EntryControl.swift, RustSettingsHost.refreshTips |
| ST03 | Advanced → Reset all settings → Reset | Question sheet, then everything back at its default | ResetAllButton.swift |
| ST03 | Import page with `fake_home.py` (nine terminals), `imports.py b` | Nine cards; each opens; Import selected writes only the ticked rows; the outcome line says how many | ImportCards.swift |
| ST03 TEST | Agents with fake agents on `WORMHOLE_AGENT_PATH` and `WORMHOLE_AGENT_HOME` scratch: Install hooks, Repair, Remove hooks, skill Install/Update/Uninstall, MCP Copy command, Command line tool Install/Uninstall (`WORMHOLE_CLI_DIR`), Agent access mode, Revoke | Files appear and go only under the scratch homes; the row's state words follow | AgentList.swift, SkillList.swift, CommandLineControl.swift, ApprovalList.swift |
| ST03 | Terminal → Default terminal in a signed bundle run with `--background` | Button enabled, press shows "this instance never changes the system", the Mac's default is unchanged | DefaultTerminal.swift (the seam), EntryControl.defaultTerminal |
| ST03 | Updates: channel select, Check now, switches | Writes `update_channel`; Check now runs against a local feed. Unfinished (A03): Restart Now question, channel note, security line are the aux lane's | UpdateStatus.swift, UpdateDetails.swift |
| ST09 | Fonts → each font row's picker with the real pointer (or `pointer-window popover`): search, Monospace/All, a family in its own face, recents, ↑↓ ↩ Esc | Pick writes `font_family`/`code_font_family`/`ui_font_family`; the specimen follows while open | FontControl.swift, FontPickerUI |
| STX2 TEST | Project folders: search, pick a project (`ProjectsModel.select`), per-feature segments, Trust (`WORMHOLE_AUTH=allow`), Revoke trust, Open in a new terminal; run once with `WORMHOLE_AUTH=cancel` | Choices write the project's config; the "Approval isn't available on this Mac" banner shows where the owner can't answer; trust is shown in the Project tab too | ProjectsPanel.swift, ProjectPageView.swift (frames `project.*`) |
| STX3 | Feature flags: each flag's switch, reset arrow, ⌃⌥⌘F | `feature_flags` in config.json; nav gating follows | FlagList.swift (frames `flag.<key>`) |
| PL02 | Hotkey page: click the field, press ⇧⌘Space; ⌘T; ⇧⌘4; a bare J; ⌥Space; Esc; Delete; the × | Accepted / "⌘T is New Terminal" / "macOS uses ⇧⌘4 for screenshots" / needs a modifier / "The shell uses ⌥Space" / ends unchanged / clears / clears. While recording the hotkey is held off. RISK: in a scripted run, after a refusal the field stopped receiving keys (Esc did nothing): check with a real keyboard that Esc ends a recording after a refusal; GPUI leaves recording after a refusal, here it keeps waiting | ShortcutRecorder.swift, KeyCapture.swift |
| PL02 | Put `hotkey` = `cmd-space` into config.json by hand | The field shows the macOS conflict in the warning colour; a key the system refused to register shows its error the same way | EntryControl.shortcutProblem, SettingsModel.hotkeyError, HotkeyController.error |
| PL02 | Jump key: record ⌥⌘K; ⇧J; ⌘T; its own ⌥⌘J | Accepted; refused (shift alone); refused; accepted (own action is no conflict); the new key jumps to the waiting terminal | ShortcutRecorder.swift, JumpKey.swift |
| PL02 | The global hotkey against another app (manual, MANUAL.md) | Registered key opens the quick panel from any app; held off while recording | HotkeyController.swift |

No consumer yet (checked in the source; the row writes the right key and shows its state): `scrollback_lines`, `image_memory_mib`, `allow_screen_checksum`, `allow_window_ops`, `allow_title_report` (ffi/src/terminal.rs `spawn_options` uses constants and defaults), `default_shell` and `shells` (nothing reads them when a terminal starts), `icon_theme` (only the bundled theme is used), `restore_terminal_text` (reaches the sidebar's look only). `option_as_meta` is read by the Swift KeyMap.
Not built: per-version theme cards, the looping motion scenes, the two other density scenes, A03's Settings update parts, ⌥-held / hover reveal of config keys.


## sidebar lane (S01 S02 S03 S05 S07 S09 S10)
Driven already in a --background instance before fast mode (transcript evidence/s3-sidebar/close-question.txt, scripts there): row menu and palette close ask the question; rename ends and keys return to the terminal; click on a selected row gives the terminal the keys; lock and worktree marks; group in-place rename and popover opening and Esc; tab keycaps; tab context menu; right-click of the + (both places) and a chosen shell; Ctrl-Opt-[ ] 1 2 with last selected; Add to new group names at once; group-menu (control) lists the keyboard menu. Everything else below was built, not run.
| S01 | Run `sleep 300` in a terminal, hover its row, click the x | the close question names `sleep 300` and Close ends it; Cancel keeps it (hover never shows in a background instance) | SidebarUI/Row/TerminalRowView.swift (`context.closeTerminal`), SidebarContext.swift, Wormhole/AppController+Quit.swift |
| S01 | Hover a row whose name is cut off, and the second line when it is cut off | tooltip: name over "place · detail" | SidebarUI/Row/RowLines.swift, CutProbe.swift |
| S01 | Hover the lock and the fork marks | two-line tooltips "Hidden from agents" / "<name> · worktree of <repo>" over the folder | SidebarUI/Row/RowMarks.swift, SidebarText.swift |
| S01 | Move a row into a group (menu or drag) | the row is highlighted for 600 ms, fading out | SidebarUI/Row/RowRings.swift `RowFlash`, SidebarView `act` |
| S01 | Right-click a row (a real pointer) | while its menu is open the row wears the focus ring, gone when it closes | SidebarUI/Row/RowRings.swift `OpenMenuRing` (follows NSMenu tracking while hovered) |
| S01 | Make a group with "Set folder…" + "Only terminals in this folder", open another row's "Add terminal to group" | that group is greyed in both the right-click menu and the Ctrl-Return menu | SidebarUI/Menu/RowMenu.swift, Wormhole/Menus/RowMenuKeyboard.swift, core `admits` over `group_admits` |
| S01 | Fill the pane area so a split has no room, open "Show beside…" | only terminals with room are listed | RowMenu.swift besideMenu, RowMenuKeyboard.swift (`paneDropRefusal`) |
| S01 | Row menu "Copy path" | the "Copied" notice; the pasteboard file has the path | WindowUI/Hub/WindowHub+Sidebar.swift `copyFolder` |
| S02 | Rename a terminal, then press Esc / click elsewhere | the old name stays (Esc) / the typed name is kept; the keys are in the terminal after each | SidebarUI/Row/TerminalRowView.swift `rename` |
| S03 | Press Ctrl-Return with the list cursor on a group head (a real keyboard) | the group's menu opens under the head (no colour dots in it: they are the popover's and the right-click menu's) | SidebarUI/Keys/SidebarView+Keys.swift, Wormhole/Menus/GroupMenuKeyboard.swift |
| S03 | Rename… in a group's menu, type, Return / Esc / blank | in place on the head; Return keeps, Esc cancels, blank keeps the old name; keys go to the terminal | SidebarUI/Group/GroupHeadView.swift |
| S03 | Open a group's popover | No-colour ring first; folder lines; New terminal in group; Move group to new window; Pin group; closing it gives the terminal the keys | SidebarUI/Group/GroupPopover.swift, GroupFolderItems.swift, SidebarSectionView.swift |
| S03 | Hover a group's name; hover its + | tooltip with the folder; "New terminal in <name>" | GroupHeadView.swift, HeadControls.swift |
| S03 | Set folder… (system panel) | a background instance holds on the panel: do this in a visible instance | SidebarUI/Menu/GroupFolderPanel.swift |
| S05 | Terminals as tabs, hold Cmd alone | ⌘N keycaps on the tabs in place of the x, the core's numbers (a folded group's terminals have none) | WindowUI/TerminalColumn.swift, WindowHub+Sidebar.swift `tabBar` |
| S07 | Split two terminals; drag a pane header onto a row's middle / onto a gap / below the rows (a real pointer: system drag) | middle joins or moves like a row drop (mark drawn); below the rows or on a row that takes nothing the terminal leaves its split, unless let go on its own split's row | SidebarUI/Drag/PaneHeaderDrop.swift (`PaneHeaderLanding.decide` has a unit test) |
| S09 | Tabs mode, switcher popover, New group | a new terminal in a new group of its own | WindowUI/Hub/WindowHub+Sidebar.swift (`newTerminalInNewGroup`) |
| S09 | Switcher "Show all groups in the sidebar" and View > Show Terminals in Sidebar/Tabs, quit and relaunch | `terminals_in` in config.json follows; the choice survives the relaunch | Wormhole/AppController+Windows.swift, ActionRouter+View.swift |
| S09 | Switcher footer; a pinned empty group's card | key hint "⌃⌥[ / ] previous, next group" and "⌃⌥1–9"; a + on the card starts a terminal in it | PanesUI/Tabs/GroupSwitcherView.swift |
| S09 | Ctrl-Opt-] / [ / 1-9 with a group folded in the sidebar and Ungrouped last; with an empty pinned group | folded groups count; a set opens on its last-selected terminal; an empty pinned group gets a new terminal (via the number key) | WindowHub+Menu.swift `selectGroup`, `openSet(at:)` |
| S09 | NOT BUILT: switcher cards' popover / menu / wash / agent logos; profile chip in the sidebar's title row; tab tile with agent logo and status glyph; tab VoiceOver "n of m" | - | PanesUI/Tabs/GroupSwitcherView.swift `SetCardView`, WindowUI/SidebarTitleRow.swift, TerminalTabView.swift |
| S10 | Tab bar or title row: right-click the + (a real pointer) | "New terminal with" and the shells, the default checked; a choice opens that shell (menu bar's New Terminal With action) | PanesUI/Tabs/ShellsMenu.swift, WindowUI/SidebarTitleRow.swift |
| S03 | NOT BUILT: the group menu's colours as an inline swatch row and "Other colour…" opening the system Colours panel directly (the SwiftUI menu has a Colour submenu of names, the popover has swatches and a picker; the Ctrl-Return menu has no dots) | - | SidebarUI/Menu/GroupMenu.swift, PlatformModel/Context/GroupMenu.swift |
| S10 | First run in the terminal welcome on a machine with no coding agent, finish it | the sidebar's note "No coding agents yet"; x gives "Tip closed" with "Don't show again"; that persists until Settings > Advanced > Reset tips; "Coding agents settings" opens that page; the note goes when a terminal runs an agent | SidebarUI/Tip/NoAgentsTip.swift, WindowUI/SideColumns.swift, HookHost+Welcome.swift (`welcomeEnded(note:)`), FFI `dismiss_no_agents_tip`. NOT BUILT: the old intro window's own ending does not raise the note |
| S10 | Right-click the + in the sidebar title row of a window | same as above | WindowUI/SidebarTitleRow.swift |
| S06/gap 50 | Cmd-F with the list focused | the terminal's Find opens (the sidebar search is removed) | Wormhole/Actions/ActionRouter+View.swift |
| S01 | Space on a row (a real keyboard; SwiftUI never gets injected keys) | the terminal is shown and the list keeps the keys; Return gives the terminal the keys | SidebarUI/Keys/SidebarView+Keys.swift `openTerminal`, WindowUI/TerminalViews.swift `keepKeys` |
| S01 | TEST (data loss): Cmd-W / x / menu on a terminal inside a FOLDED group that runs `sleep 300`; an idle agent (claude copy) at its prompt; Quit and Close Window with such terminals | the close, window and quit questions list them (the core's `command` and state decide, not the row's wording); Cmd-W on the folded one closes it after the question | Wormhole/AppController+Quit.swift (`closeTerminal`, `closeGroup`, `quitFacts`, `askToClose`), AppController+WindowEnding.swift, WorkspaceModel/Terminal.swift `command` |

## Lane window (round S3)

Built and not checked (what I did check is in `evidence/s3-window/`). Instance recipe: `macos/parity/DRIVING.md`; use a single window for key checks (a later window hears no injected key: driver limit).

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| W06 | Two profiles, window in the shown one: `profile-menu move 1`, then `new` in the moved window with persistent terminals on; read the Default profile's database `tabs.host_id` rows | the new terminal's host row is in the profile the window moved to; none new in the one it left; closing a terminal that ran before the move forgets its row there | `ffi/src/hosting/desk.rs follow_profile`, `registry.rs follow`; `Settings/ProfileChipBinding.swift move`; `HostingSetup.profile` for windows opened beside (test `hosting::tests::a_window_that_moved...` passed) |
| W06 | Open Window in a second profile with persistent terminals on, start a terminal there | its host row is in that profile's database, not the active one's | `Wormhole/WorkspaceHost.swift` (`HostingSetup(profile:)`), `HostedSessions.open` |
| W03 | Two windows, status bar on, a hook makes a terminal in window 2 need input | window 1's bar shows the attention button and counts of both windows; a click selects it and brings window 2 forward | `AppController+Windows.swift` (`everyTerminal`, `showTerminal`), `WindowBody.statusBar` |
| W03 | Three terminals need input, `since` differs | the button is the one waiting longest; tooltip "a, then b, c and 2 more · go to the first (⌃⌘↓)" | `WindowUI/StatusSummary.swift` |
| W03 | Terminal working/done/failed for a few minutes | the right of the bar says "working · 4m" / "done · 3m" / "failed · 2m" at every width (failed has no "exit N": the model carries no exit code); "needs input", "exited" | `WindowUI/StatusBar.swift stateText`, `ActivityLook.stateWords` |
| W03 | Run `vim` or an agent whose title is set | the size item says the program ("vim · 90x40"), not the title | FFI `Workspace.program_name`, `WorkspaceStore.programName`, `WindowBody.statusBar` |
| W03 / W12 | Window in another profile than the shown one | status bar leading item (Words width) names the profile with its colour dot and tooltip; a 3 pt edge in the profile's colour across the window's top | `WindowBody.swift` overlay, `StatusBar.swift`, `ProfileChipBinding.profileList` (colour now from `SettingsProfileRow.color`) |
| W12 | Window of profile B while profile A is shown, different opacity/theme/fonts in B | NOT BUILT: the window still draws with the shown profile's settings, theme and fonts (one `ThemeStore.shared`, one `WindowLook`). Design: key the look and the theme store by profile, hand them down by environment from the window's session; the settings host needs a read of another profile's values (`Config::load_for`) | `WindowUI/WindowLook.swift`, `DesignSystem/Theme/ThemeStore`, `SettingsApplier.swift` |
| W01 | Drag a pane divider and press Esc before letting go | the layout goes back and the rest of the gesture moves nothing | `PanesUI/Panes/PaneDragSession.swift cancelled`, `PaneDividers.swift`, `WindowFocus.escape` |
| W01 | Hover Zoom and Close in a pane header | tooltips "Zoom pane (⇧⌘↩)" / "Close pane (⌘W)" | `PanesUI/Panes/PaneHeader.swift`, `PaneButton.key` |
| W04 | Rest on a column handle, then drag it | after ~120 ms the line takes `resize.handle.hover`, `resize.handle.active` while dragged; the readout stands beside the pointer (x+8, y-8) | `WindowUI/Columns/ColumnHandle.swift`, `WindowBody+Drag.swift` |
| W09 | System Settings, Double-click a window's title bar to: Minimize / Fill / Do nothing, then double-click a title row | minimises / zooms / does nothing | `WindowUI/WindowDragArea.swift doubleClick` |
| W02 | `control ports` with a dev server in the terminal for 10 s, `ports 1` | chip listed; the browser launch is recorded in `launches.txt` (checked); a real click opens the browser | `PanesUI/Title/PortChips.swift`, `TitleClicks.swift` |
| W02 | In a non-background instance pick an Open in item and a path card button by the pointer | the real editor opens / Finder shows / path copies; nothing else changes | `PanesUI/Title/OpenInButtons.swift`, `PathCard.swift`, `SystemOpenIn.swift` (Launcher) |
| S08 | Reduce Motion on (system), fold a group, change a status, drag a row over another | no pose, no ring, no nudge, crossfade is a cut | `SidebarUI/Group/GroupRows.swift`, `Tile/StatusPlay.swift`, `Drag/LiftedPicture.swift` |
| S08 | Fold a group while another fold is running (two quick clicks) | ends in the right state, no rows stuck | `GroupRows.swift` (run token) |
| S07 | Drag a row, look at the picture | row card with shadow at the grab point, crossfades to the chip over a row's middle (checked), back when it leaves | `SidebarUI/Drag/LiftedPicture.swift` |
| W07 | Settings page that has a row Zen overrides while Zen is on | NOT BUILT: GPUI shows a note with "Leave Zen mode" (settings_window/zen_note.rs); settings lane's page | `SettingsUI` |
| X04 | In a single window: F6 to the panel's list, arrows and Tab stops over Changes/Notes/Project/Usage; Esc from each overlay (palette, find, shortcut sheet, diff) | arrows walk the list, one Tab stop per section, Esc closes the overlay and the keys return to the terminal | `WindowUI/Focus/*`, panel lane's `PanelKeyboard` |
| X04 | In a second window press a key through the control folder | KNOWN DRIVER LIMIT: `NSApp.sendEvent` reaches only the first window; fix `AppController+ControlKeys.press` | `Wormhole/AppController+ControlKeys.swift` |
| N3 | Drive any click, then read `NSApplication.keyWindow` | AppKit's own answer outside an injected event; with `MallocStackLogging` on, perf-profile leaves the logger as it was | `Wormhole/Driving/InFront.swift` (methods swapped only during the call), `TerminalMetal/AllocationCounter.swift` (restores the previous logger) |
| W05 | Real full screen: ⌃⌘F, quit with ⌘Q, relaunch; and a window on a second display that is then unplugged | comes back full screen / on the main display inside it | `WindowUI/WindowFrames.swift` (the seam is background-only) |

## Lane panel-c, wave 3 (round S): the second audit's gaps in P01 to P09

Nothing here was run in an instance. Tested (real FFI or model): the trust window's held approval (`AuxBridgeTests/TrustBridgeTests`), the notes.md merge saved at once (`WormholeTests/NotesMergeSavedTests`), CRLF line cutting and the picture decoder (`PanelTests/HostileFileTests`).

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P08 | untrusted project with more than 6 lines, "View all N…", press Trust with `WORMHOLE_AUTH=allow`; a second run where `.wormhole/actions.json` is rewritten while the dialog is up (real Touch ID, a `git pull`) | first: trusted. Second: the window reads "changed" with the new lines; what was on screen is what got approved, never the rewrite | `ffi/src/aux_backends/trust.rs hold_reviewed/approve_reviewed`, `AuxBridge/TrustBridge.swift approve`, `OnboardingBridge.swift trust` |
| P08 | "Open a window in ▸ Work" while Wormhole is in Default; open a project trusted only in Default there; also "View all N…" from that window | Work's window says not trusted; approving there writes Work's `trusted-projects.json`, not Default's | `FilesBridge/FilesBridge.swift makeBackends` (`followProfile`), `Wormhole/Aux/AuxBackends.swift trust(root:profile:)` |
| P08 | project with a `layout` of two terminals with `run`; "Open terminals" twice | the second click opens nothing, types nothing and selects nothing. KNOWN: the match is the shown title, not the custom name (the summary carries none) | `Wormhole/Panel/PanelEffects.swift open` |
| P08 | action with `target` a named terminal that is open but not selected | that terminal is selected, then typed into | `PanelEffects.swift terminal(for:)` |
| P08, P07 | `cd repo/sub` in a repository; Files tab, Project tab: pick a folder colour, and (no project) Create project folder | the tree and path menu are of `repo`; the colour tints the band; the caption says `Creates ~/…/repo/.wormhole/` and that is what is made; the project is still found from `repo/sub` upwards | `PanelUI/Controller/PanelController.swift follow/settle`, `ffi/src/git/service.rs repository_root`, `PanelUI/Stores/ProjectStore.swift band` |
| P08 | a prompt's "Copy into a new note" | the Notes tab shows with the note open for writing | `ProjectStore.copyToNote` → `PanelHost.show(tab:)` |
| P06 | project trusted with Shared notes; terminal on show; add a line to `notes.md` outside; wait 3 s; quit at once (no edit, no terminal switch); start again | the line is in the Notes tab and still in `notes.md` after the restart; the sidebar row's count follows the merge | `Wormhole/Panel/NotesCoordinator.swift sync` |
| P06 | ⌥⌘S with text at an agent's prompt | a new note holds it, the Notes tab shows, the keys stay in the terminal. NOT BUILT: the terminal's selection taken first | `PanelUI/Stores/NotesStore.swift stashPrompt` |
| P06 | ⌘↩ and ⇧⌘↩ in a note, the hover Send on a step | pasted, the terminal has the keys; with ⇧ Return follows about 120 ms later | `NotesStore.send(text:submit:)` |
| P06 | Notes feature off (flag or switch): ⌥⌘N, ⌥⌘S | nothing happens. NOT BUILT: the sync still runs with the feature off | `Wormhole/Actions/ActionRouter+View.swift` |
| P02, P03 | two terminals A and B in one repository: in A pick a compare branch, fold a section, ⌘-click two files, open a diff and comment a line; select B; back to A | B: diff closed, vs HEAD, no marks, no comments, no review bar. A again: its branch, folds, marks and comments are back (the diff stays closed); "Paste into Terminal" in A pastes into A | `PanelUI/Stores/ChangesStore.swift follow(_:terminal:)`, `WindowUI/WorkspaceRootView.swift followTerminal` |
| P04 | A: pick a branch in History, File History of a path, open a commit; select B; back to A; `cd` elsewhere in A | B has its own HEAD log; A gets rev, filter and open commit back with details read again; a `cd` keeps the rev and drops filter and commit | `PanelUI/Stores/HistoryStore.swift follow(_:terminal:)` |
| P07 | A: open two folders, select a row, Choose Folder…; B; back to A; a rename field or viewer open while switching | each terminal has its own tree, selection and chosen folder; the viewer and the rename field go with the switch | `PanelUI/Stores/FilesStore.swift follow(_:terminal:)` |
| P02, P04 | right-click a changed file ▸ File History | the strip marks History, the log is filtered, the open commit is folded; clicking Changes goes back. With History off the item is not in the menu | `PanelController.showFileHistory`, `PanelUI/Changes/PathMenu.swift`, `WindowUI/Hub/WindowHub+PanelHost.swift showTab/offersTab` |
| P02 | ⌘-click a submodule row; ⇧-arrow over one | it opens like a click and is never marked | `ChangesStore+Walk.swift click, mark` |
| P02 | Open in set to Finder: hover a changed file, a diff's header, a diff line, a file in the viewer | the open icon is there and shows the file in Finder | `FileRowView.swift`, `DiffHeader.swift`, `DiffView.swift lineOpen`, `FileViewerView.swift`, `WindowUI/SystemPanelHost.swift open` |
| P02 | Copy Path from a row menu | the notice chip says the path was copied. NOT BUILT: Copy error and History's copies post no chip (no notice kind for them) | `SystemPanelHost.copyPath` |
| P02 | switch to another tab and back to Changes | the status is read again | `PanelUI/Changes/ChangesView.swift onAppear` |
| P03 | › on the last file of the list, ‹ on the first (Changes and a commit's files) | wraps round; the list's arrow keys still stop at the ends | `ChangesStore.step`, `HistoryStore.step` |
| P03 | a narrow diff (under 560 pt); a file moved to another folder with the same name; a submodule's diff | "File N of M" shows; "Renamed from <old name>"; a badge with the submodule's words | `PanelUI/Diff/DiffHeader.swift` |
| P03, P07, P01 | close a diff by × or "esc close", the viewer by ×, the panel by ⌥⌘I | the terminal has the keys each time (the panel's title button is not wired, only the key and menu) | `PanelController.closeCover`, `FileViewerView.swift`, `ActionRouter+View.swift toggleRightPanel` |
| P07 | open a file in Files, switch to Notes; open a diff while a file is open | the file stays over the terminal on any tab; opening a diff takes the file down for good and the reverse | `PanelController.cover`, the stores' `opened` |
| P07 | view a CRLF file and an LF file ending in a newline | CRLF: one numbered line per line, no ␍; LF: no extra empty last line | `PanelModel/Files/ViewedLines.swift` |
| P07 | view a large photo, a PNG whose header declares 60000 x 60000, a broken picture | scaled to at most 4,096 px, decoded once off the main thread; "This picture is too large to show."; "This picture can't be shown." | `PanelUI/Files/FilePicture.swift`, `FilesStore+Viewer.swift readViewed` |
| P07 | Open in an editor that is not installed any more; a file named `x.command` | Finder shows the file; it is never opened in its default app. NOT BUILT: the path is still not resolved inside the root before the launch | `SystemPanelHost.open` |
| P07 | rename a pinned folder, then open the path menu; rename the boot disk | Unpin shows for the new name at once; `/` is named as the disk is | `FilesStore+Rename.swift`, `FilesStore.pathMenu` |
| P04 | Esc in the branch picker; click a parent link far down the log | Esc only closes (the open commit stays); the parent's row scrolls into view | `PanelUI/Shell/RevPicker.swift`, `HistoryStore.show` |
| P04 | open a commit, then move a tag onto it from another terminal; clear the File History chip while its log loads | the ref chips update; the whole log shows, not the filtered answer | `HistoryStore.refresh` |
| P05 | Settings: set "Worktrees folder" to `~/wt` and turn on clone build folders; New Terminal in New Worktree…; `wh new-terminal --worktree x` | the sheet's "Folder:" is under `~/wt` (also when the setting changes while it is up), `node_modules`/`target` are cloned; the command line reports the same folder | `Wormhole/Settings/PanelSettings.swift`, `PanelUI/Stores/WorktreeStore.swift setOptions`, `ffi/src/git/worktrees.rs base_of` |
| P05 | project with a multi-line `worktree.setup`; make a worktree | the setup is pasted bracketed and submitted in the new terminal | `Wormhole/PanelBackendsFactory.swift worktreeCreated` |
| P09 | two windows, an agent in each on screen; `usage-status` report through the hook socket; close the first window, report again | both Usage tabs show each report. KNOWN: "Move Window to profile" moves the one shared service to that profile | `PanelBackendsFactory.usage`, `FilesBridge.makeBackends(usage:)` |

Not built, from the audit's tables (each still as the audit describes it):
- P01: actions of a tab not offered (menu, keys); Zen and show-tab; the stale tab left while the panel is closed; tooltips with keys; VoiceOver tab words.
- P02: nested repository row; the picker's "Loading branches…" and compare hint; Copy error's chip.
- P03: the submodule card; re-read while comparing with a branch; tree-sitter colours; rows built once.
- P04: History's copy notices; the details menu and ↩ on its row; the palette's commit when the log lacks it; age timer; wrapping refs; the open icon only for files on disk; PR checks notification; `gh` under the window's profile; picker words.
- P05: `--command` with `--worktree`; the command line's Refused / already-up answers; trust asked of the repository; folded groups in the close question; the per-profile worktree store; a trust refusal keeping the sheet up.
- P06: ⇧⌘E into the notes; the draft step's keys and actions; the 12 wrapped lines preview; the sync's "being written" test; the small rules still in Swift; the sync with the feature off.
- P07: the fixed line-number gutter; pins per profile; the crumb as a button; resolve-within-root before Open in; rename's catalog sentences and busy guard; icon theme; no reads while hidden; non-UTF-8 names.
- P08: the project's theme; action keys; the Project feature switch honoured in the FFI; hub problems in the no-project card; the `.andromeda` head; per-terminal ids are never dropped from the stores' kept state when a terminal closes (a few values per closed terminal until the window closes).
- P09: the per-second and per-minute redraw; reports per profile.

## themes (round S wave 3)

Control folder: `studio` opens the Studio, `studio-do <verb>` drives its session (`open <theme>`, `new`, `new-from <built-in>`, `new-from-own <file>`, `duplicate`, `use`,
`use-in-project`, `add-appearance light|dark`, `update-apply`, `follow-system`, `close`, `menu` = what the menus offer, plus the older `set/reset/rename/save/undo/...`).
Build with `scripts/build-ffi.sh macos/swift` and `swift build -c release --manifest-cache none` (without that flag SwiftPM can keep a cached manifest and build without the FFI).

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| SD03 | Put `Mine.json` (`{"name":"Mine","extends":"Nord","ui":{"primary":"#ff6600"}}`) in `<data>/themes/`, `set theme="Mine"` (also by its slug and by its file path) | `look-keys` says `accent primary=#ff6600`; a pick that does not resolve keeps the default and says why | `SettingsUI/ThemeSync.swift` `applyPicks`, `DesignSystem/Theme/ThemeStore+Resolve.swift` |
| SD03 | With that theme shown, edit the file's primary on disk | within a second the app recolours (no restart) | `ThemeStore.refreshIfThemeFilesChanged`, `SettingsApplier.apply`, FFI `theme_files_stamp` |
| SD01 | Studio: change a token, rename, Save | file in `themes/`, `theme` in `config.json` (not only in memory), still the pick after quitting and starting again; Settings → Appearance marks the card | `StudioDocument+Save.swift`, `RustSettingsHost+ThemePicks.swift`, FFI `settings/theme_picks.rs` |
| SD01 | Onboarding Look step: click a light theme card while the mode is Match system and the system is dark | `theme_light` is set, `theme_mode` unchanged; same for Studio Save and a Settings card | `ThemeStore.setTheme`, `LookStep.swift` |
| SD01 | Open a theme made by the GPUI app (version 1, with a `base`), edit, save | the file stays version 1 (`id`, `base`, key spelling `cursor.text`, `ansi.4`); Base filter segment and `base` origin tags show on untouched tokens; "Reset to base" on an edited one | `StudioModel/File/ThemeFile*.swift`, `StudioKeys.swift`, `StudioDocument+List.swift` |
| SD01 | Open a hand-written (older) theme, edit, save | saved as version 1 with the notice "Saved in the new theme format…"; the previous file is under Browse All Versions | `StudioDocument+Save.swift` `writeOwn`, FFI `studio_convert` |
| SD01 | Theme menu ▸ New Theme… / New Theme From ▸ (yours, Wormhole ▸, families) | "Untitled" (neutral palette) / "Untitled (from X)" with its base; asks to save unsaved edits first ("…before opening “X”?") | `StudioSession.swift`, `ThemeNameMenu.swift`, FFI `studio_new_theme`, `studio_copy_theme`, `studio_theme_list` |
| SD01 | Theme menu ▸ Add Light/Dark Appearance on a single theme; Light \| Dark control; Save | becomes a pair `light`/`dark`; one undo step "Add Appearance"; Export writes the shown appearance only, with the note | `StudioDocument+Pair.swift`, FFI `studio_add_appearance`, `studio_export` |
| SD01 | A copy of a built-in whose source changed (edit `base` in the file by hand) | title shows the update button; menu says "Update from X…"; the sheet lists changed values ("yours · kept" on own ones); Update is one undo step; unchanged copy: menu says "Up to date with X" greyed | `UpdateSheet.swift`, `StudioDocument+Pair.swift`, FFI `studio_update_check/apply` |
| SD01 | Save under a name another theme file has | question Replace/Cancel (not a refusal); Replace keeps the old file as a saved version; a name of one version of a pair is refused | `StudioSession.write`, `StudioQuestions.exists`, `StudioDocument+Save.swift` `writeNew` |
| SD01 | Theme menu ▸ Duplicate "X" (⇧⌘S) | writes "X copy" (then "X copy 2"), the window now edits the copy, the original stays as saved, picks unchanged | `StudioDocument.duplicate`, `StudioSession.duplicate` |
| SD01 | Open a theme not in use (New Theme From ▸ …) | "Preview" chip with Use; Use asks for a name/saves, then it is the pick; after Save of a saved pair the notice offers "Use with Match system" (sets both picks and mode Match system) | `StudioDocument+Use.swift`, `TitleBar.swift` `marks`, `NoticeRow.swift` |
| SD01 | Open a file with `"version": 9` and an unknown key | "Made with a newer Wormhole" chip with the skipped count in its tooltip; Save refuses with the message and the Duplicate button; the file is untouched | `StudioDocument.isNewer`, `StudioDocument+Save.swift`, FFI `studio_read` |
| SD01 | Esc in the Studio (not in a field/rename/completion list); ⌥⌘, with it open | Esc closes (asks about edits); ⌥⌘, closes it, again opens it | `StudioView.swift` `.onExitCommand`, `StudioLauncher.toggle`, `ActionRouter+App.swift` |
| SD01 | In a token's expression field type `mix(primary, surf` | list under it; ↑ ↓ move the highlight, Return and Tab take it, Esc closes only the list | `ExpressionField.swift` |
| SD01 | Settings → Appearance text size larger, open the Studio | the Studio's text follows (`settings_text_size`) | `ThemeStore.textScale`, `ThemeSync`, `StudioView.swift` |
| SD01 | Export JSON… twice (real panel); quit and start; Export again | the panel starts in the folder of the last export | `StudioSession+Export.swift`, FFI `studio_export_dir` (store meta `studio_export_dir`) |
| SD01 | Studio in front, Edit menu | Undo/Redo read "Undo Change “primary”" and run the draft's steps | `StudioUndoManager.swift` |
| SD01 | `studio-do open <nonexistent>` or a file that does not parse | the notice "That theme could not be opened." | `StudioSession.switchTo` |
| SD01 | Theme menu ▸ This Project / Save ▸ Use in this project… in a terminal inside a project with `.wormhole/` (trusted, theme on) | the project's `theme.json` is written (the one standalone theme the shown appearance works out to); untrusted: the sentence "Trust this project first…"; no terminal in a project: "…isn't in a project with a .wormhole folder." | `StudioProjectSource.swift`, FFI `files_panel/project/theme.rs` |
| X14 | Put `.wormhole/theme.json` in a project (switch on), put a terminal there in front | the whole app uses that theme (primary kept from the person's choice) until another terminal comes to the front; switch the project's Theme off: the person's pick returns | `ProjectSettingsFollower.swift`, `ThemeStore.projectTheme`, FFI `resolve_theme_json`, `theme_overlay` |
| X14 | Put a Zed icon theme folder in `<data>/icon-themes/`, turn on the Icon themes flag, pick it under Settings → Appearance; then "None"; then the flag off | Files/Changes rows draw its icons (mono ones tinted); None draws the system's symbols; flag off draws Wormhole Icons | `Wormhole/Look/CoreIconTheme.swift`, `SettingsApplier.applyIcons`, `FileIconTheme/FileIconSource.swift`, FFI `icon_theme.rs` |
| X13 | see the fonts section of `evidence/s3-themes-fonts/X13.md` (checked in an instance by the fonts sub-lane) | | |
| SD02 | see `evidence/s3-themes-gallery/SD02.md` (checked in an instance by the gallery sub-lane); not built: icon tabs specimen (the tab is WindowUI's), status bar/resize handle specimens | | |
| X01 | Click +, refresh, copy, split buttons, a menu chevron, a panel tab with Animate icons on; then off; then Reduce Motion on | the icon turns/spins/fills to a check/flips; nothing moves with Animate icons off or Reduce Motion on (hover-only ones: MANUAL.md "icon motion") | `DesignSystem/IconButton.swift`, `UITests/IconMotionTests.swift` |
| X05 | Increase Contrast on: theme cards, segmented controls, tab selections, cards, labels/badges, title row with a project colour | outlines on those; the project tint is a line in the title row | `DesignSystem` `contrastOutline`, `Segmented`, `SectionLabel`, `PanesUI/Title/TitleRowView.swift`, `SettingsUI/Rows/ThemeCard.swift` |
| X06 | (covered by SD01 line on `setTheme`) | | |
| X01 | `+` in the tab bar, sidebar title row, group head | the + turns a quarter (~0.2 s) | `DesignSystem/IconMotion.swift`, `WindowUI/TitleButton.swift`, `SidebarUI/Group/HeadControls.swift` |
| X01 | Changes tab refresh; a copy icon (diff note, commit detail) | arrows go once round (~0.5 s); the sheet fills to a check ~1 s (copy check and Usage gauge not driven) | `PanelUI/Shell/BandButton.swift` |
| X01 | Split right / down; pane zoom button; Open in… chevron | half of the icon fills; old icon shrinks and new grows; chevron up while open (not driven) | `PanesUI/Title/SplitButtons.swift`, `PanesUI/Panes/PaneButton.swift`, `PanesUI/Title/OpenInButtons.swift` |
| X01 | Right-panel tabs, one after another; Animate icons off; Reduce Motion on | the clicked tab's glyph plays once; nothing animates with the gate off | `WindowUI/PanelTabBar.swift`, `DesignSystem/TabGlyph.swift` |
| X05 | Increase Contrast with a coloured group selected; Split\|Unified segmented control; theme cards; diff preview | 2 pt group line in the title row; outlines on track, active segment, cards, preview (segmented test edit never compiled) | `WindowUI/TitleTintLine.swift`, `DesignSystem/ContrastOutline.swift`, `Segmented.swift` |

## Lane menus, wave 3 (round S)

Not run in an instance. Compiles; the one focused test is core's `workspace::tests::close::a_program_in_a_folded_group_is_asked_about_and_a_failed_command_is_not` (plus the mapping of the new fields in `WorkspaceBridgeTests/MappingTests`). The close question's one entry point is `AppController.closeTerminal(_:in:fromCommandLine:then:)` (`Wormhole/AppController+Quit.swift`), which views reach as `hub.actions.closeTerminal(id)`; groups go through `hub.actions.closeGroup(id)`. A keyed menu in a test instance is kept for `menu` / `menu-pick`, never shown.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| W08 | start `claude` (or any agent, or `top`) and let it sit idle at its prompt; ⌘W | the question: "Still running: claude. Closing ends the process."; the same terminal makes Close Window and ⌘Q ask | core `workspace/close.rs asks_before_closing`; `Wormhole/AppController+Quit.swift closeTerminal`, `asking(in:)` |
| W08 | run `false` (or a failing `cargo build`) in a plain terminal; ⌘W, then ⌘Q | closes and quits with no question; no "Still running: … · exit 101" anywhere | same |
| W08 | two terminals in a group, `sleep 300` in one, fold the group; then ⇧⌘W in a second window, ⌘Q, and Close group from the head's menu | each asks, names "sleep 300" and counts both terminals; with the sidebar's search typed so no row is listed, the same | `AppController+Quit.swift quitFacts`, `closeGroup`, `askToClose`; `AppController+WindowEnding.swift shouldClose` |
| W08 | select a terminal, fold its group (or filter it away by the search), ⌘W; also the pane's × and `wh close <id>` | the terminal closes (asking if something runs); before, nothing happened | `closeTerminal` (reads `state.all`) |
| W08 | window with an agent working and a plain `npm run dev`: Close Window, Close group, ⌘Q | "An agent is still working. Still running: claude, npm run dev." (the agent's program is named too) | `TerminalSummary.runningFact`, `command` |
| W08 | ⌘W on a running terminal twice quickly; ⇧⌘W twice | one sheet; the second request is dropped, not queued behind the first | `WindowSession.asking` |
| W08 | run `sleep 300`; look at the red button; let it end | the dot (document-edited mark) is in the close button while closing the window would ask, and goes when nothing runs | `AppController+WindowEnding.swift followCloseDot` |
| W08 | a window (not the last) or a group whose only stake is a terminal with a host that cannot be shown: close it | it asks (the host counts as running) | `asking(in:)` |
| W08 | Persistent terminals on; close a hosted terminal with "Leave It Running", or leave a host of another profile running; Quit and End All Terminals | `Wormhole sessions list` shows no live host of the data folder afterwards | `ffi/src/hosting/desk.rs finish` (`hosts::end_all_live`, 5 s each, on the quitting thread) |
| W08 | `kill <pid>` of a test instance with terminals open | it quits without a question, the windows saved (the next launch restores them), hosted terminals left running | `AppController+WindowEnding.swift quitOnSigterm` |
| W08 | restart-to-update and Restart When Idle with a running terminal in a folded group | the restart question lists it; "when idle" waits for it | `runningTerminals`, `Aux/UpdateFollower.swift allTerminalsIdle` |
| O02 | terminal has the keys; ⌥⌘/ then Esc; ⌘⇧P then Esc; palette then ⌥⌘/ then Esc | typing goes to the terminal again each time, with no click; with the keys in the sidebar's list before, they return to the list | `WindowUI/Hub/WindowHub.swift send`, `Focus/WindowFocus.swift overlayChanging`, `overlayClosed` |
| O02 | open the sheet, click an empty part of the card (the field loses the keyboard), press Esc | the sheet closes | `WindowFocus.swallows` |
| O02 | `--onboarding` (first-run page up), ⌥⌘/ | no sheet opens under the page | `Actions/ActionRouter+View.swift keyboardShortcuts`, `OnboardingWindow.isOpen` |
| O04 | right-click a group's head in the sidebar (real window) | the native menu: Rename…, one row of nine colour dots in the hues of the heads' dots (the worn one ringed; a click sets the colour and closes the menu), Other colour…, No colour, the folder items, the five actions. It replaces the SwiftUI menu with the palette row (`SidebarUI/Menu/GroupMenu.swift`, deleted), so the platform lane's O04 line about the dots is answered here. ⌃↩ on the head shows the same menu, dots included | `Wormhole/Menus/GroupMenuKeyboard.swift groupMenu`, `runGroup` (`.colour`), `SidebarUI/Group/GroupHeadView.swift`, `SidebarHooks.groupMenu`, `DesignSystem/NativeContextMenu.swift` |
| O04 | Tabs on top, open the group switcher, right-click a group's card | the group's menu (the keyboard's native one) | `PanesUI/Tabs/GroupSwitcherView.swift`, `Wormhole/Menus/GroupMenuKeyboard.swift groupMenu`, `DesignSystem/NativeContextMenu.swift` |
| O04 | F6 to the title strip (the terminal's tab strip), ⌃↩ | the folder's path menu under the strip; with Folder hidden in the title row, the row's own menu | `PanesUI/Title/TitleKeyMenu.swift`, `WindowUI/Focus/WindowFocus+Keys.swift openStripMenu` |
| O04 | keys in a panel list (Changes), ⌃↩ | nothing is typed into the terminal | `WindowFocus+Keys.swift listKey` |
| O04 | Zen mode (or a window of a profile that is not the shown one); right-click the title row | parts, Reset and Customize are greyed | `TitleActions.canEditRow`, `WindowHub+Title.swift`, `HubActions.editsSettings` |
| O04 | title row menu ▸ Customize title row… | Settings opens on the Title row setting, revealed | `HubActions.revealSetting` |
| O04 | a window too narrow for another pane (or a diff over the terminal); right-click the terminal | Split Right / Split Down are greyed | `Wormhole/AppController+Context.swift terminalMenu` |
| O04 | History: open a commit with a long message and a parent in the log; right-click its details block | Show more (Show less once open), the five copies, Open Parent <short> per listed parent | `PanelUI/History/CommitDetailsView.swift menu` |
| O04 | History: keys on the open commit's details row, ↩ | that menu under the row | `HistoryStore+Walk.swift press`, `showKeyMenu` |
| O04 | Files tab: right-click a file ▸ Open in, then a folder ▸ Open in | a file lists only the editors Wormhole knows and Finder; a folder every app; each with its icon | `PanelUI/Files/FileRowMenu.swift`, ffi `open_in_known` |

Unfinished or not built in this lane:
- Dropped on the rebase because the sidebar, platform and settings lanes had landed the same behaviours: the row menu's greyed groups, Show beside by room and Copy path notice, the row's Close asking, the group head's ⌃↩ menu and in-place rename, the + buttons' shell menu, File History's gate, the settings row menu. Their lines are in those lanes' sections.
- O04, not built by any lane: "Other colour…" in the group's menu opens the group's popover, not the system colour picker (`platform::pick_colour`).
- O04: the native-menu catcher (`nativeContextMenu`) takes right-click and ⌃-click only when AppKit's current event is one; check that a plain click, a drag and hover on a group head, the + buttons and the commit details still work, and that ⌃-click opens the menu.
- W08: `hosts::end_all_live` runs on the main thread as the app quits (GPUI: off it); with an unresponsive host End All can take up to 5 s per host.
- W08: a quit asked while a close question is up is not dropped (GPUI's `quit_prompt`); the Dock's Quit still asks (deliberate, as before).

## Lane launch (S3): PL07, PL09, A07, X15, A01 to A06

| row | what to do | what must happen | file |
|---|---|---|---|
| PL07 | `control open-url file:///<folder>` and `file:///<x.command>` in an instance | a terminal in the folder; the script's quoted path typed and run in its folder; https URL and a missing file refused. TEST (script run) | ffi/src/hook_server/control/open.rs, Wormhole/SystemOpen.swift |
| PL07 | bundled build: set as default terminal, double-click `.command`; Open With a folder; folder on the Dock | new terminal as above; macOS shows the privacy sentences of Info.plist for a mic/Desktop access | Wormhole/Info.plist |
| PL09 | `kill -TERM <pid>` of an instance | quits as Cmd-Q without the question; windows saved, hosts let go | Wormhole/AppController+WindowEnding.swift `quitOnSigterm` |
| PL09 | second launch of the same data folder (`control second-launch hide` hides windows first) | a window opens when none is visible | SystemOpen.swift `activateFromSecondLaunch` |
| A07 | edit config.json `agent_access` Off->Open (or Off->Ask, Ask->Open) by hand with the Automation flag on | request still judged by the old mode; sheet "changed outside Wormhole, keep it?"; Don't Keep writes the old mode back; no answer in 60 s = not kept; edits in Settings, narrowing: apply at once. TEST | ffi/src/aux_backends/access/modes.rs, hook_server/core.rs `Seat::settings`, Wormhole/Aux/AccessDriver.swift |
| A07 | a control request question left unanswered | after 60 s the client gets ask_timed_out and the sheet goes | ffi/src/aux_backends/access/mod.rs `next_question` |
| A07 | two profiles open | a profile's question goes on that profile's window, none open = not answered | Wormhole/Aux/AuxController.swift `window(ofProfile:)` |
| X15 | none yet (never runs on a `Wormhole-swift*` folder) | old hooks replaced, default-terminal role of the old bundle id taken, data folder moved. TEST (never touches a real folder) | ffi/src/legacy.rs, Wormhole/Platform/LegacyLaunch.swift |
| A03 | release build (`WORMHOLE_UPDATE_URL` set at FFI build): Settings > Updates | feed, key, team, fallback baked; channel from Settings; test instances ignore the baked feed. TEST (feed/signature) | ffi/src/aux_backends/updater.rs `update_release_setup`, Wormhole/Aux/AuxBackends.swift `updateSetup` |
| A03 | Restart Now with a command running; Restart When Idle then Don't restart | question first; "waits" status and Don't restart button | SettingsUI/Panels/UpdateStatus.swift, Wormhole/Aux/UpdateFollower.swift |
| A03 | security release feed; channel switch with an Unstable build installed | severity line and GHSA links (opened through ExternalOpen); channel note | updater.rs `release_record`, DesignSystem/ExternalOpen.swift |
| A03 | replace the running bundle from outside | Installed phase and the chip within a minute | updater.rs `watch_bundle` |
| A03 | two windows, ready chip | chip follows the front window; x closes it for that version everywhere | UpdateFollower.swift `keepOffered`, TerminalUI/Notice/NoticeCenter.swift `onDismissed` |
| A04 | Help > Session Hosts | cards start folded (badge, note, id, program, problem lines), click or Return opens; link/heard/quiet facts for attached hosts; badge = Persistent terminals setting; attachments re-read each look | AuxWindows/Hosts/HostCardView.swift, ffi/src/aux_backends/hosts.rs |
| A01 | `control welcome cursor` / `logo`, `control welcome-dock` | ship lands on the real `>` / logo chevron cell | Wormhole/Aux/AuxController+Intro.swift, ffi welcome_dock |
| A02, A06 | Licenses window; project commands window | texts in the code font; a second project replaces the first in the open trust window | AuxWindows/About/PackageDetail.swift, Trust/TrustWindow.swift |

## Orchestrator (merge fixes)

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| A07 | TEST: start with `agent_access` "ask" and the Automation flag off; while running, edit config.json to turn the flag on and set `agent_access` "open" in one write; make a request | the earlier mode stays in force and the keep-it question is put | `crates/wormhole-ffi/src/hook_server/core.rs` `Seat::new` |
| T20 | Persistent terminals off: `control hosts` for a terminal in the app | no hosting state (`off`), and the Session Hosts badge is off | `WorkspaceBridge/RustWorkspaceBackend+HostProblems.swift` `hosting(of:)` |
| A01 | build with Xcode 26 on a clean checkout, play any intro | the intro draws (its shader files are in the bundle) | `macos/scripts/build-ffi.sh` (copies `Resources/shader`) |

## Lane studio, wave 4

Written without compiling: expect compile errors.

The gallery against GPUI's (`crates/wormhole/src/studio/gallery/mod.rs` `SPECIMENS`, 21 specimens; core `TOKEN_USES`, 49 uses).
Before: 27 Swift specimens, 40 of the 49 uses marked. GPUI specimens or parts with no Swift specimen: the status bar (in
Sidebar), the icon tabs (in Right panel and Tabs and chips), the quick panel, the resize handle with its readout, the size
readout and the block readout (all four in Notifications), the pane drop zones and the dragged divider (in Panes). Uses not
marked: `status.bar`, `icon.tab`, `readout`, `readout.block`, `resize.handle`, `tooltip`, `menu`, `dialog`, `scrollbar`.
GPUI's outline and "Used by" come from one table (`Env::mark` over `TOKEN_USES`); a mark has no click of its own (only the
swatches select a token), and Swift does the same through `.marked(id)` and the generated `GalleryUses`.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| SD02 | Open Theme Studio, area All or Sidebar | A "Status bar" specimen under the columns: the real bar twice (wide with words, narrow with glyphs and counts), the waiting terminal's button, the state and "120×40" at the end; clicks do nothing | `WindowUI/WindowSpecimens.swift` (`StatusBarSample`, `SampleWindow`), `StudioUI/Gallery/GalleryExtras.swift`, `Wormhole/Platform/StudioLauncher.swift` (sets `GalleryExtras.view`) |
| SD02 | Area Panel | An "Icon tabs" specimen: the right panel's six tabs, Changes selected with 3, Usage with 2 | `WindowSpecimens.view(for: "icontabs")`, `WindowUI/PanelTabBar.swift` |
| SD02 | Area Ambient, "Readouts" | A divider line with a handle that lights after the pointer rests on it and turns active while dragged (nothing moves); beside it "430 · max", "350" and "Close" on the tooltip fill; a "120 × 40" size chip that stays whole (no fade, no clock running); block readouts "exit 0 · 380ms"-like, a red exit 101 and one without a code, each with Copy output that does nothing | `StudioUI/Gallery/ReadoutsSpecimen.swift`, `WindowUI/Columns/ResizeReadout.swift` (moved out of `ColumnHandle.swift`), `TerminalUI/Scaling/SizeReadoutChip.swift` (`held`) |
| SD02 | Area Ambient, "Command palette", press "Show the quick panel" | The menu bar's quick panel on its card: search row with ⇧⌘Space, a waiting agent, a working, a failed and an idle terminal, New terminal and Open folder, the footer's hints; Esc or a chosen row closes it, typing narrows the rows; the Studio's own fields keep the keyboard until the button is pressed | `StudioUI/Gallery/PaletteSpecimen.swift` (`quick`), `Package.swift` (StudioUI now uses PlatformModel and PlatformUI) |
| SD02 | Area Chrome, "Panes" | Under the two headers a third divider in the active colour, then a pane with the right side lit ("Show on the right"-like pill) and the other three sides as dashed hints | `PanesUI/Samples/PaneSpecimen.swift` (`dropZones`) |
| SD02 | Select `tooltip.background`, then `resize.handle.active`, `agent.attention.tint`, `state.selected` in the token list | The dashed outline goes round the readouts; the handle; the status bars; the icon tabs (each tab) and whatever else draws it. The editor's "Used by" lists "resize readout", "command block readout", "resize handle", "status bar", "icon tab" | `.marked("status.bar")` `WindowUI/StatusBar.swift`, `"icon.tab"` `PanelTabBar.swift`, `"resize.handle"` `ColumnHandle.swift`, `"readout"` `ResizeReadout.swift` and `SizeReadoutChip.swift`, `"readout.block"` `TerminalUI/Blocks/BlockReadout.swift`; `macos/scripts/gen-gallery-uses.py` (five ids, `chrome()` reads `ChromeColours`) |
| SD02 | In a real window with the Studio closed: drag a column edge, resize the window, hover a finished block, look at the status bar and panel tabs | Nothing new is drawn round them (the marks are empty outside the gallery) and the size chip still fades after 700 ms | the same marks; `SizeReadoutChip(model:held:)` defaults to false |
| SD02 | `python3 -I macos/scripts/gallery.py` | ok; GalleryUses.swift lists 47 components | `StudioUI/Gallery/GalleryUses.swift` (regenerated) |

Not written, and why:

| row | what | why |
|---|---|---|
| SD02 | `tooltip` specimen and mark | The Swift app has no tooltip of its own: every one is the system's (`.help`), which the theme does not colour. `tooltip.background`/`tooltip.text` are shown by the readouts |
| SD02 | `menu` specimen and mark (the select's opened menu with its search row) | Selects and menus are the system's (`Picker`, `NSMenu`); the Menus specimen shows the triggers. No themed menu view exists to show |
| SD02 | `dialog` specimen and mark (the close question on its scrim) | The Studio's and the window's questions are `NSAlert` sheets (`StudioQuestions.swift`). The SwiftUI sheets on `surface.overlay` (new profile, probe, worktree) need their own stores and are not GPUI's dialog |
| SD02 | `scrollbar` specimen and mark | Lists scroll with the system's scrollers; `scrollbar.thumb` is drawn only by the minimap and the diff's overflow cue, which GPUI's gallery does not show under this use |
| SD02 | Windows: first-run sheet's card with step dots, About, Licenses list | `AuxWindowsSpecimen` shows the hosts' cards, badges and buttons only; the other windows' views take their models (not compared part by part in this wave) |
| SD02 | Tiles and tabs: the status change caught mid-way per variant, counts half turned (`rolling_number`), the tile on a selected row | No Swift rolling number exists; the status change (`SidebarUI/Tile/StatusPlay.swift`) has no way to be held at a point in time |
| SD02 | Chrome: the hit area drawn round a small glyph, the context meter and agent gauge, row-end buttons pulled out | No Swift view of these names (`hit_area`, `context_meter`, `agent_gauge`); not searched further |
| SD02 | Tabs and chips: path crumb, drag ghost and lifted card, "Used by" chip, split mark; Sidebar: the drop rows (row target, drop line), chain mark | Not added: the sidebar's drag views (`SidebarUI/Drag`) are internal and driven by a live drag; no path crumb view found |
| SD02 | The specimens the window draws are empty in `StudioTests` and previews | Only the app sets `GalleryExtras.view`; the snapshot test sees the titles with nothing under them |
| SD02 | TEST | none: no data, trust or wire format touched |

## Lane panel-files, wave 4

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P07 | TEST. Window of profile Work (Wormhole in Default): pin a folder in Files, rename a pinned folder; then "Move this window to" another profile | the pin is written to Work's `config.json` and read from it, never Default's; the renamed folder's pin moves there; after the move the pins are the new profile's | `ffi/src/files_panel/files.rs follow_profile, paths`, `settings_edit.rs update_settings_of`, `FilesBridge.makeBackends`, `ProfileChipBinding.swift` |
| P07 | TEST. List a folder, then replace a listed file (and a listed folder) by a link that leads out of the root; Open in (row menu, viewer button, binary panel), Reveal, the viewer's crumb | nothing is opened or shown: the core resolves the path inside the root first and it must still be a file (folder) | `files.rs resolve`, `PanelUI/Stores/FilesStore+Open.swift`, `FileRowMenu.swift`, `FileViewerBody.swift`, `FileViewerView.swift` |
| P07 | Rename: type a name with a leading space; press return twice quickly; click elsewhere while the rename runs; rename a folder that holds the selected row and the open file | the name goes to the core as typed (its refusal is the chip's sentence); one rename only; the field stays until the answer; selection and viewer follow to the new path | `PanelUI/Stores/FilesStore+Rename.swift` (`renameBusy`) |
| P07 | Click the path in the viewer's header; right-click it | Finder shows the file; a menu with Reveal in Finder and Copy Path. NOT BUILT: the path card on resting the pointer | `PanelUI/Files/FileViewerView.swift FileViewerHeader` |
| P07 | View a file with long lines, scroll sideways | the line numbers stay at the left edge over the code | `PanelUI/Files/FileViewerBody.swift FileLines` (`across`) |
| P07 | Panel closed, or another tab shown, or the Files feature off: change files in the terminal's folder; then show Files; open a file, switch to Notes, edit the file outside | nothing is listed or watched while hidden; showing the tab lists; the open file over the terminal still follows the disk | `FilesStore.setShown, startReading, stopReading`, `WindowUI/WorkspaceRootView.swift filesShown` |
| P08 | TEST. Turn the Project feature off (flag or switch) with a trusted project in front: Settings overrides, theme, a worktree of the repository, notes.md, a colour pick, Project Actions menu | the overrides and theme are unwound at the next read, the worktree has no trust condition and runs no setup, notes.md is not read or written, the colour goes to the person's own settings, an action is refused by the core | `ffi/src/files_panel/project/service.rs set_project_feature_on, off`, gates in `overrides.rs`, `theme.rs`, `notes_sync.rs`, `paint.rs`, `service.rs fresh`; told by `Wormhole/Panel/NotesCoordinator.swift changed` |
| P08 | TEST. Trusted project with Actions on and an action with `"key": "cmd-ctrl-t"`: press the key with the terminal focused; revoke trust, press again | the action runs (the core checks trust again at the run); untrusted, the action is not listed, so no item has the key. An F-key alone depends on the menu taking it as an equivalent | `PlatformBridge/CoreMenus.swift keyed(by:)`, `PlatformModel/Menus/MenuState.swift ProjectAction.key`, `Wormhole/Menus/RecentProjects.swift` |
| P08 | Break `trusted-projects.json` of the profile, open a folder with no project; open an older project with `.andromeda/` | the no-project card says the stores were reset; the head reads `name/.andromeda` | `service.rs problem`, `ProjectStore.problem`, `NoProjectCard.swift`; `records.rs dir`, `ProjectHeadCard.swift` |
| P06 | TEST. Trusted project with Shared notes: write in a note while `notes.md` changes outside; then the same with the keys in the search field or another app in front and no note open | while a step, the draft step or a note of the shown terminal has its editor open the merge waits; any other field or app in front does not hold it back, and never merges over a note being written | `NotesStore.isWriting, setWriting`, `TaskRowView.swift`, `NoteRowView.swift`, `NotesCoordinator.sync` |
| P06 | TEST. Notes feature off, or Project off, in a trusted project with Shared notes | the sync timer does not run: notes.md is neither read nor written | `NotesCoordinator.changed, offers` |
| P06 | ⇧⌘E from the terminal: with a note written in before, with notes but none written in, with no note; panel closed | Notes tab and panel shown, section unfolded, that note (the first; a new one) has the keys; ⇧⌘E again goes back to the terminal | `WindowUI/Focus/WindowFocus.swift toggleTerminalAndPanel`, `NotesStore.focusIn` |
| P06 | In the step being written: ⌘↩, ⇧⌘↩, ⌫ on the empty field, hover Send and ✕ | what is typed is pasted (and submitted); the draft goes and the step above opens for editing; Send pastes, ✕ drops what is typed | `PanelUI/Notes/TaskDraftRow.swift`, `NotesStore.backOutOfDraft`, `StepsCard.swift draft` |
| P06 | Edit a step, change its words, press the row's Send before return | what is typed is pasted, not the saved words | `TaskRowView.swift` |
| P06 | Select text in the terminal, ⌥⌘S; a note of spaces only | the selection becomes the note (before the agent's input box); the band's count does not count the blank note | `WindowUI/Hub/WindowHub+PanelHost.swift takePrompt`, `PanelModel/Notes/NotesModel.swift filledNotes` |
| P06 | Show all on a long note, fold it, open it | it is cut again | `NoteRowView.swift` |

Not written:
- P06: the preview cut by wrapped lines with the two fading lines (needs text layout measured in the row; not done blind); a note's body still by Swift's copy of `note_body`, the cursor at the end on a title click, and ⌥⌘↑/↓ read off the keystroke.
- P07: names that are not UTF-8 (the records carry text; needs the bytes or an index through every call); the path card on resting on the crumb; tree-sitter colours (counted under P03). The icon theme was already there (`FileIconSource.shared`).
- P08: per-terminal kept state is still not dropped when a terminal closes; a folder's own colour (`Paint::Own`) is still read from the Default profile's settings; the action key is a menu key equivalent, not the GPUI app's check before the terminal sees the key. Already there before this wave: the project's theme, env and shell for new terminals, the window's own profile's trust.

## Lane settings, wave 4

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| ST02 | View ▸ Zen Mode, then Settings → Appearance, Terminal, Fonts | Appearance and Terminal carry the note "Zen mode is on, so the window shows less…" with Leave Zen mode; Fonts has none; the button ends Zen mode and the note goes | SettingsUI/Pages/ZenNote.swift, SettingsPage.swift, SettingsModel `zenOn`/`leaveZen`, Wormhole/Settings/ZenMode.swift, AppSettings.swift |
| ST02/ST03 | Advanced → Show config keys on; move the pointer over a row; hold ⌥; click a key | The key shows only under the pointer, on every row while ⌥ is held (window in front), and while it says Copied; off, no key whatever is held; the row's menu works either way | SettingsUI/Rows/SettingRow.swift, RowState.swift (`\.optionHeld`), SettingsView.swift `onModifierKeysChanged` |
| ST02 | Feature flag Motion on; Motion → The sidebar and panel hide or show: each choice, then toggle the sidebar and the panel; toggle back mid-slide | Classic 250/200 ms; Soft spring both ways on 170/26; Content after fades the column out (80 ms) before it goes and in after it begins to come; flag off: Classic whatever the file holds; Reduce Motion: cuts. Known difference: Content after fades the whole column with its surface (GPUI keeps the surface) | WindowUI/Columns/ColumnSlide.swift, WindowBody.swift, WindowLook `columnMotion`, SettingsApplier.swift |
| ST02 | Settings → Motion (flag on), wide and at the window's least width; pick each option; Reduce Motion on | Four cells (two by two when narrow) loop every 4 s, each from core's poses; a pick restarts only its cell and rings it; Reduce Motion holds the first picture and stops the clock; captions are the rows' labels | SettingsUI/Preview/Motion/*.swift, PreviewScene.swift, PreviewStrip.swift, WorkspaceModel/RowMotion.swift, WorkspaceBridge/CoreRowMotion.swift, crates/wormhole-ffi/src/motion_scenes.rs |
| ST02 | Appearance → theme cards, Show all; filter by a family's name | A family with one version on a shelf is named by the family ("Soltari"), with several by each version's name; the filter finds a card by either name | SettingsUI/Rows/ThemeCards.swift `label`, ThemeCard.swift |
| ST02 | Not written: the Motion cells' own VoiceOver labels (the strip is one labelled group, as the other scenes); the ring following the row under the pointer or with an open menu (it follows the option changed last) | | |
| ST02/ST03 | Found already written (not changed here, still to verify): Updates' Restart Now question, channel note, security line, bundle-replaced watch; the Shells list, worktree caption and "zsh" face; the Changed page's Copy as JSON ("Copied"); the "changes are not saved" banner; Reset tips disabling; the Terminals list scene's three densities | | UpdateStatus.swift, EntryRowView.swift, ChangedPage.swift, SettingsPage.swift, EntryControl.swift, SidebarScene.swift |
| ST02/ST03 | Key audit (96 keys of the 14 pages against the Swift sources and the FFI): the one key without a consumer was `motion.column`, wired above; the terminal keys (scrollback_lines, default_shell, shells, image_memory_mib, allow_*) are read by the core at launch (`launch.rs`) | | macos/parity/punch-list.md row ST02 is out of date |

## Lane panel-git, wave 4

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P04 | filter History to a file (File History) or pick another branch, then pick a commit in the palette that the shown log does not list | the filter chip goes, the log goes back to HEAD when the commit is not on the picked branch (always from Stashes), the commit is open with details and files, its row scrolled to. TEST: `is_on` gives false for a hash git does not know | `PanelUI/Stores/HistoryStore.swift reveal`, `Wormhole/Platform/PaletteSourcing.swift`, `PaletteSourcing+Listed.swift`, `ffi/src/git/asked.rs is_on` |
| P04 | leave History showing with a commit a few minutes old | "3m" becomes "4m" on the minute without touching anything; no timer runs with the tab hidden or the log empty | `PanelUI/History/HistoryView.swift .task`, `HistoryStore.nextAgeChange/tick`, `ffi/src/git/asked.rs git_next_age_change` |
| P04 | open a commit with many branches and tags, and a merge, in a narrow panel | ref chips and parent links wrap onto more lines, names whole, "+N" last | `PanelUI/Shell/WrapLayout.swift`, `History/CommitRefs.swift`, `CommitDetailsView.swift parents` |
| P04 | open an old commit one of whose files was deleted since; hover its files | the open icon only on files that are regular files in the working tree now; none past the first 2,000. TEST (hostile input): a path with `..`, or a link leading out of the repository, is not present | `HistoryStore.present`, `HistoryView.swift details`, `ffi/src/git/asked.rs present_files` |
| P04 | move a tag or branch onto the open commit from another terminal | its ref chips update (the log's refs are compared, not only the ids) | `HistoryStore.refresh sameRefs` |
| P04, P02 | open either picker | the first choice reads "HEAD (current branch)" (History) or "HEAD (uncommitted changes)" (Changes); "Loading branches…" under the list until it is in; Changes then says "Changes since this branch split off from it"; selecting another terminal closes the picker | `PanelUI/Shell/RevPicker.swift`, `ChangesStore.loadRevs`, `HistoryStore.loadRevs` |
| P04 | a pull request whose checks are running finishes them (title row shows the PR part) | the first terminal on that branch gets "Checks passed on #N" / "Checks failed on #N" with the branch as body, through the notification rules (as `wh notify`) | `ffi/src/workspace_extras.rs tell_checks_settled`, `core/src/workspace/extras.rs first_on_branch` |
| P04 | TEST (trust): a window in a second profile, the project trusted only in the Default profile (and the reverse) | `gh` is started only where the window's own profile trusts the project and has project folders on | `ffi/src/workspace_extras.rs profile_of, approved` |
| P03 | from History open a file's diff | the commit's short hash follows "File N of M" in the primary, also in the title row's header | `PanelUI/Diff/DiffHeader.swift commit`, `PanelCoverTitle.swift` |
| P03 | open the diff of a submodule whose pointer moved forward by a few commits, back by some, by more than 200; one not checked out; one with uncommitted changes inside | "abc1234 → def5678" in the code font, "New commits" / "Commits left behind" each with its total, a two-line row per commit (subject; hash, author, age), "and N more", core's sentence when nothing is listed and what is inside; the body scrolls. TEST (wire): `GitDiff::Submodule { label, card }` | `PanelUI/Diff/SubmoduleCard.swift`, `PanelModel/Diff/SubmoduleCommits.swift`, `GitBridge/GitDiffFile.swift`, `ffi/src/git/service.rs submodule_diff`, `records.rs GitSubmoduleCard` |
| P03 | compare with a branch, open a diff, edit the file so its counts stay the same; then fetch so the merge-base moves | the open diff is read again both times | `ChangesStore.stamp(of:)`, `keepOpenDiffInStep`, `GitStatus.base_rev` |
| P03 | "Load diff" on a very large patch, unwrapped, scroll sideways | no walk over the lines per scroll step: the rows of both layouts are cut once when the diff is read | `PanelUI/Stores/DiffSession.swift DiffBuilt`, `Diff/DiffView.swift lines` |
| P02 | a repository with an untracked folder that is itself a repository (git lists "vendor/lib/") | a folder icon, the name "lib", its parent "vendor" as the folder, no "+0" or "bin" | `PanelModel/Changes/ChangedFile.swift isFolder, folder, lines`, `PanelUI/Changes/FileRowView.swift`, `LineCounts.swift` |
| P07, P02 | a terminal in a subfolder of a repository | the Files tree and the project's colour are the repository root's: `GitBackend.root(of:)` was only an extension method, so a call through `any GitBackend` always gave nil; it is a requirement now. Check that nothing relied on the nil | `PanelModel/Backends/GitBackend.swift`, `PanelUI/Controller/PanelController.swift follow` |

Not written:
- P03 syntax colours from tree-sitter: the parser and its grammars are in the app crate (`crates/wormhole/src/syntax`, with GPUI); bringing them under the bridge is a dependency change that has to be built to be right. The Swift lexer stays.
- Already there before this wave (the audit's list was older than the code): History's copy notices, Copy error's chip, the details menu and ↩ on its row, Esc in the picker.
- Two tests were only kept in step with the changed wire (`Tests/GitBridgeTests/MappingTests.swift`, `crates/wormhole-ffi/tests/git.rs`); none was written.

## Lane sidebar, wave 4

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| S03 | Right-click a group's head (or Ctrl-Return on it), "Other colour…", drag in the Colours window (a visible instance: a background one opens the popover instead) | the system's Colours window opens on the colour the group shows (its own, else its named one, else mid grey); the group follows every step of the drag; "Other colour…" is ticked afterwards | `Wormhole/Menus/GroupColourPanel.swift`, `GroupMenuKeyboard.swift` `pickColour` |
| S03 | Give a group a colour of its own (the popover's picker or the Colours window): black in a dark theme, white in a light one, a mid colour | the card is washed in it and, with cards off, the head's dot shows it; the card's text still reads (the wash moves in lightness only as far as contrast needs; nothing caps the choice) | `SidebarUI/Colour/GroupPaint.swift`, `Group/SidebarSectionView.swift` |
| S09 | Tabs mode, a terminal running an agent (a `/bin/cat` copy named `claude`) that works, needs input, finishes | its tab's tile is the row's own: the agent's logo on its colour, the light, the badge; with Status dot off every tile is quiet | `SidebarUI/Tile/TerminalStatusTile.swift`, `PanesUI/Tabs/TerminalTabView.swift`, `WindowUI/Hub/WindowHub+Sidebar.swift` `tabActions` |
| S09 | Tabs mode, VoiceOver on a tab (or `a11y`) | "name, state, selected, n of m" in core's words; the sample (no core) says the name only | `ffi/src/workspace_tabs.rs` `tab_labels`, `WorkspaceBridge/RustWorkspaceBackend.swift`, `WindowHub+Sidebar.swift` `tabBar` |
| S09 | Open the group switcher with coloured groups, one with a colour of its own | each card is washed in its group's colour; its tiles are the terminals' own (agent logos), loudest first, at most 8 | `PanesUI/Tabs/GroupSwitcherView.swift` `SetCardView`, `GroupPaint.swift` `cardWash` |
| S09 | In the switcher click a card's name; click elsewhere on the card | the name opens the group's popover under it (name field, colours, folder items, actions; New terminal in group asks the folder question); elsewhere goes to the group. VoiceOver: the card is a button with the actions "Edit group" and, pinned and empty, "New terminal in <name>" | `SetCardView`, `SidebarUI/Group/GroupEditPopover.swift`, `SidebarContext+Away.swift` |
| S09 | In the switcher right-click a card, Rename… | the card's popover opens with the name in its field (the list is not on screen to rename in place). Least sure: whether the switcher popover survives the native menu | `TabBarActions.renameAsk`, `WindowHub+Sidebar.swift` |
| S09 | Two profiles, terminals in the sidebar | the profile chip is first in the sidebar's title row, after the lights and the collapse button; none in tabs mode (the bar has it) or with one profile | `WindowUI/SidebarTitleRow.swift`, `SideColumns.swift` |
| W12 | Profile B with another theme, opacity, density, terminal font and `terminals_in`; Wormhole in A; Profile > Open Window in B | the B window is drawn with B's theme, opacity and blur, sidebar look, tabs or sidebar, status bar, code font, and its terminals with B's font, bell, Option and clipboard rules; the A windows do not change | `Wormhole/Settings/ProfileLook.swift`, `ProfileLooks.swift` (`dressForProfile`), `WindowUI/WindowDress.swift`, `WindowLook.themes`, `WorkspaceRootView.swift` |
| W12 | With that B window open, edit B's `config.json` (theme, `opacity`, `font_size`, `title_row`); then edit A's | only the B window follows B's file, only the A windows follow A's (title row, Open in, sidebar numbers and the panel's switches too) | `ProfileLook.apply`, `Settings/SettingsApplier.swift` `ownHubs`, `LookSettings.swift`, `HubSettings.swift` |
| W12 | Move Window to B, then back to A | the window re-dresses in place (theme, look, its running terminals' fonts and colours) each way; nothing of it is rebuilt | `ProfileChipBinding.swift` `move`, `ProfileLooks.swift` |
| W12 | TEST (wrong profile's data): in a B window flip a panel switch (diff split, wrap) | it is written to B's `config.json`, never to A's | `ProfileLook.dress` (`persist`) |
| W12 | Zen mode on and off with an A and a B window | both take the preset and both get their own row back | `Settings/ZenMode.swift`, `ProfileLook.applyZen` |
| W12 | Switch Wormhole A to B while a B window is open beside; open another B window; close windows | every B window shows B's values (the first on its own look, the new one on the app's); a profile's look is dropped when its last window closes | `ProfileLooks.swift` `dropUnusedProfileLooks` |
| T07 | TEST (data): `seq 1 5000`, wait for a save (a minute, or quit and relaunch once), Cmd-K, then kill the app and relaunch | the cleared lines do not come back. Already in the bridge before this wave (the NOT BUILT line above is stale): Cmd-K is `clear_history`, which reports `Touch::Cleared`, which erases the store's copy | `ffi/src/interaction/pointer.rs` `clear_history`, `workspace_feed.rs` `on_touch`, `workspace_texts.rs` `erase_saved_text` |
| W12 | NOT WRITTEN: the static scales (`Palette`, `Radius`, `TextRole` in DesignSystem) read `ThemeStore.shared`, so a B window's interface font, text size and radius metrics outside `@Environment(\.tokens)` are still the shown profile's; `Keycap`, `ContrastOutline`, `NoticeChip` read the shared store's system switches (the same in every store unless a control command flipped them) | - | needs the store handed down by environment into those scales |
| W12 | NOT WRITTEN: the engine keeps one palette for colour queries (OSC 4/10/11, mode 2031), told by the shown profile only, so a program in a B window is answered with A's colours (it is drawn with B's); the Probes' status items, the file icon theme, the hotkey and the menu bar extra stay the shown profile's | - | `Look/EngineColors.swift` needs a per-terminal call; `HookHost.swift` `probeItems` |
| S03 | NOT WRITTEN: the title row's tint line under Increase Contrast and the title tint for a group with a colour of its own (`selectedGroupColor` is a named colour only) | - | `WindowUI/TitleTintLine.swift`, `Chrome.swift` |

## Lane profile, wave 5

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| W12 | Profile B with another interface font, `ui_font_size` and a theme with other radii and border widths; Wormhole in A; Open Window in B | every text, radius and line of the B window (sidebar, tabs, title row, panel, status bar, palette, shortcut sheet, notices, find bar) is B's; the A windows keep A's. The roles reach a view through `\.tokens` (`tokens.size(.caption)`, `tokens.font(.body)`, `tokens.metrics.radiusChip`); nothing in DesignSystem's components, PaletteUI, PanelUI, PanesUI, SidebarUI, TerminalUI or WindowUI reads `ThemeStore.shared` or `Palette`, `Radius`, `Rule`, `TextRole.x.size/font` any more | `DesignSystem/Scale/TextRole.swift` (`ThemeTokens.size/font/lineHeight`, `RoleFont`), 93 files of those modules |
| W12 | devctl `increase-contrast on`, `reduce-transparency on` with an A and a B window | key caps, the outlines of tinted shapes, the notice chip and the folder picker follow the store of their own window (`\.increaseContrast`, `\.reduceTransparency`, set by `.themed()`) | `DesignSystem/Theme/ThemeEnvironment.swift`, `ContrastOutline.swift`, `SidebarUI/Row/Keycap.swift`, `TerminalUI/Notice/NoticeChip.swift`, `PlatformUI/Picker/FolderPickerView.swift` |
| W12 | Drag a terminal out of a B window | the chip that follows the pointer and the window growing from it are drawn in B's theme and sidebar look | `SidebarUI/Drag/TearOffChipView.swift`, `Wormhole/TearOff/TearOffGrow.swift`, `AppController+TearOff.swift` |
| W12 | TEST (wire). In a B window (light theme) with Wormhole in A (dark): `printf '\e]11;?\e\\'`, `printf '\e]10;?\e\\'`, `printf '\e]4;1;?\e\\'`, `printf '\e[?996n'`; change A's theme, ask again; change B's theme with mode 2031 on (`printf '\e[?2031h'`); the same for a persistent (hosted) terminal; Move Window to A | the answers are B's colours and `CSI ? 997 ; 2 n`; A's change does not reach the B terminal; B's change is reported once to the program that asked; after the move the terminal answers with A's. A terminal no view has dressed (and every terminal at spawn) has the colours of the profile Wormhole is in until its view dresses it | `crates/wormhole-ffi/src/engine_look.rs` `Terminal::set_colors`, `UniFFISource/UniFFIFrameSource.swift` `setEngineColors` (kept for `fill`), `TerminalMetal/MetalTerminalView.swift` `setPalette`, `TerminalCore/FrameSource.swift` |
| W12 | B window, B's theme other than A's: the edge across the top, the status bar's profile tag (dot and name), VoiceOver on the tag | both in the profile's colour as B's theme draws it (it was A's theme's); the tag goes with the words in a narrow window; VoiceOver says "Window in profile B". The edge and the tag were there before this wave | `Wormhole/Settings/ProfileChipBinding.swift` `profileList`, `WindowUI/StatusBar.swift`, `WindowUI/WindowBody.swift` |
| S03 | A group with a colour of its own (`#rrggbb`: black in a dark theme, white in a light one, a mid colour), its terminal selected; then Increase Contrast | the folder's icon and parents in the title row are in the main text colour (as for a named colour; this was missing for both); under Increase Contrast a 2 pt line in the colour, moved in lightness only as far as it takes to be seen on the window (3:1). The GPUI app's bar has no fill of its own any more (`window/title/mod.rs`: "its content stands on the window"), so none is drawn | `WindowUI/TitleTintLine.swift`, `Chrome.swift` `selectedGroupHead`, `SidebarUI/Colour/GroupPaint.swift` `titleLine`, `GroupWash.titleLine`, `PanesUI/Title/FolderCrumb.swift` (`\.titleTinted`), `TerminalColumn.swift` |

Not written, and why:
- W12: the windows that are the app's own keep reading the app's store through the static scales (`TextRole.x.size`, `Radius`, `Palette`): AuxWindows, StudioUI, SettingsUI, FontPickerUI and PlatformUI's quick panel, folder picker and shortcut recorder (175 uses). They are themed with the app's store at their root, so this is what they should show; the statics stay for them.
- W12: the icon theme and Probes' status items are app-wide in the GPUI app too (the Files rows read the global `SettingsStore`'s `icons` and `active_profile::paths().icon_themes_dir()`; the status bar reads the global `ProbeRuntime`), so they stay the shown profile's here. The global hotkey and the menu bar extra are one per app.
- W12: a terminal starts with the colours of the profile Wormhole is in (the spawn options) and gets its window's when its view dresses it, before its program can ask; a B terminal with no view is told A's changes.
- W12: a font file that comes or goes in B's fonts folder moves the app's count (`ThemeStore.shared.fontFiles`), so every window's text is laid out once more, not only B's.

## Lane studio, wave 5

Written without compiling: expect compile errors.

What wave 4 left of the gallery (its "Not written" table above), finished. Every specimen is the app's own view on sample
data; where the Swift app has no view of its own the system's control stands in the gallery with a line saying so.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| SD02 | Open Theme Studio, area All: Status bar, Icon tabs, Readouts' handle | The three specimens of the window's module are drawn as before (nothing empty) | `StudioView<Extras>`, `GalleryView<Extras>`, `SpecimenFrame`, `SpecimenContent.view(for:context:extras:)`, `ReadoutsSpecimen<Handle>` (StudioUI); `StudioWindowController.show(theme:environment:store:extras:)`; `StudioLauncher.show` passes `{ WindowSpecimens.view(for: $0) }` (`@ViewBuilder`, `some View`) |
| SD02 | `python3 -I macos/scripts/standards.py` | No any-view for `WindowSpecimens.swift`; `GalleryExtras.swift` is gone (any-view 19 to 14 in all) | the same |
| SD02 | `swift test --filter StudioTests`, previews | They build with `StudioView(session:store:)` (`Extras == EmptyView`): the window's three specimens are empty there by construction, because the test target has no window module to build them; only the titles show | `StudioView.swift` (the `EmptyView` init), `Tests/StudioTests/StudioSnapshot.swift` (unchanged) |
| SD02 | Area Sidebar, "Tiles and tabs" | Under the three sizes: the list's own tile for a plain terminal (quiet, running, bell, unread, failed), for an agent with its logo (quiet, working with the light, needs input, done, failed) and the agent's again on the selected row's tint. A tile with "Play a status change": each press goes working to needs input or back, played by the status motion in use (badge pop by default; the flare only where Settings chose it and the core's tables are there). Two count badges and "Turn the counts over": the digits roll (12 to 13, 19 to 20) | `SidebarUI/Samples/TileStatesSample.swift`, `StudioUI/Gallery/TilesSpecimen.swift` |
| SD02 | Area Sidebar, "Drag and drop" | Two short lists of three terminals. Left: "sidebar polish" outlined in the primary, the dragged "diff perf" as the compact chip on its lower edge. Right: a 2 pt line in the gap under the first card, the dragged row as a lifted card with a shadow under it, its row dimmed where it stood. Under them the chain mark (a link glyph) before a notes count. Nothing answers a click | `SidebarUI/Samples/DragSample.swift` (the internal `SidebarView(…caught:)`), `RowMarksSample.swift` (`ChainMarkSpecimen`), `StudioUI/Gallery/DropsSpecimen.swift` |
| SD02 | Area Chrome, "Chrome", last line | The row's × in a 28 pt outlined square (hover: its fill and the system tooltip), and the × on the selected row's tint | `RowMarksSample.swift` (`CloseHitSpecimen`), `ChromeSpecimen.swift` |
| SD02 | Area Chrome, "Tabs and chips" | A second title row with the long path as the folder crumb writes it (parents muted), the pull request and ports as far as the width allows; then three gauges: "Opus 4.5" 41 % with a primary meter, 82 % with a warning meter, 96 % without a model | `ChipsSpecimen.swift`, `PanesUI/Samples/GaugeSpecimen.swift` |
| SD02 | Area Ambient, "Windows" | Under the host cards: the first run's card (Step 1 of 3, Skip setup, "Where do you work?", the folder rows, Back, three dots with the first a bar, Continue in the primary, not dimmed), then Licenses (three package rows, the first selected, "3 packages", the package's head and two license texts in the code font), then "Show About Wormhole" with the line that About is the system's panel; the button opens the standard About panel | `AuxWindows/Shared/WindowPartsSpecimens.swift`, `AuxModel/Samples/SampleLicenses.swift`, `StudioUI/Gallery/WindowsSpecimen.swift` |
| SD02 | With the Studio in front and the Windows specimen on the page: press Return, press ⌘O | Neither reaches the specimen's card (no step change, no folder chosen): the card's keys are off in a picture (`stepKeys`) | `StepCard.swift` (`StepFooter`), `FolderStep.swift`, `WindowPartsSpecimens.swift` |
| SD02 | First run for real (a fresh data dir, or Replay intro): Return on a step, ⌘O on the folder step | As before: Return continues, ⌘O opens the folder panel (`stepKeys` is true by default) | the same |
| SD02 | Area Settings, "System controls" | Four parts, each with a muted line saying macOS draws it and a theme does not change it: a label whose tooltip appears on hover; "Open a menu" (a system menu with a destructive item); "Show a question" (the system's alert as a sheet on the Studio, Save / Don't save / Cancel, none does anything); a short list with the system's scroll bar | `StudioUI/Gallery/SystemSpecimen.swift`, `StudioWords.swift` |
| SD02 | Area Settings, "Menus" | Under the three triggers the rev picker's opened list: a search row, HEAD, main with a check, a branch, a tag, a commit with its subject, on `surface.overlay`. The search box does not take the keyboard from the Studio | `PanelUI/Shell/RevListSpecimen.swift`, `RevPicker.swift` (`RevList` internal, `takesKeys`) |
| SD02 | Changes or History in a real window: open the "vs" picker | As before: the popover opens with the search box focused | `RevPicker.swift` |
| SD02 | Select `surface.overlay`, then `scrollbar.thumb` in the token list | The outline goes round the rev list (and "Used by" lists "menu"); round the diff's overflow bar where the Diff specimen's code runs past its column ("Used by" lists "scrollbar") | `.marked("menu")` `RevPicker.swift`, `.marked("scrollbar")` `PanelUI/Diff/OverflowCue.swift`, `gen-gallery-uses.py` |
| SD02 | `python3 -I macos/scripts/gallery.py` | ok; GalleryUses.swift lists 49 components | `GalleryUses.swift` (regenerated) |

Not written, and why:

| row | what | why |
|---|---|---|
| SD02 | `tooltip` and `dialog` marks ("Used by") | The Swift app draws neither: tooltips are `.help`, questions `NSAlert`. A mark on the system's control would claim a token it does not draw. `tooltip.*` is outlined on the readouts; the sheets on `surface.overlay` (new profile, probe, worktree) are forms with stores of their own, not GPUI's dialog |
| SD02 | The status change caught a third of the way per variant | `StatusTile` changes by SwiftUI transitions and the flare by a private animatable; neither can be held at a time. The change is played on a press instead, by the motion in use |
| SD02 | Counts caught half turned | No rolling number of the app's own: `CountBadge` uses the system's numeric transition. The counts turn on a press |
| SD02 | Row-end buttons pulled out (`flush_left`/`flush_right`) | The Swift app has no such helper: its title rows place their buttons by their own insets, so there is no component to show |
| SD02 | About as a themed card | About is the system's standard panel in the Swift app (`AboutPanel`); shown by its button and note |
| SD02 | TEST | none: no data, trust or wire format touched |

Least sure to compile: `DragSample.swift` (the internal `SidebarView` init, `LiveDrag`/`DropAim` values, and the line's place worked out from `SidebarMetrics` rather than measured: it may sit a point or two off the gap); the generic chain (`StudioWindowController.show<Extras>` with a `@ViewBuilder` closure returning `WindowSpecimens.view`'s opaque type, and `SpecimenContent.view`'s non-escaping `@MainActor` closure); `keyboardShortcut(keys ? … : nil)` in `StepCard.swift` and `FolderStep.swift`.

## Lane syntax, wave 5

Written without compiling: expect compile errors.

The owner's call, 2026-11-13: the bridge's own engine stays as it is (the Swift app replaces the GPUI app at 2.0, so it
becomes the only one). Whether Rust, Go, JS, TS, HTML, JSON and Markdown need GPUI Kit's queries and injections to look as
today is judged in the looks pass, side by side; P03 counts as built.

The brief's premise did not hold: `crates/wormhole/src/syntax` has no parser. It is a cache over GPUI Kit's `SyntaxHighlighter`
(`gpui-component` 0.8, third party), whose parse, grammars' queries and injections are written on GPUI's own types (`HighlightStyle`,
`SharedString`, its rope and sum tree). There is no plain part to `git mv`, so nothing was moved and the GPUI app is untouched (no
file under `crates/wormhole`, `crates/wormhole-core` or the root `Cargo.toml`/`Cargo.lock` changed). The bridge got an engine of its
own on the same grammar crates instead (`crates/wormhole-ffi/src/syntax`), following GPUI Kit's rules.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P03 | open the diff of a changed `.rs`, `.py`, `.ts`, `.go`, `.toml`, `.c` file | keywords, strings, comments, types, functions in the theme's `syntax.*` colours, the same on both sides of a split; a block comment or string that starts above the hunk is coloured (the whole file is parsed, not the hunk) | `ffi/src/syntax/engine.rs paint`, `ffi/src/git/colors.rs`, `records.rs GitDiffLine.spans`, `GitBridge/GitDiffFile.swift`, `PanelModel/Syntax/CoreSpans.swift` |
| P03 | a removed line, an added line, a context line; a deleted file; a new file | removed from the old file's parse, the others from the new one (the old one for a deleted file) | `ffi/src/git/colors.rs line` |
| P03 | a diff of `notes.txt`, `Makefile`, `a.h`, `a.kt` | plain, as in the GPUI app (no grammar: nothing parsed); the Swift lexer no longer colours real diffs | `ffi/src/syntax/language.rs of`, `PanelModel/Diff/DiffHighlighting.swift` |
| P03 | a diff over 20,000 lines; a side over 4 MiB | "No highlighting" and plain lines; the big side plain, the other coloured | `ffi/src/git/service.rs file_diff` (core's `DiffLimits`), `records.rs highlight_skipped` |
| P03 | edit the file while its diff is open, before it is read again | a line whose text the file no longer has at its number is plain, never coloured by another line's colours | `engine.rs line_of` |
| P03, P07 | open a `.rs`, `.json`, `.md`, `.swift`, `.html` file in the Files viewer; then a 50,000-line one | text at once, colours a moment later, scrolling never parses; no beach ball | `FilesBridge/Support/CoreSyntaxHighlighter.swift`, `PanelUI/Stores/FilesStore+Viewer.swift` (unchanged), `Wormhole/PanelBackendsFactory.swift` |
| P03 | TEST (hostile input): lines with a combining mark after a control, a bidi override, an emoji with joiners, `\r` inside, a 2-million-character line, a 5 MiB file, nesting 10,000 deep, a file that is not what its extension says | no crash, no offset outside its line, colours only in the first 5,000 characters, over the cap plain; a parse or query over 2 s gives plain | `engine.rs spans`, `paint`, `MAX_TEXT_BYTES`, `BUDGET`, `MATCH_LIMIT`; `CoreSpans.swift inCharacters` |
| P03 | TEST (wire): `GitDiffLine` gained `spans`; `SyntaxRun.key` indexes `syntax_keys()` | Swift and Rust agree on the key table's order; a key out of range is dropped | `ffi/src/syntax/mod.rs`, `GitDiffFile.swift keyNames` |
| P03 | a theme that styles `tag` but not `tag.doctype` | the doctype takes `tag`'s colour (first segment), not `variable`'s | `DesignSystem/Theme/ThemeTokens.swift syntaxStyle` |
| P03 | compare the same Rust, TypeScript, Go, HTML, JSON and Markdown file in both apps | colours can differ in details: the GPUI app uses GPUI Kit's own queries for these seven and colours a language inside another (a fenced block, `<script>`, `<style>`); the bridge uses the grammar crates' queries and leaves an embedded language plain. Swift files are coloured here and not in the GPUI app (GPUI Kit has no query for Swift) | `ffi/src/syntax/language.rs` |
| P03 | first diff of a TypeScript or TSX file | coloured: JavaScript's query and TypeScript's compile as one against the TypeScript grammar; if they do not, the log says "syntax query for typescript" and only TypeScript's own captures colour | `engine.rs compile` |

- Not written, needs the owner's decision: making the two apps colour identically. Either the bridge's engine takes GPUI Kit's query
  files (Apache-2.0 text copied into this repository, with its notice) and its injections (about 900 lines there), or the GPUI app moves
  onto the bridge's engine (it then becomes a leaf crate beside `wormhole-text`; changes what the shipping app colours), or the bridge
  links GPUI Kit (GPUI inside the Swift app). Until then P03 stays 0.5 in `built.csv`.
- Not written: a cache of parsed texts (the GPUI app keeps 32). A diff read again parses both sides again, off the main actor
  (about 50 ms per 10,000 lines); the compiled query is kept per language.
- `crates/wormhole-ffi/Cargo.lock` was resolved with `cargo metadata` (twice: the second after pinning `tree-sitter-md` and
  `tree-sitter-swift` to the app's versions). It has `tree-sitter-language` 0.1.9 where the app's lock has 0.1.8.

## Lane panel-rest, wave 5

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P06 | Open a note whose text under the title is one long paragraph that wraps to more than 12 lines in the panel; one of 12 short lines that do not wrap; drag the panel wider and narrower | cut at 12 lines as wrapped (the core's `note_cap`), the last two at 0.65 and 0.3, "Show all · N lines" with N the wrapped count; the short one is whole; the count follows the width. A cut text ends in the system's ellipsis (the line limit), which GPUI does not draw | `PanelUI/Notes/NotePreview.swift`, `ffi/src/notes_text.rs note_cap` |
| P06 | A note whose first line is spaces only, then a title, blank lines, a body | the body under the title is the core's `note_body` (the blank lines around it gone), a note of one line shows it once | `NoteRules.body`, `NotesModel.swift Note.body`, `notes_text.rs note_body` |
| P06 | Click a note's title; click in its text; ⇧⌘E | the title click opens it with the cursor at the end of its text; the others leave the cursor where it was. No core rule exists for this (GPUI computes the editor position in `note_ops.rs show_note`); it is the text's end index | `NoteRowView.swift open(toEnd:)` |
| P06 | In a step's or note's editor press ⌥⌘↑ and ⌥⌘↓; rebind Move Item Up in the keymap and press the new key | the item moves by the keymap's list action (`item_key`), also on the changed key; typing without ⌘, ⌥ or ⌃ never asks the core | `PanelUI/Notes/ReorderDrop.swift reorderKeys`, `NoteRules.moveBy`, `notes_text.rs note_item_key` |
| P07 | TEST (hostile input). In a scratch folder make a file and a folder whose names are not UTF-8 (`touch $'a\xffb.txt'`, `mkdir $'d\xfe'` with a file in it) beside `a.txt`; list, open the folder, open the file in the viewer, Reveal, Open in, rename it, Copy Path, pin the folder | both list with U+FFFD in the label; the folder opens, the file shows; Finder shows that very file (never `a` or another name cut short); the rename field holds the drawn name and renames that file; Copy Path is refused with the chip (as GPUI: not text); the pin is refused by the core as in GPUI (pins are text). A key that does not read back (a NUL not followed by the two hex digits of a byte over 0x7f) answers gone and makes no URL | `ffi/src/files_panel/names.rs`, `files.rs plain`, `files_records.rs`, `open_file.rs open_file_run`, `PanelModel/Files/DiskName.swift`, `WindowUI/SystemPanelHost.swift reveal`, `RenameField.swift` |
| P07 | Rest the pointer on the file's path in the viewer's header (over the terminal: in the title row); click a button of the card; right-click the path | after 400 ms the path card opens under it with the whole path, Open in Finder, Copy path (says Copied) and Insert into terminal (the path in single quotes at the prompt); it stays while the pointer is on it and closes 200 ms after leaving; the title row's folder card is unchanged | `DesignSystem/PathCard.swift`, `PathCardHover.swift` (moved from PanesUI), `PanelUI/Files/FileViewerView.swift FileViewerHeader`, `PanesUI/Title/TitleClicks.swift button`, `SystemPanelHost.insertPath` |
| P07 | Files shown on a large tree: touch one file in an open folder, edit the open file outside, save it unchanged with `touch`; `fs_usage` on the app | only the folder whose stamp changed is listed again and only a file that looks different is read again; several wakes during a pass make one more pass, not several at once | `FilesStore.reload` (`reloading`, `stamps`), `FilesStore+Viewer.swift readViewed(onlyIfChanged:)`, `files.rs stamp` |
| P07, P02, P03 | Open folders, a compare branch and a path filter for a terminal, select another, close the first | what the Files, Changes and History tabs kept for the closed terminal is dropped (also when it moves to another window) | `PanelController.keep`, `FilesStore/ChangesStore/HistoryStore.keep(terminals:)`, `WindowUI/Hub/WindowHub.swift followGauges` |
| P08 | Close the last terminal of a folder, open one there again after the project appeared or went | the folder's project root and repository root are asked again, not read from what was kept for the closed terminal | `PanelController.keep` (`roots`), `Wormhole/Panel/NotesCoordinator.swift forgetClosed` |
| P08 | TEST (wrong profile's data). Window of profile Work while Wormhole is in Default, a folder with no project: pick a colour in the Project tab, then None | written to and read from Work's `config.json` `folder_colors`, never Default's; after Move Window to another profile the colour is that profile's | `ffi/src/files_panel/project/paint.rs own_profile`, `settings_edit.rs` |
| P08 | TEST (trust). Trusted project, Actions on, actions with `"key"` `cmd-k`, `cmd-shift-v`, `cmd-ctrl-t` and `f5`: press each with the terminal focused; then revoke trust (and separately switch Actions off, and edit `actions.json` so it reads changed) and press again at once | each runs its action and the terminal hears nothing of the key (no clear, no paste, no F5 escape sequence); the menu does not run it a second time. After revoking, the core refuses the run (`run_action` reads the project and trust again); once the tab has read the project again the key is nobody's and reaches the terminal. Between the two the key is swallowed and nothing runs, where GPUI lets it through | `Wormhole/Panel/ProjectActionKeys.swift`, `TerminalMetal/Pointer/TerminalShortcuts.swift windowClaims`, `Keyboard/KeyDown.swift`, `Wormhole/TerminalFactory.swift claimKey`, `AppController+Windows.swift connect` |

Not written, and why:
- P07: a terminal whose own folder path is not UTF-8 (the root comes from the workspace as text; the Files bridge reads a key back but nothing hands it one). Tree-sitter colours are the syntax lane's (P03).
- P08: the action key is claimed from the focused terminal and, elsewhere in the window, by the menu's key equivalent (wave 4); a text field that takes the same key first keeps it, where GPUI's window claims it wherever the focus is.
- The wave-4 section of the lane panel-files above is empty in this base; its text is in commit `1594200a0`.

## Lane devtools, wave 5

Written without compiling: expect compile errors.

Row X12 (dev tooling). Every command is the control folder's (`WORMHOLE_DEVCTL`), listed in `DRIVING.md`; none is reachable from the control socket.
Least sure to compile: `DesignSystem/Inspect/Inspection.swift` with `TokenMark.swift` (an `@Observable` singleton read in a `ViewModifier`, an `NSViewRepresentable` in a conditional background); `Wormhole/Inspect/Inspector.swift` (the event monitor's isolation, `NSColor(Color)`); `Driving/PerfProfile+Reading.swift` (the generated names `perfCountersStart`/`perfCountersStop`, `PerfLockUse`'s fields) and `AppController+ControlDevTools.swift` (`desk.grantScripts(always:)`, `AccessApprovalRecord`'s fields).

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X12 | Debug build: Developer ▸ Inspector (⇧⌥⌘I); move the pointer over the window | the Developer menu is there (it was never shown: `MenuState.debug` was never set); a panel opens beside the window; the marked element under the pointer gets an outline in the primary; the panel says its id, `Module/File.swift:line`, that line of code, its size, its style in the scales' names and its tokens | `Wormhole/Inspect/Inspector.swift`, `InspectorView.swift`, `InspectNote.swift`, `DesignSystem/Inspect/Inspection.swift`, `DesignSystem/TokenMark.swift`, `WindowUI/Menu/MenuStateMaker.swift`, `Actions/ActionRouter+App.swift` |
| X12 | In the panel: click an element; turn the wheel; Pick; Copy | the click pins and does not reach the window; a wheel gesture walks out one parent and does not scroll the window; Pick picks again; Copy puts the note on the clipboard in GPUI's format (`file:line  id`, code, `W×H · style`, `tokens: …`) | `Inspector.took`, `InspectNote.text` |
| X12 | Inspect a shared component (an icon button, a key cap, a count badge, a banner, a segmented control, a status dot) | the location is the view that asked for it, not `DesignSystem/…` (the components pass their caller's `#fileID`/`#line`), and `style` is filled | `DesignSystem/IconButton.swift`, `KeyCap.swift`, `CountBadge.swift`, `StatusBadge.swift`, `Banner.swift`, `Segmented.swift`, `StatusDot.swift` |
| X12 | Release build: look for the Developer menu; `strings` the binary for the build machine's home folder | no menu, ⇧⌥⌘I does nothing; sites carry `#fileID` only, and the one `#filePath` is inside `#if DEBUG` | `InspectNote.read`, `ActionRouter+App.swift` |
| X12 | `inspect on`, `inspect 150 66 3`, `inspect parent`, `inspect copy`, `pasteboard`, `inspect off`; then `inspect pick`, `hover 150 66`, `click 150 66` | each prints the note (and parents with frames from the content's top left); the copy is in the instance's own clipboard file; while picking, `hover` and `click` go to the inspector, afterwards to the window again; with inspect off no probe view is left in the window (`bounds`) | `Driving/AppController+ControlInspect.swift` |
| X12 | Check that the frames are the window's: `inspect` a row in the main window, in Settings (`pointer-window Settings`) and in the Studio | the outline sits exactly on the element in each (the probes are asked for their window, so several windows do not mix) | `Inspection.elements(at:in:)`, `InspectHighlight.show` |
| X12 | Persistent terminals on, two hosted terminals: `detach`, `window-state`, `hosts-list`, type in the pane, `reattach`, `window-state` | `detached 2`; both read `detached(hostID: …)` and keep their last screen, the hosts still run and nothing typed arrives; `reattach` prints both ids, they read `persistent` again, the screen repaints and typing arrives. TEST (data loss): a program running in the host survives the round trip with its output intact | `WorkspaceBridge/RustWorkspaceBackend+Detach.swift`, `Driving/AppController+ControlDevTools.swift` |
| X12 | `reattach` after `host-kill <id>` of a detached terminal | a new terminal in its place, as a launch does for a host that is gone | `HostConnect.run` through `connect` |
| X12 | Automation on, access Ask; run `wh list` from outside; `agents`; `agent-ask session` | `agents` shows `asking <id> … shown deny,session,quit[,always]`; the answer lets the request through and `agents` lists `approved memory:…`; `agent-ask` with nothing up says `error: no question is up`. TEST (permissions): `agent-ask always` on a question about one terminal comes to Quit, never a stored approval | `Aux/AccessDriver+Scripted.swift`, `Aux/AccessDriver.swift` |
| X12 | `agent-script quit`, then a new program asks; `agent-script deny`; `agent-script hold` | answered at once with no held question (`question` says none is up); deny refuses with the declined reply; hold leaves the next one for `agent-ask` | `AccessDriver.present`, `AccessDriver.script` |
| X12 | `agent-grant scripts`, then a shell script calls the control API; `agent-grant scripts always`, restart, call again | let in without a question (`approved memory:… Scripts`); after `always` the approval is in Settings ▸ Agent access and holds after a restart. TEST (permissions): nothing over the control socket can reach `grant_scripts` | FFI `AccessDesk::grant_scripts` (`crates/wormhole-ffi/src/aux_backends/access/scripted.rs`) |
| X12 | The mode question (widen `agent_access` in `config.json` by hand), `agent-ask deny` / `agent-ask quit` | deny puts the earlier mode back in the file, anything else keeps it | `AccessDriver.answerPending` |
| X12 | `menubar-icons`, open the TIFFs | nine files `status-item-<idle, working, working-0…3, needs-you, failed, finished>.tiff` in the control folder, the marks in the theme's colours; a name with `/` falls back to `status-item` | `AppController+ControlDevTools.saveStatusIcons` |
| X12 | `studio`, then each `studio-*` line of DRIVING.md | the reply's state line follows (selected, shown, dirty, counts, area, undo/redo, notice); `studio-set` of a bad expression says `ok=false field=<reason>`; `studio-save <name>` of an untitled copy writes `<name>.json`; `studio-save` over an existing name holds a question for `question`; `studio-menu json` writes `studio-out/pasteboard.txt` and leaves the clipboard alone; `studio-shot s` writes `<ctl>/s.png` | `Driving/AppController+ControlStudioFamily.swift` |
| X12 | `studio-versions`, `studio-versions select 1`, `studio-versions close`, `studio-versions restore` | the rows; every window previews the older save, then the file as it is; restore makes it unsaved edits (one undo step) | `studioVersions` |
| X12 | `perf-profile 2 x` under a load (`yes` in a local terminal), in a bridge built with and without `--features perf-counters` | with: `reader: bytes=… mb_per_s=… parse_mb_per_s=… parse_busy_pct=… slices=…`, the two lock lines, and `reader`/`ui_lock` in `x.json` under GPUI's names; without: `reader: not counted …`; both: `frames: per_second=`, `fetch in Rust, per frame: snapshot= pack=`, `terminals: hosted= local=` | `Driving/PerfProfile+Reading.swift`, `PerfProfile.swift`, FFI `perf_counters_start`/`perf_counters_stop` (`crates/wormhole-ffi/src/perf_counters.rs`), feature `perf-counters` in `crates/wormhole-ffi/Cargo.toml` |
| X12 | Debug build: pin a key cap (`inspect X Y`), `inspect nudge padding x +`, `window-shot`, `inspect copy`, `inspect reset` (and the panel's − / + / Reset) | every key cap's side padding grows by 4 pt at once (the step moves the scale's value, so every element that uses `_1` follows: the panel says so); the note ends `change: padding x _1 → _2`; a radius steps 2 pt (`radius chip → 8px`), nothing goes below 0; Reset and a restart put everything back (no file, no defaults) | `DesignSystem/Inspect/DesignOverrides.swift` (whole file `#if DEBUG`), `ScaleValue.of` in `DesignSystem/Scale/Space.swift`, `Scale/Metric.swift`, `Scale/Radius.swift`, `Wormhole/Inspect/InspectNudge.swift`, `InspectorView.swift`, `Driving/AppController+ControlInspect.swift` |
| X12 | Release build: `inspect nudge padding x +`; look for `DesignOverrides` in the binary (`nm`) | `error: the pinned element has no … to step`; no such symbol: `ScaleValue.of` returns its standard, `Space`/`Metric`/`Radius.row` are computed constants now instead of `static let` (same values) | `ScaleValue.of` |
| X12 | NOT WRITTEN (nudging): what an override cannot reach. Values copied out of the scales once: the `static let`s made from them (`StudioMetrics.galleryGap`, `PaletteUI/SheetLayout` `sectionGap`/`bodyTop`, `QuickPanelMetrics` `searchHeight`/`rowHeight`/`labelHeight`/`answerIndent`, `LookStep.sketchWidth`) and anything read outside a SwiftUI body (AppKit layout, a window's size) keep the value of their first read. Text roles are not stepped (`TextRole` is not in the table; the owner named Space, Metric and Radius). Numbers a view writes itself (`min height 40`, `radius full`) have no stepper. A step is per scale value, not per element as GPUI's is: the note names the pinned element. The outline keeps the frame of the pick after a step | - | `InspectNudge.swift` head comment |
| X12 | NOT WRITTEN: source locations and style for elements that are not marked (`.marked` is on the design's components, about 60 sites; GPUI records every `div`) | - | a view says so with `.marked("id", style: …)`; SwiftUI has no way to ask an arbitrary view where it was built |
| X12 | NOT WRITTEN: `WORMHOLE_AGENT_ASK` at launch (GPUI devctl builds read it) | - | the Swift control line exists in release builds too, and an environment variable that answers approvals belongs to no shipping binary: `agent-script` as the scenario's first line does it. The owner's call, 2026-11-13: left out; maybe later, once the control line itself is compiled out of release builds, and then inside that same gate. It does not hold the row back: `built.csv` has X12 at 1 |
| X12 | NOT WRITTEN: `agent-hide <n>` (not in this lane's list), and `studio-frames … tick` (SwiftUI's animations are not stepped from outside) | - | `hideFromAgents` is the menu's action; `studio-frames` times plain draws and `scroll` |
| X12 | Read `macos/scripts/build-ffi.sh` with and without `WORMHOLE_FFI_PERF_COUNTERS=1` (`bash -x`, in the verification wave) | with it cargo gets `--features perf-counters`, without it no feature argument at all (the empty array expands to nothing under `set -u`) | `macos/scripts/build-ffi.sh` |
| X12 | NOT WRITTEN: `shape` and `images` as spans of their own (the Swift renderer has no such parts: it has `fetch/apply/assemble/encode/wait/cpu/gpu` and counts words shaped) | - | cannot exist |

## Lane terminal, wave 6
Written without compiling: expect compile errors.
| row | what to do | what must happen | where the code is |
|---|---|---|---|
| T16 | open a terminal with "New terminal: fish" (or a project shell), then `exit` | the exited bar says "fish exited", not "zsh exited" | ffi `workspace_waited.rs` `shell_name`, `TerminalOverlay.exited(shell:)`, `TerminalPanes.swift` |
| T16 | chip off; program copies (OSC 52) while its clipboard question or a host bar is up | the notice shows after the question/bar goes, until its 4 s are up | `TerminalOverlay.sayInBar`, `askEnded` |
| T16 | chip off; a copy by a program in pane B while pane A is selected | the notice is in B's bar, not A's | `TerminalOverlay` `say` closure, `WindowHub+BarNotice.swift` |
| T25 | close a fold in view, wheel and drag past the edge | the view moves one shown row per step, a closed fold being one | ffi `pointer.rs` `scroll_wheel`, `terminal.rs` `scroll_rows`, `DragSelection.swift` |
| T25 | scroll up with a fold above the view's top, ⌘↑ | goes to the prompt above the top row as drawn | ffi `blocks.rs` `top_line`, `BlockController.jumpPrompt` |
| T29 | program sends `CSI 8;30;100 t` then `CSI 8;40;120 t` (or a maximise) at once | the second is worked out from 30x100, not the old size; Forget on maximise/full screen | `Wormhole/Platform/WindowOps.swift` `asked` |
| T01 | `vim` with a coloured background, or any program painting the edge cells | the colour runs into the 4 pt padding on all four sides; the block cursor at an edge does not | `MetalRenderer.encode`, `Shaders.swift` `solid_vertex`, `RowBuilder.flushBackground` |
| T07 | ⌘C ⌘V ⌘A ⌘K ⌘⌫ ⌘← ⌘→ in a terminal; rebind one in Settings | each does what it did; a rebound key follows the table | `TerminalShortcuts.swift`, `TerminalLook.terminalAction` |
| TEST T20 | restore after restart on, relaunch so a terminal restarts from a checkpoint, quit within 10 s of the start | the old checkpoint is removed only when the new host's reads, and not after 10 s of silence | ffi `hosting/checkpoints.rs` `start` |
| TEST T20 | quit while a host is still starting | the new host is let go of (detached), not ended with the app | `HostConnect.swift` `keptPastQuit` |
| T19 | none written: constants, bell, answered, settle, title gate and marks read equal to GPUI's | | core `workspace/status/` |

## Lane devctl, wave 6

Written without compiling: expect compile errors.

The GPUI commands the Swift control line lacked (audit X12), by family, with what this lane did. Files: `Wormhole/AppController+ControlRead.swift`, `+ControlFiles.swift`, `+ControlFlow.swift`, `DevCtl.swift`; hooks `WindowUI/Hub/WindowHub+Reading.swift`, `TerminalUI/Blocks/BlockController+Listing.swift`, `MetalRenderer.imagePlacements`, bridge `hook_server/control_line.rs`.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X12 | Write `key f6` to `cmd2` with two windows (`new-window` first), then `keys` and `state` in `cmd2` | the key and the answers go to window 2 (its focus changes; `out2.txt` has the answers, `out.txt` has none of them); a `cmd` line afterwards acts on the front window again | `DevCtl.drain` (reads `cmd<n>`), `AppController.commandWindow`/`front`, `run(line:window:)`, `reply` |
| X12 | `cmd2` while only one window exists | the line acts on the window commands act on (no crash, no `out2.txt` mix-up beyond the file name) | `AppController.front` (index checked) |
| X12 | In a marking shell (`fixture-e.sh`): `type` two commands, `blocks`, `block 1 select`, `block 1 copy-output`, `pasteboard`, `block 9 select` | `blocks` lists `n: lines a..=b output c..=d \| exit 0 · 0.1s \| cmd`; select highlights the output; copy-output puts it on the (diverted) pasteboard; block 9 says `error: no block 9` | `BlockController.all()` (walks `promptLine` from the bottom, so a block with no prompt mark above it is not listed: compare with the number of prompts), `WindowHub.blockAction(number:named:)` |
| X12 | Print an image (kitty/Sixel) and run `images`; scroll it out of view and run again | total, then `key: WxH n bytes` and `placement line … column … cells CxR z …`; GPUI counts the engine's store, this counts what the last frame placed, so an image out of view is not listed | `AppController+ControlRead.imagesListing`, `MetalRenderer.imagePlacements` |
| X12 | `pane-bar` with a program asking to read the clipboard (OSC 52 read, `clipboard-front on`), after `clipboard-answer once`, and in a terminal whose host cannot be shown | `question <program> wants to read the clipboard`, then `none`; `host-unavailable …` | `WindowHub.paneBarSummary` |
| X12 | `panes` in a single terminal, after `split`, after `zoom` | `split=false` and one pane; `split=true` with two `pane id x y w h` lines (one `focused`) and a divider; `zoomed` marks one | `WindowHub.panesListing` |
| X12 | `layout columns`, `rows`, `grid`, `single`, `layout x` | each applies View ▸ Layout (`panes` shows it); `x` says `error: expected columns|rows|grid|single` | `ControlRead` `layout`, `.applyLayout` |
| X12 | `shells`, `new-shell -`, `new-shell <a key from shells>`, `new /tmp`, `new ~/` | `shells` lists `key<TAB>name<TAB>command`; each opens a terminal (`state` terminals +1; `new-shell` takes no folder, GPUI's `[dir]` is not supported) | `ControlRead` `shells`, `new-shell`, `new` |
| X12 | `close 2` with three rows, `close 9`; `agent-hide 1`, `agent-hide 1 off` | row 2 closes at once with no question (`close` without a number is unchanged); 9 says `error: no row 9`; hide toggles, `state`/menu show it | `ControlRead` `close`, `agent-hide`, `finishClosing` |
| X12 | `zen`, `zen on`, `zen off`, `zen toggle`, `zen x` | `zen off`, `zen on`, `zen off`, `zen on`; `x` is an error; the panel and status bar hide while on | `ControlRead` `zen`, `ZenMode.swift` |
| X12 | Files tab: `files`, `files-root /tmp`, `files`, `files-follow`, `files-pin <a folder below the root>` twice, `files-rename a.txt b.txt` on a scratch folder | `files` prints `files: <root> (following\|chosen) pinned=…`, rows indented; root changes then follows; the pin toggles (`files` shows it); the file is renamed on disk. TEST: the rename writes to disk, run it on a scratch folder only | `AppController+ControlFiles.swift`, `FilesStore.choose/followTerminal/setPinned/commitRename` |
| X12 | `delay 300` between two `state` lines; `sync a` | the second answer comes after 300 ms; `sync a` answers `sync a` | `AppController+ControlFlow.swift` |
| X12 | TEST: `control list --json`, `control --from <id> list --json`, `control status 1`, `control read 1`, `control bogus` | the reply's JSON line (the second as from that tab, so the access gate may ask or refuse as for an agent: answer with `agent-ask`); `read` says `not carried here`; `bogus` says `error: …`. Goes through the same gate and handlers as the socket, with `Peer::default()` | `hook_server/control_line.rs`, `HookServerHandle.controlLine`, `ControlFlow` |

Not written, by family (each is either not a thing in the Swift app or needs a view hook that is a lane of its own):

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X12 | NOT WRITTEN, no Swift counterpart: `paints`, `refresh` (GPUI's paint and cache counters), `scaling settle`/`reveal-test` and `grow-hold` (GPUI's resize animation and first-frame tricks), `scroller` (the system draws Swift's scroll bars), `a11y live` | - | the mechanism is GPUI's |
| X12 | NOT WRITTEN: `drag <x> <y> <path>`, `drag-end` (a file hovered over the window with its drop tint), `front`, `close-window`, `pane-bar` for a bar's buttons | - | needs an `NSDraggingInfo` stand-in; `front` is forbidden by the background rules (`InFront`) |
| X12 | NOT WRITTEN: `menu-native`, `popover`, `popover-press|close|type|key|colour|native|real`, `path-menu`, `files-menu`, `available <action>` | - | popovers and menus here are native objects kept unshown by the injector (`menu`, `menu-pick`); a popover recorder would be a hook of its own in `PlatformUI` |
| X12 | NOT WRITTEN: the whole `folder-picker …` family (state, type, tab, right, left, up, down, enter, pick, forget, click, crumb, scroll) | - | `folders` opens it; its model is in the folder-picker lane's files |
| X12 | NOT WRITTEN: sidebar `group-ask`, `rename-edit`, `folder-color`, `stash`, `send`, `send-submit`; profiles `profiles`, `profile-open`, `profile-move`, `profile-delete`; panel `project-init`, `comment`, `compare`, `picker compare`, `commit-details`, `commit-more`, `note-seed`, `note-toggle`, `task-seed`, `task-done`; settings `import-scan`, `font-picker`, `settings-record|reveal|preview|reset-confirm|cli-tool`, `update-installed`; windows `hosts-toggle`, `hosts-scroll`, `licenses-search`, `licenses-select`, `intro-frames`, `onboarding-tailor`, `welcome-probe`; notices `notice sticky`, `notice-click`, `notice-material`, `notify-sim`, `notify-permission` | - | out of this lane's time; each is a model call over the store its page or window already has, in the order the audit lists them |
| X12 | NOT CHANGED: `pointer` keeps its `ROW COL` meaning (GPUI's is a point in points); `hosts` keeps opening the window (`hosts-list` is GPUI's `hosts`) | - | a rename would break scripts written for the Swift control line |

## Lane platform, wave 6

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| PL01 | Press ⌃⌥2, ⌃⌥⌘1, ⇧⌘N, ⇧⌘], ⇧⌘[, ⌘= in a real window | the action of the table's row runs (Group 2, the intro trial, New Terminal, next terminal, previous terminal, bigger); a menu item's own first key still goes through AppKit and is not run twice | `Wormhole/Actions/ExtraKeys.swift`, started in `AppController.applicationDidFinishLaunching` |
| PL01 | Switch History off (Settings ▸ Features), open the menus, the palette (Actions) and the shortcut sheet | Show History, Show Notes/New Note, Show Project, Show Files (whichever is off) are gone from all three; the window of another profile follows its own profile's switches | `SettingsBridge/ProfileFlags.swift` `featuresOff`, FFI `SettingsHandle::features_off`, `Wormhole/Menus/RecentProjects.swift` `menuState` |
| PL01 | No agent on screen; terminal with no folder | Show Usage is not in the menu; Show Project is greyed | `WindowUI/Hub/WindowHub+Menu.swift` |
| PL01 | Two windows, one terminal waiting in the other: ⌥⌘J, again; ⇧⌥⌘J | the one that waited longest, its window comes forward and the app activates; again goes to the next; finished likewise | `Wormhole/Ambient/AppController+Ambient.swift` `jump(toWaiting:)` |
| PL04 | From another app choose New Terminal, a terminal's row, Jump to Next Waiting, Open Folder in the status item or the Dock menu | Wormhole comes forward first; the menu shows the global hotkey beside Show Wormhole and ⌥⌘J beside Jump; Projects lists every open project's actions | `AppController+Ambient.swift` `bringForward`, `AppController+Windows.swift` `show(terminal:)`, `AmbientController.swift` `pushStatus` |
| PL03 | Signed build with Time Sensitive allowed: a terminal needs input while the app is in the background | the banner goes out Time Sensitive; Wormhole in front on that terminal retracts it, in the background it stays until the app comes forward | `Wormhole/Notices/NoticeFeed.swift` |
| PL03 | Settings ▸ Notifications ▸ Send test: macOS not asked, then allowed; then off in System Settings | a line under the row says what happened (waiting, sent, off, never answered); nothing ends silent | `NoticeFeed.sendTest(…report:)`, `Settings/SettingsPlatform.swift` |
| A05 | `wh send-text` to a terminal the program did not open | the question names the terminal's row title and folder; the program's facts end with "Runs in <terminal>" when it runs in one of ours | `Aux/AuxBackends.swift` `accessResolver`, FFI `Workspace::terminal_running` (`workspace_access.rs`) |
| A05 | TEST (permissions): questions waiting, set agent access to Off in Settings; again with questions waiting set it to Allowed while open | Off: every waiting request is refused `agent_access_off`, the sheet goes, streams under approvals end. Open: every waiting request is let in at once. Ask: nothing changes | FFI `hook_server/core.rs` `settings_changed`, `AccessDesk::settle_questions`, `approval_ids` |
| A05 | TEST (permissions): a Scripts question | the buttons are Don't Allow, Until Quit (no "For this session") | `AuxModel/Access/AccessDisplay.swift` `buttons` |
| A07 | Widen the mode by editing `config.json` outside the app | the keep-it question appears when the file changes (the settings applier calls `settingsChanged`, which now looks at the mode at once) | `ffi/hook_server/core.rs` `settings_changed` |
| A01 | First run: pick a folder with a `.wormhole/` that is not trusted, go to the agents step | the trust card shows for the picked folder (else the front terminal's project); Trust with Touch ID goes through the authenticator seam, the card follows a changed pick. TEST (trust) | `AuxBridge/OnboardingBridge.swift` `trustCard(for:)`, `AuxModel/Onboarding/OnboardingSession.swift` |
| A01 | First run, folder step | recently opened projects are under "Recents" | `Aux/AuxBackends.swift` `onboarding` |
| A01 | First run with no agent on PATH: Start on the primer | the sidebar shows the "No coding agents yet" note unless it was closed for good; the primer lists "Jot a note" only with Notes on | `AuxBridge/OnboardingBridge.swift` `onPrimer`, `AuxWindows/Onboarding/PrimerStep.swift` |
| A01 | First run, pick a folder and finish with the window's one untouched terminal | one new terminal in the folder replaces it; with two terminals, or the folder already there, nothing opens; a dropped file picks its folder | `Aux/AuxBackends.swift` `onboarding`, `AuxWindows/Onboarding/FolderStep.swift` |
| A01 | Help ▸ Welcome with the intro page up | the page plays again, no second window | `Aux/AuxController.swift` `showOnboarding` |
| A03 | Check for Updates… in the menu, status item or Settings | the Updates window opens and a check starts (unless one runs or an update is ready) | `Aux/AuxController.swift` `showUpdates` |
| A02 | About | the copyright line shows | `Wormhole/Info.plist` |
| A04 | Session Hosts with a hosted terminal in a folded group | it shows as attached, not detached | `Aux/AuxBackends.swift` `hostAttachments` |
| A06 | Project commands for no open project | says "This project is no longer open in a terminal." | `AuxModel/Trust/TrustModel.swift` `ClosedProjectTrust` |
| O01 | History off, open the palette in a repository | no commits listed | `Platform/PaletteSourcing.swift` |
| O03 | A terminal changes folder in a second window or a folded group | the folder counts as a visit | `Platform/FolderPicking.swift` |
| A04 | NOT WRITTEN: an unavailable host's card carries the error's detail | - | `HostProblem.unavailable` carries only the kind: the error text of `HostingError.Unavailable` is dropped in `HostConnect.swift` and would have to travel through `Connected`, the book and `HostProblem` |
| PL01 | NOT WRITTEN: Undo/Redo titles of a window with its own history (Studio), Open Recent per profile (also O01, O03), an event announced by its terminal's profile's settings (PL03), the palette's commits as HEAD's log rather than History's | - | each is its own data or model change (a per-profile store, a Studio menu writer, `NoticeSettings` per profile, a git log call) |

## Lane crosscut, wave 6

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| logging | Start a test instance (`WORMHOLE_DATA_DIR=<scratch>/Wormhole-swift-x`), run a minute, `WORMHOLE_LOG=debug` once | `<data>/logs/wormhole.<date>.log` exists (owner-only folder), holds the core's `tracing` lines; level follows `WORMHOLE_LOG`; Help > Show Logs opens a folder with a file in it; never the real data folder | `crates/wormhole-ffi/src/logging.rs`, `Wormhole/Platform/LaunchLogging.swift`, `AppController.swift` (init) |
| logging | Force a panic on a Rust thread (a test build) | `<data>/logs/last-panic.txt` holds the report, written before the abort | `logging.rs` `install_panic_hook` |
| logging | New dependencies `tracing-subscriber`, `tracing-appender` in `crates/wormhole-ffi/Cargo.toml` | The ffi workspace's own `Cargo.lock` needs updating by the first build (offline it fails): check the lock diff | `crates/wormhole-ffi/Cargo.toml` |
| TEST data folder | Put a file with mode 0644 and a folder 0755 in a scratch `Wormhole-swift-x` folder, launch | Both become owner-only a moment after launch (no hang on a big folder: it runs on its own thread) | `logging.rs` `make_data_dir_private`, `LaunchLogging.start` |
| X08 | `Wormhole split --size 30` on a terminal with room, then with `--size 99` in a small window | The new pane has about 30 % and the reply's `size` says what it got (clamped); a split without room is refused `no_room` and the budget is not spent | `hook_server/control/open.rs` `split`, core `workspace/splits.rs` `set_split_share`, `split_fits` |
| X08 | Two windows, the second one in front; `Wormhole new-terminal` from a terminal of the first; `list` | The terminal opens in the front window; `list` marks the front window `front`; `new-window` copies the front window | `hook_server/core.rs` `front_share`, `mod.rs` `set_front_window`, `Hooks/HookHost.swift` (didBecomeKey) |
| X08 TEST | `Wormhole new-terminal --worktree fix/x --command "echo hi"` | The worktree question comes up (no `unsupported` any more); the command travels in the event (`HookAction.worktreeAsked(command:)`) into `WorktreeStore.firstCommand`. NOT BUILT: typing it into the terminal after Create is the panel lane's side | `control/worktree.rs`, `HookBridge/HookAction.swift`, `WindowHub+Worktree.swift`, `WorktreeStore.swift` |
| X07 | `python3 -I macos/scripts/words.py` | 0 literals, 0 unknown ids, 0 variable mismatches; `*Controller` and `*Controls` files are read now | `macos/scripts/words.py` `SKIP_PATH`, `AuxText+Questions.swift`, `swift-aux.ftl`, `swift-window.ftl`, `changes.ftl` |
| X07 | Run with `WORMHOLE_LANGUAGE=en-XA`, close a window / group / terminal that runs something; toggle sidebar and panel; the open-folder panel | The questions, buttons, toggle labels and tooltips, the open panel's button are pseudo-language | `AppController+Quit.swift`, `+WorktreeClose.swift`, `WindowControls.swift`, `+Ambient.swift` |
| X07 | Settings > Notifications > the "after" stepper | Says "2 min" / "30 s" through the catalog (the count was not passed before) | `SettingsUI/Rows/CommandDoneControl.swift` |
| X03 | VoiceOver on the panel's tab bar (or `a11y`) | A tab says "Changes, 3 changed files", "Notes, 2 open next steps, 1 note", "Changes, no changes"; the value stays "2 of 6" | `WindowUI/PanelTabBar.swift`, `RightPanel.swift`, `workspace_spoken.rs` |
| X03 | VoiceOver on the status bar's needs-you button and counts | "Fix pile needs input, waiting 2 minutes", "2 terminals working", "1 terminal finished" | `WindowUI/StatusBar.swift`, `WindowBody.swift`, `workspace_spoken.rs` |
| X03 | VoiceOver on an empty Notes, Next steps, History band and an empty Changes group | "Notes, no notes", "Next steps, none open", "History, no commits", "<group>, no files" | `swift-panel.ftl`, `NotesCard.swift`, `StepsCard.swift`, `HistoryView.swift`, `ChangesListView.swift` |
| X03 | Settings > Changed > Copy JSON with VoiceOver | Says "Copied" | `SettingsUI/Pages/ChangedPage.swift` |
| X11 | `--background` instance: open the group colour panel | It goes behind the other windows, takes no focus | `Wormhole/Menus/GroupColourPanel.swift` |
| not written | Rolling counts in the status bar | Left to the lane sidebar-window | n/a |

## Lane panel, wave 6

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P02, P03, P04 | TEST: in a repository, `cd` into a subfolder (`crates/x`), change a file there; in Changes use the row's menu (Copy Path, Open, Reveal), the open icon on a row, and open the diff | Copy Path copies `<repo root>/<path>`; Open and Reveal are enabled; the diff header's open button, the per-line icons and Reveal on the too-large card show; History's file menu the same | `PanelUI/Controller/PanelController.swift` `repoRoot` (set as `panelRoot` in `PanelContentView`, `PanelCoverView`, `PanelCoverTitle`), `Stores/ChangesStore.swift` `root` |
| P02 | TEST: a file that is a link out of the repository (planted symlink) in the list | the row's open icon and the menu's Open and Reveal do nothing (no fallback to the unresolved path) | `Changes/PathMenu.swift` `resolved`, `Changes/FileRowView.swift` |
| P02 | Open a submodule row in a subfolder; press its terminal button | a terminal opens in the submodule's folder under the repository root | `Changes/SubmoduleRowView.swift` (root now the repository's) |
| P02 | Choose a compare branch, `cd` in the same terminal | the compare branch stays (an amber line if the new folder lacks it) | `Stores/ChangesStore.swift` `follow` |
| P02 | Click the title row's branch chip (panel closed, and open on another tab) | Changes shows and the "compare with" list opens | `WindowUI/Hub/WindowHub+Branch.swift`, `PanelUI/Shell/RevPicker.swift` `requested`, `ChangesStore.pickerRequested` |
| P03 | Open a diff, change Settings tab width | the sideways scroll extent follows the new width without reading the diff again | `Stores/DiffSession.swift` `retab`, `Stores/PanelPreferences.swift` |
| P03 | Comment on a line whose text is over 160 characters with a combining mark or emoji; paste the review | the quote is cut at 160 Unicode scalars and trailing white space trimmed, as core's | `PanelModel/Review/Review.swift` `quote` |
| P03 | Open a diff that is loading | it says "Loading diff…" | `PanelUI/Diff/DiffView.swift`, `swift-panel.ftl` `diff-reading` |
| P04 | History: focus the list, press ⌘C on a commit | the hash is copied and the notice chip says it | `Shell/SelectableListSplit.swift`, `Stores/HistoryStore+Walk.swift` `copiedAs` |
| P05 | TEST: Shell → New Terminal in New Worktree from a terminal inside a linked worktree of a repository whose `.wormhole/` is untrusted | refused with the trust message, asked of the main working tree's project; with project folders off nothing is asked | `GitBridge/RealWorktreeBackend.swift` `repository`; ffi `git/worktrees.rs` `repository` |
| P05 | TEST: with a sheet up, revoke trust of the project (Project tab), press Create | the sheet stays up and says the trust message under the field | `PanelUI/Stores/WorktreeStore.swift` `create`, `footnote` |
| P05 | TEST: `Wormhole new-terminal --worktree b --command "npm test"` (trusted project with `worktree.setup`) and Create | one terminal in the new folder gets `setup && npm test` pasted and run; with no setup only the command; no other terminal gets text | `ffi hook_server/control/worktree.rs`, `HookBridge/HookAction.swift`, `Wormhole/PanelBackendsFactory.swift`, `Wormhole/Panel/PanelEffects.swift` (`.new` waits for the view) |
| P05 | TEST: the same command line in an untrusted project; and a second request while the sheet is up | Refused (exit code for refused) with the trust words; Failed "a worktree dialog is already up in that window", and the sheet is not reset | `ffi hook_server/control/worktree.rs`, `HookServer.worktree_sheet`, `WorktreeStore.sheetChanged`, `Wormhole/Hooks/HookHost.swift` |
| P05 | TEST: two profiles; make a worktree in a window of the second, start a terminal in it | the worktree and ports are remembered in the second profile's database; its terminal gets `WORMHOLE_PORT`; the first profile has none of it | ffi `git/worktrees.rs` `follow_profile`, `env_for(cwd, profile)`, `launch.rs`; `GitBridge.makeBackends(profile:)`; `Settings/ProfileChipBinding.swift` |
| P05 | Close a group that is folded, whose terminal is in a worktree Wormhole made; close a terminal whose neighbour is in `…/fix-login2` | the question lists the worktree; `…/fix-login2` does not count as inside `…/fix-login` | `Wormhole/AppController+WorktreeClose.swift` |
| P01 | Press ⌃⌘4 with no agent on screen; look at View ▸ Show Usage / Show Project without a folder | nothing happens; Show Usage is hidden, Show Project greyed | `WindowUI/Hub/WindowHub.swift` `send`, `Menu/MenuStateMaker.swift` `offeredTabs` |
| P01 | Under Zen mode press ⌃⌘3; leave Zen | the tab changed, the panel did not open; the panel shows that tab after Zen | core `Action::SetPanelTab`, `WindowHub.send` |
| P01 | Restore a window whose saved tab is Usage with no agent | the tab leaves at once, panel open or not | `WindowUI/Hub/WindowHub.swift` `followTabs`, `WindowHub+Focus.swift` `leaveMissingTab` |
| P01 | Click the panel button to close the panel while a note is being edited | the terminal has the keys | `WindowUI/WindowBody.swift` |
| P01 | Hover a tab, the stash button, the Notes + | the tooltip names its key ("Changes (⌃⌘1)", "New note (⌥⌘N)") | `PanelTabBar.swift`, `PanelModel/Backends/PanelHost.swift` `keyed`, `StashPromptButton.swift`, `NotesCard.swift` |
| P01 | VoiceOver on the strip | tab reads "Changes, 3 changed files", "Notes, 2 open next steps, 1 note", value "2 of 6", in a tab bar | `WindowUI/Hub/WindowHub+TabSpeech.swift`, ffi `panel_spoken.rs`, `FilesBridge.installRules` |
| P01 | Switch Changes to Notes, then back | Notes slides in from the right (after Changes in the strip), Changes from the left | `WindowUI/RightPanel.swift` `arrival` |
| P06 | TEST: with a trusted project and notes sync on, tick a step within the 2 s sync, repeatedly (add a line in `notes.md` at the same time) | the tick is kept, the file's new line arrives, no step is lost or ticked back | `PanelModel/Notes/NotesRebase.swift`, `Wormhole/Panel/NotesCoordinator.swift` `sync` |
| P06 | Start a new step (draft row open), select another terminal | the draft row is gone | `NotesStore.leaveDraft`, `NotesCoordinator.changed` |
| P07 | Files tab: focus the list, put the cursor on the path button, press Return or Space | the path menu opens under the button; with a file viewer up, walking onto the button does not | `Files/FilesPathButton.swift` `nsMenu`, `Stores/FilesStore+Walk.swift`, `PanelWalk.swift` `pressesBand` |
| P08 | Layout with a terminal named `api` whose program title shows another name; and a terminal you renamed `api` | "Open terminals" opens nothing new for either; a terminal whose title is `api` but has no custom name does not block it | `Wormhole/Panel/PanelEffects.swift` `existing`; core `TerminalSummary.custom_name` |
| P09 | Usage tab with a limit that resets in under a minute | "resets 45s" counts down each second; "Updated" changes at each report's minute; nothing runs while the tab is hidden | `PanelUI/Stores/UsageStore.swift` `keepTime`, ffi `files_panel/usage.rs` `next_change` |
| P07 | NOT WRITTEN: a terminal whose own folder is not UTF-8 | - | the root comes from the workspace as text (a `Vec<u8>` path through the bridge is a wire change) |
| P08 | NOT WRITTEN: an action's key as the window's wherever the focus is | - | needs a window-level key handler that asks the core's trust; a text field takes keys first (the lane said so) |
| P09 | NOT WRITTEN: reports per profile | - | the one `UsageService` is the hook socket's; per-profile services need one socket service per profile |
| P03 | NOT WRITTEN: the comment form as a dialog titled "Comment on <place>" with Add Comment / Save | - | a popover on the line (a design choice, not a missing behaviour) |

## Lane sidebar-window, wave 6

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| W02 | A terminal hidden from agents (row menu): look at the title row | a lock after its name, tooltip "Hidden from agents" | `TitleFactsMaker` (`hiddenFromAgents: terminal.isAgentHidden`) |
| W02 | Terminal in a submodule folder; one in a repo on a detached HEAD (`git checkout --detach`) | "in <parent>" after the branch chip (while the row has room); an amber "detached" chip instead of no chip | core `TitleTerminal.superproject`/`detached` (`workspace/snapshot/title.rs`), `GitFacts.superproject`, `TitleMapping.swift`, `TitleFactsMaker.branch(of:extras:)` |
| W02 | Status bar off; an agent working for 4 minutes, one done, a failed command (`false`) | "working · 4m", "done · 3m", "failed · exit 1" after the name; the minute moves by itself (one timer, no poll); with the bar on the row says nothing | `StateWords.swift`, `CentreTitle` (`TerminalColumn.swift`), `selectedTerminal` merges `since` from `all` (`Chrome.swift`) |
| W03 | Same with the status bar on: the state item at its end | "failed · exit 1" for a failure with a code, "working · 4m" (the time was missing before: the row has no `since`) | `StatusBar.stateText` |
| W02 | Terminal with an agent (claude) | the title row's tile, each pane header's tile and the status bar's attention button show the agent's logo with the status light, as the row does | `WindowHub+Tiles.swift`, `TitleActions.tile`, `PaneActions.tile`, `StatusBar.tile` |
| W02 | Drag a file from Finder onto the title row | the terminal under it gets the path typed (escaped, with a space) and the keys; the row tints while the file hovers | `TitleFileDrop.swift`, `MetalTerminalView.insertDropped` |
| W02 | Exited terminal, open the path menu / hover card | "Insert into terminal" is off only for an exited one, on otherwise | `WindowHub+Title.swift` `canInsert` |
| W03 | View ▸ Show Status Bar (and the key), quit, relaunch; also open Settings meanwhile | the bar's state is the `status_bar` setting: it survives the relaunch and the Settings switch follows | `ActionRouter+View.swift`, `settingsBackend(of:)` in `ProfileLooks.swift` |
| W12 | Window of profile B in front: View ▸ Show Status Bar | B's `config.json` gets `status_bar`; B's windows follow, A's do not | `settingsBackend(of:)`, `ProfileLook.backend` |
| W05 | Window of profile B in front: ⌘N | the new window is a window of profile B (not the shown profile) | `AppController+WindowOpening.swift` `openNewWindow` |
| W09 | Select another terminal / rename one / a program sets a title; open the Window menu, ⌘`, Mission Control | the window is named after the selected terminal and follows it | `AppController+WindowFacts.swift` |
| W09 | Motion feature on: open and close a window; then Reduce Motion on; then a torn-off window; then a test instance | the system's document animation; a cut; the window's own growth (default behaviour); default | `AppController+WindowFacts.swift`, FFI `window_animation`, `WindowsBackend.windowAnimation` |
| S08 | Release build (Motion feature off) with `motion.status`, `motion.insert`, `motion.fold` set in config.json | rows, status and fold are cuts (Classic), as GPUI; with the feature on and the keys absent: Classic status, From side, Tuck | `SidebarSettingsMaker.appearance(…, motion:)`, `LookSettings.apply` |
| S08 | Motion on: drag a row down and up the list | the lines it passes step aside on the quick spring and a gap opens where it would land; the dragged row leaves the list; no drop line is drawn; Esc and letting go settle everything; a drop on a row's middle (join) moves nothing | `ListRoom.swift`, `DropFrame`, `SidebarView+Drag.swift` `aim`, FFI `room_pushes`. TEST (visual): rows are pushed inside their own layout box, so the drop still lands on the right line |
| S07 | Motion on: rest the pointer on the line between a row's top quarter and middle | the part does not flip back and forth; with Motion off the parts are cut at the plain quarter edges | `DropLines.over(at:held:part:)`, FFI `drop_part_at`, `SidebarAppearance.motionInForce` |
| S08 | A terminal closed or made inside a group's card (insert pose set to Reveal, From side, Soft spring) | the row plays the pose in the card as one in no group does | `GroupRows.swift` |
| S08 | Make a split (⌘D or drop a row on another's middle) a while after launch | the bracket line fades in down the two rows in 160 ms (first row's stretch first); at launch and after a restore it is simply there; Reduce Motion cuts | `SplitLine.swift`, FFI `bracket_draw_ms`, `bracket_share` |
| S08 | Motion on, status bar on: an agent starts working while another finishes | the counts turn over like a counter (160 ms); with Motion off or Reduce Motion they just change | `StatusBar.statusRoll` |
| S03 | Press + on a group with no folder whose terminals are in a project folder; tick the box | the alert has Use Folder / Not Now and a box "Only keep terminals in this folder"; with terminals elsewhere the detail says how many would leave; Esc is Not Now; with the box ticked those terminals leave the group; Not Now is remembered; the terminal opens either way. TEST (data loss: terminals leave the group) | `GroupFolderQuestion.swift`, FFI `group_folder_leaving`, core `WorkspaceModel::members_leaving` |
| S09 | Same question from the tab bar's +, from an empty pinned set opened in the switcher and by Ctrl-Opt-N on it | the question is asked before the terminal opens | `WindowHub+Sidebar.swift` `newTerminal(inGroup:)` |
| S09 | Another group has a terminal that needs you, then one that failed | the dot on the switcher button and the card is amber for needs-you (louder) and red for a failure; a plain terminal with a bell does not make a dot | `TabAlert`, `TabItem.alert`, `TerminalStatusTile.alert(of:)` |
| S09 | Switcher search: type a terminal's name and a group's name | the terminal's hit carries its tile, the group's a colour dot | `GroupSwitcherView.hitMark` |
| S09 | Click a tab, then type | the keys are in that terminal | `WindowHub+Sidebar.swift` `tabActions` |
| S01 | Fold the group of the selected terminal, F6 into the list | the keys are on the group's head, the first arrow moves from there | `SidebarView.enter` |
| S10 | First run, no coding agent on the Mac, end the intro window on "Ready when you are" with Start | the sidebar's no-agents note is up (unless closed for good) | `OnboardingWindow.onEnded`, `AppController.showNoAgentsTip` |
| W11 | Edit a Probe's `probe.toml` behind the app's back, wait a minute | the Probes are read again | `HookHost.start` (one timer a minute), `HookServerHandle.reloadProbes` |
| W01 | Open the pane menu; look at tooltips of Zoom and Close; change a key in Settings | the menu items and the tooltips show the keymap's keys now | `WindowHub+PaneKeys.swift`, `PaneActions.shortcut/keyText` |
| W03 | Hover the attention button; change "Go to waiting" key | the tooltip ends with the key the keymap has now, nothing when it has none | `StatusSummary.attentionTooltip(jumpKey:)` |
| W08 | Close a group whose worktree has a terminal in a folded group | the worktree is named as shared, so not removable | `AppController+WorktreeClose.swift` (`state.all`) |
| W12 | Window of profile B: Notes preview | note text is in B's interface font | `NotePreview.swift` |
| X04 | Tab strip focused (←, →) | steps between the tabs shown (the set's), a folded group's too | `WindowFocus+Keys.swift` `walkTerminals` |
| S01/S02 | NOT REAL: `RenameField.font` is already `Font?` (nil is the row's own), so nothing reads `tokens` in a default value | - | `SidebarUI/Row/RenameField.swift:16` |
| W01 | NOT WRITTEN: the split's opening motion (new pane slides in on the spring, the old terminal keeps its grid's size until the end and is resized once). It needs the core's `split::Opening::of` over two layouts exported through the FFI, and a pane host that holds a terminal at a fixed size while its box changes; writing that blind would risk a resize storm on the PTY | - | `PaneArea.swift` still animates the layout as a whole (`Motion.layout`) |
| W09/S10 | NOT WRITTEN: the terminal welcome's ending already raised the note; the intro window's is now too (above) | - | - |

## Lane settings-studio, wave 6

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| ST03 | TEST. Set `"notifications": false` with `notify_bell: true` in config.json, open Settings > Notifications | every kind switch reads off; turning one on writes it and sets `notifications` true; turning the last kind off sets it false | `crates/wormhole-ffi/src/settings/state.rs` (`values`, `set_many`, `NOTIFY_KINDS`) |
| ST03 | Open Notifications in a test instance | the permission row says "Outside the app bundle" with its hint and no button; no real permission prompt | `SettingsUI/Panels/NotifyControl.swift`, `Wormhole/Notices/NoticeFeed+Test.swift` |
| ST03 | TEST. In a real bundle press Allow notifications, then Send test | macOS asks once; the test row says Waiting, then Sent, or Denied / Unanswered with Open System Settings beside it | `NoticeFeed+Test.swift` (`runTest`, `publishStatus`), `SettingsPlatform.swift` |
| ST03 | NOT DONE: the test row's "refused with the error" state | the state exists (`NotifyStatus.Test.refused`) but nothing sets it: `SystemNotifier.post` only logs a failure | `PlatformUI/Notifications/SystemNotifier.swift:49` |
| ST03 | Turn "Needs input" row on a Developer ID build | a Time Sensitive chip and the plain description otherwise | `RowExtras.chip`, `RowCaptions`/`EntryRowView.caption`, `Notifying.deliversTimeSensitive` |
| SD01 | TEST. Switch to a second profile, open View > Theme Studio, save a theme | the theme file lands in that profile's `themes` folder, and the list shows that folder's themes | `Wormhole/Platform/StudioLauncher.swift` (`themesFolder`), `AppController.swift:80` |
| SD01 | Switch profile while the Studio is open | NOT handled: the open Studio keeps the folder it opened with; reopen it | `StudioLauncher.show` |
| ST02 | Appearance > Branch colour > Custom | swatches of the theme's ANSI colours and a colour well; a pick writes `branch_color_custom` and keeps Custom; the row says "Pick a colour below" until one is picked | `SettingsUI/Rows/RowExtras.swift` (`BranchSwatches`), `RowCaptions.branch` |
| ST02 | Turn blur off and opacity to 80 % | a warning under Blur with "Turn on blur" (hidden while a project sets blur) | `RowExtras.swift` (`BlurWarning`), `settingsBlurUnsafe` |
| ST02 | Turn on Reduce Transparency or Increase Contrast | the opacity slider is off with the note; Liquid Glass (if on) says why it shows no glass; Liquid Glass wears a Beta chip | `EntryControl.swift`, `RowExtras.swift` |
| ST02 | Appearance > Primary colour, theme's own off | description says "Yours · #hex" / "Pick a colour below"; swatches of the theme's colours under the row | `RowCaptions.primary`, `RowExtras.swift` (`PrimarySwatches`) |
| ST02 | Terminal > Terminal font; Fonts > Custom fonts | "JetBrains Mono 14 pt · line height 1.30 · ligatures on" under the font row; "N families found: …" or "None yet" | `RowCaptions.swift` |
| ST02 | config.json with `scrollback_lines: 75000`, `default_shell` naming a missing profile, `open_in` naming a removed app | "75,000 lines"; the shell row says "Not found: the login shell runs instead"; the app row says "Not installed: …" | `OptionPicker.unknown`, `RowCaptions.gone` |
| ST02 | Title row list | the flexible space says what it does; Terminal status says the status bar shows it while the bar is on; switching a part on goes through core's `show_title_part` | `PartsList.swift`, `settings/page_rules.rs` |
| ST02 | Fonts specimen | tabs Terminal / Code / Interface at each slot's family and size (line height 1.4 outside Terminal) and a facts line | `SettingsUI/Preview/FontsScene.swift` |
| ST02 | Hover Make default terminal | tooltip by state | `EntryControl.swift` |
| ST03 | TEST. Agent access > a stored approval: untick Run | the tick is kept in the database (`agent_grants.caps`), streams started under it end, the row stays ticked/unticked after a reopen | `crates/wormhole-ffi/src/aux_backends/access/approvals.rs` (`set_capability`), `hook_server/mod.rs` (`set_approval_capability`), `ApprovalList.swift` |
| ST03 | TEST. Coding agents with a profile that has a login folder for an agent (inside `WORMHOLE_AGENT_HOME`) | a line "<profile> profile · Status hooks on in <folder>" under the agent with Install / Repair / Remove; a folder outside `WORMHOLE_AGENT_HOME` is refused | `crates/wormhole-ffi/src/settings/agent_logins.rs`, `AgentList.swift` |
| ST03 | Open Coding agents right after the window opens | "Looking for agents…" until the first read lands | `AgentList.swift`, `PanelRecords.agentsRead` |
| ST03 | Advanced > Reset tips | tooltip says whether a tip was closed for good | `EntryControl.swift` |
| ST01 | Search for something that does not exist | a card with "Nothing matches …", the config.json sentence, Open config.json and Clear search | `SettingsUI/Pages/ResultsPage.swift` (`NothingMatches`) |
| ST01 | Search, then pick a section in the nav or "Open <section>" | the section opens with its matches still marked; typing in the search brings the results back | `SettingsModel.swift` (`browsing`, `showsResults`), `SettingsPage.swift`, `SettingsNav.swift` |
| ST01 | Hook one agent partly (remove an event by hand) | the nav's Coding agents shows a warning dot | `SettingsNav.attention` |
| ST01 | Open a project that sets a row | the row carries the project's name as a chip, shows no config key or menu, and its swatches/buttons are held back | `EntryRowView.swift` |
| SD03 | Appearance > theme cards under Dark, Light, Match system | one shelf under Dark or Light, two under Match system; four to a row | `ThemeCards.swift` |
| SD03 | A user theme file with an unknown key (made with a newer Wormhole) | its card wears a warning mark with "n values were skipped" | `crates/wormhole-ffi/src/theme_resolve.rs` (`list`: `newer`), `ThemeCard.swift` |
| SD03 | In a trusted project with its own theme | a notice over the cards naming the project, with Project settings | `ThemeCards.projectNotice`, `ProjectSettingsFollower.swift` |
| SD03 | TEST. Hide a theme (card menu) in Swift, then open the GPUI app on the same profile | the GPUI picker also hides it: `hidden_themes` holds the card's value, by core's rule | `settings/theme_picks.rs` (`theme_hidden`, `set_theme_hidden`), `ThemeCards.swift` |
| SD03 | Filter themes; arrow keys on a card | placeholder "Filter 60 themes"; "No themes match the filter." when none; arrows, Home and End walk the shelf | `ThemeCards.swift` |
| SD03 | Hidden themes (n) open | each name has Dark or Light beside it | `ThemeCards.kind(ofHidden:)` |
| STX1 | Flip two Probe switches quickly | the second waits while the first is read | `ProbesModel.swift` (`pending`), `ProbeRowView.swift` |
| STX2 | Trust a project from the Project tab, then come back to Settings > Project folders | the page reads again when the window comes forward (not every second) | `ProjectsPanel.swift` |
| STX2 | Type in the project search, press Return | the first project left is opened | `ProjectsPanel.swift` |
| STX3 | Turn the "Settings components" flag on | a Components page in the nav with the real controls on sample state; nothing is saved | `SettingsUI/Pages/ComponentsPage.swift`, `SettingsNav.swift`, `SettingsModel.showsComponents` |
| STX3 | Open Coding agents skills before the first read | no "none found" note until the read lands; each row has the agent's tile | `SkillList.swift` |
| SD01 | NOT DONE: expression completions from core, core's contrast fixes, metric preview, per-row appearance chip, `DiskThemeFolder`/`VersionPolicy` copies | unchanged | Studio files named in the audit |
| SD02 | NOT DONE: half-turned status tiles, tooltip/dialog uses, core's `TOKEN_USES`, lazy gallery | unchanged (by construction or speed, see the audit) | `StudioUI/Gallery` |

## Lane devctl, wave 6 (second pass)

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X12 | `note-seed 5 12`, `note-toggle 2`, `notes`; `task-seed 5 2`, `task-done 1`, `notes` | five notes (the fresh one open), note 2 folded, steps `Step n` with two done spread among them, step 1 flipped | `Wormhole/AppController+ControlNotes.swift` |
| X12 | `note-add a`, `note-add b`, `send`; with a program in the terminal that echoes: `send-submit`; `stash` with and without text typed in an agent's box | the note's first paragraph appears at the prompt (and runs for `send-submit`); stash says whether a note was made | `ControlNotes` (`NotesStore.send`, `stashPrompt`) |
| X12 | Click a changed line in an open diff (`click x y`), then `comment hello`, `comment` again | first: `1 comment`, bar on the line, the popover closes; second: `error: no comment dialog is open`. The popover closing is by `.onChange(of: comment)` in `DiffRowView` | `ChangesStore.commenting`, `DiffReview.opened`, `DiffRowView` |
| X12 | `compare main`, `diff 1`, `compare HEAD`; `picker compare ma`, `picker history` | the list is of `main`; HEAD returns to the working tree; the picker listing prints `branch main` lines filtered by the query (the real picker's own filter is the view's, this filters by name) | `ControlNotes.pickerListing` |
| X12 | `history 1`, `commit-details`, `commit-more`, `commit-details`, `history 1` (close), `commit-details` | block as text with `message: compact` then `whole` (the view's Show more follows: its state moved into `HistoryStore.bodyWhole`); `none` when closed. TEST of the view: Show more in the menu still flips it | `HistoryStore.bodyWhole`, `CommitDetailsView`, `ControlNotes.commitDetails` |
| X12 | In a folder without `.wormhole`: `tab project`, `project-init`, `project-state` | the folder is created (TEST: scratch folder only); `project-state` shows it | `ProjectStore.create` |
| X12 | TEST: with a group that has no folder but terminals in one folder, click the group's + (`click x y`), `group-ask` (the folder, the group and how many terminals would leave), then `group-ask use`, `group-ask use only` or `group-ask later` on another group; also the tab bar's + and Ctrl-Opt-N | the question is HELD (the old code ran `NSAlert.runModal`, which a test instance cannot answer); `use` sets the group's folder, `use only` also keeps only this folder's terminals, `later` declines; the terminal opens after the answer, whatever it was; there is no Cancel. In a normal run the system alert (two buttons, Esc is Not Now, the checkbox) is unchanged | `SidebarUI/Group/GroupFolderQuestion.swift` (`asker`, async `settle`), its three callers (`SidebarView+Group.newInGroup`, `SidebarContext+Away`, `WindowHub+Sidebar.newTerminal(inGroup:)`), `AppController+ControlSidebar.installGroupAsker` |
| X12 | `rename-edit 2`, then `text Name`, `key return`, `rows` | row 2's name is a field; the text renames it | `ControlSidebar` (`SidebarLink.ask`) |
| X12 | TEST: `folder-color /tmp/scratch teal`, `folder-color /tmp/scratch #ff8800`, `folder-color /tmp/scratch none`, `folder-color zzz` | each says `<colour> <folder>` and the Project tab's swatch follows (writes the profile's `folder_colors`: scratch data dir only); `zzz` lists the names | `ControlSidebar.folderColor` (`ProjectBackend.setFolderColor`) |
| X12 | `set feature_flags={"folder_picker":true}`, `folder-picker open`, `folder-picker type ~/`, `folder-picker state` (until `busy false`), `down`, `tab`, `state`, `left start`, `enter`, `screen` | the state lists rows with `>` on the selection; tab goes into the folder; enter pastes the path at the terminal's cursor; `scroll` says nothing moves. `open` with the flag off says so | `ControlSidebar.folderPicker`, `FolderPicking.driver`, `FolderPickerDriver.isBusy` |
| X12 | TEST: `profile-new Work`, `profiles`, `profile-open <id>`, `sessions`, `profile-move <id>`, `profile-delete <id>`, `profiles`, `sessions` (scratch data dir only) | the list has `* default` and the new one; a window opens in the profile; the front window moves; delete removes the profile and ends its windows (as Settings' delete); `profile-delete default` and an unknown id say why | `Wormhole/AppController+ControlProfiles.swift` (`RustSettingsHost.profiles/removeProfile`, `openWindow(inProfile:)`, `move(_:toProfile:)`) |
| X12 | `set feature_flags={"notices":true}`, `notice sticky`, `notice chip`, `notice-click info`, `notice chip`, `notice-click close`, `notice chip`, `notice-click` | the sticky chip shows (sticky=true), ⓘ opens the explanation (info=true), × dismisses; with no chip: `error: no chip to click` | `AppController+ControlNotice.swift` (`sticky`), `AppController+ControlNotices.swift` (`NoticeCenter.dismiss/clickInfo/clickBody`) |
| X12 | `agents`-fixture terminal needing input: `notify-sim needs-you 1`, `notify-sim command-done 1 post`, `notice-log`, `notify-sim bogus` | the line says Banner or Nothing with level/sound/bounce and the words as the core writes them (compare with GPUI's line for the same settings); `post` puts the banner in the log; a bad kind lists the kinds | `crates/wormhole-ffi/src/notify/simulate.rs` (`notify_simulate`), `PlatformBridge/NoticeSimulation.swift`, `ControlNotices.notifySim` |
| X12 | `notify-permission denied`, `notify-permission allowed`, `tab` of the welcome (`welcome`) | the welcome's notification page follows (`hooks.tellPermission` reads the notifier's permission within a quarter second); a real instance (not background) says it has a real centre | `LogNotifier.pretend`, `ControlNotices` |
| X12 | NOT WRITTEN, no counterpart: `notice-material glass|frosted|solid|system` | - | the Swift chip is always Liquid Glass (`TerminalUI/Notice/NoticeChip.swift:54`); there is no material choice to force |
| X12 | `hosts`, `hosts-list`, `hosts-toggle`, `hosts-scroll 300`, `window-shot /tmp/h.png Session Hosts` | the first card opens (or folds); the list scrolls (`ScrollPosition.scrollTo(y:)` in `HostsView`: if the page is shorter than 300 nothing moves) | `AuxWindows/Hosts/HostsWindow.swift` (`model`), `HostsViewModel.scrollRequest`, `HostsView` |
| X12 | `licenses`, `licenses-search tokio`, `licenses-select serde`, `licenses-search zzz` | the count line and selected package follow; select clears a hiding search; no match: `0` shown | `AuxWindows/About/Licenses.swift` (`browser`), `ControlPages` |
| X12 | `intro-frames voyage 4`, then open the BMPs | four files `<ctl>/intro-voyage-0..3.bmp`, the shader's frames over the stand-in window (no window needed; compare GPUI's, which spans the run the same way) | `AuxWindows/Intro/IntroFrames.swift` |
| X12 | `onboarding look`, `onboarding-tailor 1`, `back`, `over`, `ok` | the walk answers, goes back, starts over, settles; `onboarding-tailor 9` is refused by the core (no change) | `ControlPages` (`LookExtras.act`) |
| X12 | TEST: `welcome-probe - nottoken {"op":"state"}` (see `crates/wormhole-core/src/ipc/control/welcome.rs` for the JSON of `WelcomeOp`) | an error reply line, never a state; with a real welcome terminal's token it answers | `crates/wormhole-ffi/src/hook_server/control_line.rs` (`welcome_line`), `HookServerHandle.welcomeLine` |
| X12 | `update-installed 9.9.9`, `window-shot s.png Settings` on the Updates page, `update-installed 9.9.9 pending` | the Updates page says installed with Restart, then the idle wait line | `AuxBridge/UpdateBridge.swift` (`pretend`) |
| X12 | `import-scan <a folder with .config/ghostty/config>` | settings open on Import, the scan lists `ghostty (Ghostty: 3 settings…)`; `import-open ghostty`, `import-apply ghostty` still work over that home | `RustSettingsHost.importHome`, `ControlPages.importScan` |
| X12 | `settings-section fonts`, `font-picker terminal jet`, `window-shot` | the picker's popover is open with its query (a popover in a background instance may not draw: the answer line is the check) | `SettingsUI/Rows/FontControl.swift` (`SettingsFontPickers`) |
| X12 | `settings-section shortcuts` (or the page with the hotkey row), `settings-rows`, `settings-record <row id> cmd-k`, `settings-reveal <id>` | the recorder starts and takes the key (the row shows it; a rejected one says why); reveal opens the page and marks the row | `ControlPages.settingsRecord` (`Injector.press`, `sendKey`), `SettingsModel.reveal` |
| X12 | `set font_size=15`, `settings-reset-confirm`, `question`, `question cancel`, `get font_size` | the question is held (the settings are still 15); `question confirm` instead would reset | `SettingsUI/Rows/ResetAllButton.swift` (`SettingsActions.askReset`) |
| X12 | TEST: `settings-cli-tool`, `settings-cli-tool install`, `settings-cli-tool uninstall` (scratch data dir; `WORMHOLE_CLI_DIR` is the instance's own) | state notInstalled, then installed with the folder under the data dir, then notInstalled; never `/usr/local/bin` | `ControlPages.settingsCommandLine`, `Isolation+Homes.swift` |
| X12 | NOT WRITTEN, no counterpart: `settings-preview <regular|compact|dense|off>` | - | the Swift Terminals & sidebar scene draws all three densities side by side with the current ringed (`SettingsUI/Preview/SidebarScene.swift`); there is no hover preview of one density |
| X12 | `right-click` on a group head (or `group-menu <id>`), `menu-native`, `menu-native press 3`, `rows` | the menu is printed with its tooltips and `dots: 1. No colour | 2. Red …`; pressing dot 3 colours the group (the menu's own callback) | `PlatformUI/Menus/SwatchMenuView.swift` (`SwatchMenuProbe`), `AppController+ControlMenus.swift` |
| X12 | `path-menu`, `menu`, `menu-pick Copy Path`, `pasteboard`; `files-menu`, `menu`, `menu-pick Follow Terminal` | the title row's folder menu lists Open in Finder / Copy Path / Insert into Terminal and the folders up the path; the Files menu lists the folders, pins, Pin, Choose Folder…, Follow Terminal (the SwiftUI button's own menu is separate: both are made from `PathMenuModel`, compare the items by eye) | `TitleKeyMenu.pathMenu`, `WindowHub+PathMenu.swift`, `PanelUI/Files/FilesPathMenu.swift` |
| X12 | `available toggleSidebar`, `available bogus`, `available copy` (terminal with and without a selection) | true; false; follows the Copy item's enabled mark | `MenuController.availability` |
| X12 | Open a group's popover (the group head's ⌃↩ menu → Other colour…, or `click` on the head's name), then `popover`, `popover-type Build`, `popover-colour teal`, `popover-press Ungroup`, `popover`, `popover-key enter`, `popover-close` | rows listed; the name field takes the text (committed on close or ↩); the colour applies at once; Ungroup runs and closes; after close `popover` says `no popover`. TEST: the SwiftUI popover must still look and behave as before (its action list became a `ForEach` over `actions`) | `DesignSystem/PopoverRecord.swift`, `SidebarUI/Group/GroupPopover.swift` (`record`, `actions`) |
| X12 | `popover-native card`, `popover-real`, `window-shot s.png`, `popover-real-close` | a PNG of the popover's content; a real NSPopover appears at the window's top left and goes (in a background instance it may take no keys: only its look is checked) | `ControlMenus.popoverPicture`, `realPopover` |
| X12 | `drag 600 400 /tmp/a.png`, `window-shot /tmp/d.png`, `drag-end`, `window-shot /tmp/e.png`; `drag 5 5 /tmp/a.png` over the title row | the terminal tints while hovered and is clear after `drag-end`; over something that takes no drop it says so | `AppController+ControlDrag.swift` (`StandInDrag: NSDraggingInfo`: its conformance is the likeliest compile error here) |
| X12 | `new-window`, `use-window 2`, `close-window`, `question`, `question confirm`, `sessions` | the second window closes (after the question if something runs in it), one remains; `close-window` on the last quits the app | `ControlDrag.swift` (`performClose` into `shouldClose`) |
| X12 | KEPT, one line each: `pointer ROW COL` keeps its Swift meaning (a cell of the terminal; GPUI's `pointer <x> <y>` in points has no second name, `hover x y` is its place) | - | `AppController+Control.swift` |
| X12 | KEPT: `hosts` opens the Hosts window (GPUI's `hosts`, the session hosts as text, is `hosts-list`) | - | `AppController+ControlWindow.swift`, `Driving/AppController+ControlTooling.swift` |
| X12 | KEPT: `front` is not written (a background instance must never take focus: `Driving/InFront.swift`, `window_open` rules); `notices` keeps its Swift meaning (the logged banners; GPUI's per-window chip listing is `notice chip`) | - | - |
| X12 | KEPT, GPUI-only mechanisms with no Swift command: `paints`, `refresh` (GPUI's paint/cache counters), `scaling settle`/`reveal-test`, `grow-hold` (GPUI's resize animation and first-frame tricks), `scroller` (the system draws Swift's scroll bars), `a11y live`, `notice-material`, `settings-preview` | - | see the lines above |

## Lane crosscut, wave 7

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X-test TEST | run HookBridgeTests with `HOME` watched | every hook client reads and writes under the test's scratch `agent-home` (`WORMHOLE_AGENT_HOME`, `CODEX_HOME`, `CLAUDE_CONFIG_DIR`), nothing under the real home | `Tests/HookBridgeTests/HookBridgeTests.swift` `setUpWithError` |
| T01 | block cursor at column 0 / last column / row 0 on a coloured full-screen background | the cell's own background still runs into the padding beside the cursor, no notch of the clear colour | `TerminalMetal/RowBuilder.swift` (quad before the cursor's) |
| X04 | Esc with a menu open | unchanged: GPUI passes `popup: false` too (`window/focus/mod.rs:202`), nothing to wire | `WindowUI/Focus/WindowFocus.swift` |
| X08 | `Wormhole --json list` | "unknown command" exit 2 as GPUI's; not a launch | `Wormhole/main.swift` |
| X12 | `new-shell <key> <dir>` | the terminal starts in `<dir>` (`~/` expanded) with the shell the key names | `AppController+ControlRead.swift`, `WorkspaceAction.newTerminalWithShellIn` |
| X12 | `a11y` | SwiftUI's nodes appear under the hosting views (children, else navigation order, else subviews) | `Driving/AccessibilityDump.swift` `children(of:)` |
| X12 | launch, then look in `<data>/logs` | the Swift side's lines (Dock, notifier, links, profile data folder) are in the daily file, target `swift` | `WordsModel/AppLog.swift`, `Platform/LaunchLogging.swift` |
| X12 | switch to a profile whose folder cannot be opened | an error line "persistence disabled" in the log, and the chip as before | `Settings/ProfileChipBinding.swift` |
| X10 | `swift test --filter TerminalCoreTests` | the cursor blink flip rebuilds at most its row and shapes nothing; a repeat flip costs 0 | `FrameWorkGuardTests.swift` |
| X10 | one run of `macos/scripts/perf-compare.sh` | fill `macos/tools/baselines/swift-perf.json` (all numbers are null today) | `macos/tools/baselines/swift-perf.json` |
| X03 | Settings > Changed > Copy JSON with VoiceOver | "Copied" is said once | `SettingsUI/Pages/ChangedPage.swift` |
| X07 | VoiceOver on the Studio's changed dot | says "Changed" from the catalog | `StudioUI/List/BandView.swift`, `swift-studio.ftl` |

## Lane panel, wave 7

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P06 TEST | Notes sync in flight for terminal A (merge returns a line from notes.md), select terminal B before it lands | the merged model is saved for A (database), B's notes are not touched, no line of notes.md is later read as deleted | `Wormhole/Panel/NotesCoordinator.swift` `land`/`store`; test `NotesMergeSavedTests.aMergeForATerminalSwitchedAwayDuringItIsStoredForThatTerminal` |
| P06 TEST | Close a terminal while its merge is in flight; move one to another window meanwhile | closed: its base is dropped again and nothing is saved; moved: the merge is stored for it | `NotesCoordinator.land`, `departed` |
| P06 | Two ticks of the 2 s timer overlap a slow merge | the second run returns at once (`syncing`) | `NotesCoordinator.sync` |
| P05 TEST | Two windows of one profile make a worktree each, then remove one | both records and distinct ports remain; neither window's list loses the other's | `crates/wormhole-ffi/src/git/worktrees.rs` `update`; test `two_services_of_one_profile_keep_each_others_worktrees_and_ports` |
| P05 TEST | `Wormhole new-terminal --worktree x` twice for one window, then for another | second for the same window: Failed; another window: asked. A sheet opened from the menu also refuses a request for its window | `hook_server/control/worktree.rs`, `core.rs` `worktree_question`, `AppController+Windows.swift` `sheetChanged`; test `a_second_worktree_request_is_refused_only_for_the_window_whose_sheet_is_up` |
| P05 | `new-terminal --worktree x --window 2` with window 1 in front | the sheet comes up in window 2, trust judged by window 2's profile | `HookHost.perform(.worktreeAsked)`, `control/worktree.rs` |
| P08 | Layout with terminals where the first has `focus` and later ones do not | the first with focus is selected at the end, not the last made | `Panel/PanelEffects.swift` `.openTerminals` |
| P08 TEST | A project action with a key, focus in a text field, the sidebar or the panel | the key runs the action; with the project untrusted or Actions off the key goes on | `Panel/ProjectActionKeys.swift` `ProjectKeys`; FFI `ProjectService.action_for_key`; test `an_actions_key_is_the_projects_only_while_it_is_trusted_and_on` |
| P02 | Click the branch chip of a pane that is not selected; click it in a folder that is no repository | picker opens for that pane's terminal only; in a non-repository the request ends and no picker pops later | `ChangesStore.pickerFor`, `WindowHub+Branch.swift` |
| P09 | Two windows in two profiles; an agent reports | the report is added to the profile Wormhole is in; each window draws its own profile's; moving a window changes only that window's meters | `usage.rs` `profile_reports`, `RustUsageBackend.bound/follow/setShown` |
| P09 | A report that puts a limit into its last minute while the Usage tab is up | the words go per second at once, not up to 60 s later | `UsageStore.keepTime`, `nap` |
| P07 | Terminal in a folder whose name is not UTF-8 | NOT DONE: folders are `String` in every layer |  |
| P01 | VoiceOver on a tab | NOT DONE: SwiftUI has no per-tab trait; the position "n of 6" is said |  |

## Lane style, wave 7

Written without compiling: expect compile errors.

The comparison X01 never had: the GPUI app's shared components in `crates/wormhole/src/style/` against their Swift counterparts, by what a person can meet (states, parts, Reduce Motion / Increase Contrast / Reduce Transparency, accessibility). Looks (pixels) are not compared. "Done" is what this lane wrote; "left" is read and not written.

| component | GPUI file | Swift file | what was missing / what is left |
|---|---|---|---|
| row | `controls.rs` `row`, `rows/` | `DesignSystem/Row.swift` | DONE: disabled (held back, no hover/press), selected trait for VoiceOver. LEFT: no `RingPlacement::Inside` keyboard ring in the shared row (callers draw their own: `SidebarUI/Row/RowRings.swift`); the ring modifier now exists (`keyboardFocusRing`) but the sidebar, panel and palette rows are not moved onto it (their owners' modules) |
| icon button | `buttons.rs` `IconButton` | `DesignSystem/IconButton.swift` | DONE: disabled (faint, no hover/press), `selected(_:)` toggle (selected tint and text, VoiceOver selected). LEFT: `menu_chevron`, `with_drawing`, `in_row` 22 pt face, `narrow`, `flush_*`, active-filter dot and `strip` fills are drawn by their callers (`TitleIconButton`, `BandButton`, `RowActions`), not by the shared one |
| row hover actions | `buttons.rs` `row_actions` | `PanelUI/Shell/RowActions.swift` | none found in DesignSystem; the panel's own. LEFT |
| pill / chip | `controls.rs` `pill` | `TerminalUI/Notice/NoticeChip.swift`, `FontPickerUI/TintChip.swift`, `PanesUI/Title/*Crumb.swift` | two chips and the crumbs each draw their own fill; no shared chip with active, compact and hover states. LEFT (a shared `Chip` would move five callers in four modules) |
| icon tab | `controls.rs` `icon_tab` | `WindowUI/PanelTabBar.swift` + `PanelUI/Shell/TabCountBadge.swift` | states, count, VoiceOver tab bar present; keyboard ring is the window's own `focusRing`. OK |
| title tab / terminal tab | `controls.rs` `title_tab`, `tabs.rs` | `PanesUI/Tabs/TerminalTabView.swift` | states, close on hover, VoiceOver selected present. OK |
| count badge | `labels.rs` `count_badge`, `selected_count_badge` | `DesignSystem/CountBadge.swift`, `PanelUI/Shell/TabCountBadge.swift` | the selected variant is a second type (`TabCountBadge`) not a state of the first. LEFT (one concept twice) |
| status badge, section label, key cap, banner | `labels.rs`, `controls.rs` `keycap`, `banner` | `DesignSystem/StatusBadge.swift`, `SectionLabel.swift`, `KeyCap.swift`, `Banner.swift` | states match (no pointer states in GPUI); Increase Contrast outlines present; key cap speaks. OK |
| segmented | `controls.rs` `segmented`, `segment` | `DesignSystem/Segmented.swift` | DONE: disabled opacity, the track as a VoiceOver container. |
| text field | `text_field.rs` | `DesignSystem/FlatField.swift` (new), `SettingsUI/Controls/InputField.swift`, `PlatformUI/Shared/PanelField.swift`, `PaletteUI/PaletteField.swift` | WAS: settings used the system's rounded field (no `control.background` well, no hover, no primary ring, no Increase Contrast edge, no prefix/suffix/clear, no disabled). DONE: `FlatField` with all of them (gallery only: Settings keeps the system's rounded field, the owner's choice). LEFT: `multiline` (the Rust `TextareaState`) has no Swift field yet (notes use `TextEditor`); the two panel fields (`NSTextField`, ring off on purpose, the panel draws its own) and the Studio's name/expression fields are not on `FlatField` |
| focus ring | `controls.rs` `focus_ring`, `flat_ring` | `DesignSystem/FocusRing.swift` (new), `WindowUI/Focus/FocusRing.swift` | WAS: only WindowUI had one, in the window's chrome colours, so DesignSystem parts could not draw it. DONE: `keyboardFocusRing(_:radius:outset:)`, hidden in a window not in front (`windowFront`). LEFT: the other seven hand-drawn rings (SidebarUI x3, SettingsUI x3, StudioUI) are not moved onto it |
| checkbox, switch, radio | `checkbox.rs`, `setting.rs` `toggle` | `SettingsUI/Controls/SettingToggle.swift` | the system's toggles (tint from the theme): GPUI's `checkbox.background/border/checked` tokens do not reach them. A state Swift cannot have without owning the drawing; written down, left |
| button | `button.rs`, `buttons.rs` | `SettingsUI/Controls/ActionButton.swift`, `AuxWindows/Shared/AuxButtons.swift` | the system's bordered button: no `button.background`/`button.hover` tokens, no loading state, no Increase Contrast flat outline. LEFT (a `ButtonStyle` of the tokens is the next step; it moves ~40 callers) |
| link button | `setting.rs` `link_button` | `SettingsUI/Controls/ActionButton.swift` `LinkButton` | no hover fill past the text; the system's link style. LEFT |
| slider, stepper | `slider.rs`, `setting.rs` `stepper` | `SettingsUI/Controls/SettingSlider.swift`, `ValueStepper.swift` | slider disabled at half opacity not checked; LEFT for the verify wave |
| choice card | `choice_card.rs` | `SettingsUI/Rows/ThemeCards.swift` | focus ring and chosen outline present there (own code). OK |
| disclosure | `controls.rs` `disclosure` | `PanelUI/Shell/Disclosure.swift`, `DesignSystem/SectionLabel.swift` | Reduce Motion honoured. OK |
| find bar | `find_bar.rs` | `TerminalUI/Find/FindBar.swift` | focused field outline; checked by the find lane |
| tile (status tile) | `tile/` | `SidebarUI/Tile/*`, `PanesUI/Panes/TerminalTile.swift` | two tiles: `TerminalTile` (pane header's own fallback) and `StatusTile`; the window gives the real one through the seam. LEFT: `TerminalTile` is only the fallback (previews) |
| menu and popover layers | `menu_layer.rs`, `popover_layer.rs` | AppKit menus and popovers (`NativeContextMenu`, `PopoverRecord`) | GPUI draws them; Swift uses the system's. Cannot have a GPUI-drawn menu: left on purpose |
| X05 Reduce Motion consumers | `motion/` | `DesignSystem/Motion.swift` | every `motion`/`pressed`/`motionTransition` call falls back to a fade through one place; the 23 direct `reduceMotion` readers in SidebarUI/WindowUI/PanelUI were read: all gate by `!reduceMotion`. Not re-read against GPUI's list line by line. LEFT |
| X02, X06 | theme files, legacy upgrades | n/a | audit caveats (version-1 theme files, one app-wide project theme, legacy upgrades) are product questions, not code: not touched |

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X01 | Studio gallery, Inputs | four fields: rest, focused ring, cleanable with a prefix and the x (clears, VoiceOver says Clear), disabled | `StudioUI/Gallery/InputsSpecimen.swift` |
| X01 | Studio gallery, Chrome | icon buttons: one selected (tint, VoiceOver selected), one disabled (faint, no hover) | `StudioUI/Gallery/ChromeSpecimen.swift`, `DesignSystem/IconButton.swift` |
| X01 | any `Row` inside `.disabled(true)` | held back, no hover or press fill | `DesignSystem/Row.swift` |
| X01 | the tile seam: tab, group card, pane header, title row, status bar | the row's own status tile shows; no `AnyView` (count 19) | `WindowUI/Hub/WindowHub+Tiles.swift`, `PanesUI/*`, `Package.swift` (PanesUI depends on SidebarUI) |

### Lane style, wave 7 (continued): every file of `crates/wormhole/src/style/`

Written without compiling: expect compile errors. One line per file (81 files, sub-folders included). "read" says how far I went: **full** = the GPUI file's states and parts compared with the Swift file; **header** = the file's doc comment and its public items read and the Swift counterpart found by name, states not compared line by line (the verify wave compares those). The system's button, checkbox, switch, link and slider stay the system's (owner's choice for Settings); GPUI-drawn menus and popovers stay AppKit's.

| GPUI file | what it is | Swift counterpart | what a person meets in GPUI that Swift lacks (or "nothing found") |
|---|---|---|---|
| bands.rs | component: the band (header of a group of rows: chevron, title, count, +, folder tint), `section_card` | `PanelUI/Shell/CardBand.swift`, `BandButton.swift`, `BandMenu.swift`, `DesignSystem/CountBadge.swift`; `SidebarUI/Band/` (header) | **full** (wave 8): drop-target fill, held-⌘ count dim (`HELD_COUNT_OPACITY`) and the folder-tint pill exist in `band`, but every caller passes `drop_target: false` and only `section_card`'s default `FolderLook` (studio token list, notes, changes, history, project): no screen shows them, NOT A GAP. Count now the shared `CountBadge` (`BandCount` gone); + now the shared `IconButton` (`in_band`). Hover `band.hover` on a sticky band, `state.hover` otherwise: Swift `CardBand` uses `state.hover`, as the non-sticky case every caller has |
| button.rs | component: Settings' flat button (variants primary, outline, ghost, danger; loading; Increase Contrast outline) | `SettingsUI/Controls/ActionButton.swift` (system bordered) | **full**: variants, loading and the flat Increase Contrast outline: the system's bordered button, by the owner's choice (Settings); left |
| buttons.rs | component: `IconButton` (in_title_row, with_drawing, motion, still, menu_chevron, narrow, in_band, in_row, spoken, marked, strip, focus_ring, disabled, flush) and `row_actions` | `DesignSystem/IconButton.swift`, `SpokenName.swift`; callers `WindowUI/TitleButton.swift`, `PanesUI/Title/SplitButtons.swift`, `OpenInButtons.swift`, `PanelUI/Shell/BandButton.swift`, `RowActions.swift`, `TerminalUI/Find/*` | **full** (wave 8): `IconButton` now has GPUI's faces (`inTitleRow`, `inBand`, `inRow`), `dot` (`marked`), `focusRing`, `spoken`, `answering` (`motion`/`still`), a caller's drawing in the colour of the moment (`with_drawing`, `strip`), `band.hover` with no hover animation, and `spoken_name` (`DesignSystem/SpokenName.swift`, the key as the hint). Callers moved: `TitleButton` (WindowUI), the split and open-in buttons (`TitleIconButton` gone), `BandButton`/`RowActions`, the find bar's steps and toggles, the group head's +. LEFT: `narrow` + `menu_chevron` (Open in…'s chevron and the Studio's save arrow draw their own `Menu`), `flush_*` (callers pad); `close_x`, `hit_area`, `more_toggle`, `hash_link` read: drawn by their screens (`CloseButton`, `PaneButton`), not compared state by state |
| checkbox.rs | component: checkbox (checked, in_form label, spoken, focus ring, `checkbox.*` tokens) | `SettingsUI/Controls/SettingToggle.swift` (system), `PanelUI/Notes/TaskCheckbox.swift` (drawn, panel) | **full**: Settings uses the system's checkbox (owner's choice), left. The panel's task box: GPUI role checkbox with checked state and `spoken` text; Swift a drawn box in a button whose label is the task and value "done / not done" (`TaskRowView.swift:28-31`): same words, not the checkbox role. LEFT: `accessibilityRepresentation { Toggle(...).toggleStyle(.checkbox) }` would give the role but leaves `notes-task-not-done` unused (a catalog change for its owner) |
| choice_card.rs | component: choice card (chosen, hover, focus ring, Increase Contrast outline) | `SettingsUI/Rows/ThemeCard.swift`, `AuxWindows/Onboarding/ThemeCard.swift`, `TailorStep.swift` | **full**: radio role with chosen state, focus ring, hover `control` fill (not on the chosen one), Increase Contrast `card.edge` / primary outline. Settings' `ThemeCard`: chosen ring, hover, Increase Contrast outline, keyboard arrows (`ThemeCards.swift`) present; VoiceOver hears selected, not a radio button. Onboarding `ThemeCard`: always outlined in `border.strong` (GPUI only under Increase Contrast, in `card.edge`), hover an opacity change, not the `control` fill. LEFT (looks) |
| controls.rs | components: row, pill, icon_tab, title_tab, toolbar, disclosure, keycap, segmented/segment | `DesignSystem/Row.swift`, now `Chip.swift` (pill), `WindowUI/PanelTabBar.swift`, `PanesUI/Tabs/TerminalTabView.swift`, `PanelUI/Shell/Disclosure.swift`, `DesignSystem/KeyCap.swift`, `Segmented.swift` | **full** (wave 8): row: `Row` now has the `ROW_H` minimum, the selection's text colour and no hover fill over a selection; pill: the history filter chip is the shared `Chip` selected (GPUI's only real `pill`; an inactive pill hovers only in GPUI's gallery, so `Chip`'s hover has no screen to reach); segmented: a segment's text turns primary under the pointer; under Reduce Motion the selection fades in place (no slide). icon_tab, title_tab, disclosure, keycap: OK as in wave 7. `FormSegments` (one Tab stop, arrows, radio roles): Settings uses the system's segmented picker (owner's choice). Toolbar: layout only |
| extras.rs | components: extra chip, pull-request chip, branch status, agent gauge, context meter | `PanesUI/Title/ExtraChip.swift` (now on `Chip`), `GaugeChip.swift`, `BranchStatus*` in PanesUI/Title | **full**: extra chip (`active` fill + outline, text colour; muted unfilled), pull-request chip, branch status (dirty, ahead/behind), agent gauge, context meter: stateless parts, no hover/press/focus in GPUI. Swift `ExtraChip` on `Chip` (same states), `GaugeChip` follows Reduce Motion. Nothing missing |
| find_bar.rs | component: find bar (field focus outline, count, invalid, options, step, close) | `TerminalUI/Find/FindBar.swift`, `FindField.swift`, `FindToggle.swift` | **full** (wave 8): steps and close are the shared icon button `inTitleRow` (hover `band.hover`, `text.primary`), toggles `inRow().toggled(on)` (selected tint and text, `band.hover`); VoiceOver hears "Next match" with the key as the hint. Focused field outline and invalid count colour as before |
| folder_colors.rs | component: colour dot and colour choice (the Project tab's row) | `PanelUI/Project/ColorChoice.swift`, `FolderColorCard.swift` | **full**: colour choice is a radio (chosen state), focusable with a ring only when enabled, hover `band.hover` when enabled. Swift `ColorChoice`: hover `band.hover` when enabled, disabled held back, selected trait. LEFT: no keyboard focus ring of its own (a SwiftUI button's focus is the system's), VoiceOver selected not radio |
| groups.rs | component: a group's head in the sidebar (name, dot, pin, chevron, folded tiles, count, summary fade) | `SidebarUI/Group/GroupHeadView.swift`, `HeadControls.swift`, `FoldedSummary.swift` | **full** (wave 8): the + is the shared icon button `inRow` (GPUI `icon_button(...).in_row()`), was a 24 pt face with `state.hover`. Label opens the editor, the rest folds; "No terminals" and the + on an empty pinned group; folded summary with the shared `CountBadge` (one copy gone) and alert dot; chevron hover. VoiceOver: GPUI has two buttons (label: edit, chevron: fold with expanded); Swift one element with the fold as its action and Edit group and Rename as named actions: same reach, different shape |
| home.rs | helper: the home folder for `~/` paths | core's `platform::paths` through the bridge (Swift shows paths from Rust) | **full**: not a component (the home folder for `~/` paths; Swift shows the core's shortened paths) |
| icon_motion/canvas.rs | helper: the 24-unit icon grid | `DesignSystem/IconMotion.swift` (`Lucide` canvas) | **full**: not a component (the 24-unit grid) |
| icon_motion/copied.rs | motion: copy becoming a check | `SettingsUI/Controls/CopyButton.swift`, `DesignSystem/IconMotion.swift` | states: Animate icons and Reduce Motion gate it (`IconRun.start`, `IconMotion.plays`); its frames are a look, not compared |
| icon_motion/mod.rs | motion: icons answering with their drawing (+ turn, arrows spin, split fill, zoom swap, strip fill, tab glyphs) | `DesignSystem/IconMotion.swift`, `IconButton.swift` | **full**: which icons answer and that nothing plays under Reduce Motion or Animate icons off, and that a start missed is not replayed (`IconRun`): present. The motion an icon button takes is now the shared button's (`answering`) |
| icon_motion/pane_fill.rs | motion: split buttons filling | `DesignSystem/IconMotion.swift` | states: as copied.rs (gated by `IconRun`); frames not compared |
| icon_motion/strip.rs | motion: panel toggle strip fill | `DesignSystem/IconMotion.swift`, `WindowUI/Columns/ColumnSlide.swift` | states: the strip fills while its column is open: `PanelGlyph(fill:)` drawn in the shared button's colour (`TitleButton` → `IconButton`); frames not compared |
| icon_motion/swap.rs | motion: icon swap (zoom corners) | `DesignSystem/IconMotion.swift` (`IconSwap`) | states: as copied.rs; frames not compared |
| icon_motion/timeline.rs | helper: pure timeline maths | `DesignSystem/IconMotion.swift` | **full**: not a component (timeline maths) |
| icon_motion/trigger.rs | helper: what starts a motion (count of starts) | `DesignSystem/IconMotion.swift` (`IconRun`) | **full**: not a component; its rule (count of starts, nothing replayed after being turned back on) is `IconRun`'s |
| icon_motion/turn.rs | motion: + quarter turn, arrows, chevron | `DesignSystem/IconMotion.swift` | states: as copied.rs; the menu chevron's turn is `menuChevron(open:)`, cut under Reduce Motion |
| icon_motion/tab_glyph/mod.rs | motion: right panel tab glyph plays on activation | `DesignSystem/TabGlyph.swift` | states: plays once as a tab becomes the active one, not under Reduce Motion: `TabGlyph` present |
| icon_motion/tab_glyph/commit.rs | motion: History glyph | `DesignSystem/TabGlyph.swift` | states: as tab_glyph/mod.rs; frames not compared |
| icon_motion/tab_glyph/compare.rs | motion: Changes glyph | `DesignSystem/TabGlyph.swift` | states: as tab_glyph/mod.rs; frames not compared |
| icon_motion/tab_glyph/folder.rs | motion: Project glyph | `DesignSystem/TabGlyph.swift` | states: as tab_glyph/mod.rs; frames not compared |
| icon_motion/tab_glyph/gauge.rs | motion: Usage glyph | `DesignSystem/TabGlyph.swift` | states: as tab_glyph/mod.rs; frames not compared |
| icon_motion/tab_glyph/notes.rs | motion: Notes glyph | `DesignSystem/TabGlyph.swift` | states: as tab_glyph/mod.rs; frames not compared |
| info.rs | component: the (i) after a setting's label (focusable, one open per window) | `SettingsUI/Controls/RowAccessories.swift`, `Rows/SettingRow.swift` | **full** (wave 8): the (i) now has the hover fill, the text colour under the pointer and while its card is open, chip corners, and is a button VoiceOver reaches ("More information"; it was hidden). LEFT: `aria_expanded` (SwiftUI has no expanded state for a button), one open card per window (each (i) keeps its own popover; AppKit closes a transient popover when another opens), the press that closes the card not reopening it (the system popover's) |
| labels.rs | components: section_label, nav_group_label, panel_title, count_badge, selected_count_badge, status_badge, branch_chip, tint_chip, neutral_chip, changed_dot, line_counts, card, flat_card, empty_note, branch_label/mark | `DesignSystem/SectionLabel.swift`, `CountBadge.swift` (WRITTEN: `selected`, one type; `TabCountBadge` removed), `StatusBadge.swift`, `FontPickerUI/TintChip.swift` (now `Chip`), `PanesUI/Title/BranchCrumb.swift` (`Chip`), `PanelUI/Shell/PanelCard.swift`, `Callout.swift` | **full** (wave 8): `count_badge`/`selected_count_badge` are one `CountBadge` with the chip corners (was a capsule) and the only copy (band and folded copies moved onto it). `tint_chip` (`chip.tint`, `selected_text`, medium, optional icon) vs Swift `TintChip` `textMuted`, `neutral_chip` (`band.hover`, `text.primary`) vs `RowChip` `controlBackground`/`textMuted`: colours, a look (unchanged on purpose: the brief keeps callers' colours). section_label, nav_group_label, panel_title, status_badge, changed_dot, line_counts, card, flat_card, empty_note: stateless; present |
| lights.rs | layer: where macOS draws traffic lights and what clears them | `DesignSystem` window helpers / `WindowUI` chrome (system buttons) | **full**: not a component (where macOS draws the traffic lights); the system's lights |
| menu_layer.rs | layer: GPUI-drawn menu | AppKit `NSMenu` (`DesignSystem/NativeContextMenu.swift`) | **full**: GPUI-drawn menu; Swift uses AppKit menus (owner's choice); left |
| messages.rs | components: banner, callout, readout, block readout, key hint | `DesignSystem/Banner.swift`, `PanelUI/Shell/Callout.swift`, `TerminalUI/Scaling/SizeReadoutChip.swift`, `TerminalUI/Blocks/BlockReadout.swift`, `DesignSystem/KeyCap.swift` (`KeyHintView`) | **full**: banner kinds and Increase Contrast outline, callout, readout, block readout (copy button now the shared icon button), key hint: present |
| metrics.rs | scale: sidebar row/card/header sizes per density | `DesignSystem/Scale/SidebarMetrics.swift` | **full**: not a component (sidebar sizes per density); `SidebarMetrics` has the three densities |
| mod.rs | module root: re-exports, the Studio gallery contract | `StudioUI/Gallery` | **full**: module root and the gallery contract (`gallery.py`) |
| motion/easing.rs | scale: curves (core's) | `DesignSystem/Motion.swift`, `SidebarUI/SidebarMotion.swift` | **full**: not a component (curves) |
| motion/mod.rs | scale: how long, ease, spring; Reduce Motion is GPUI's own | `DesignSystem/Motion.swift` | **full** (wave 8): durations and Reduce Motion: `motion`, `motionTransition`, `pressed` fade; new `motionTurn` (a turn swaps at once: group and section chevrons) and `slidingSelection` (a selection fades in place instead of sliding: `Row`, `Segmented`) |
| motion/selection.rs | helper: short selection fade | `DesignSystem/Motion.swift` (`Motion.selection`) | **full**: GPUI's selection is an opacity fade in place, never a slide; Swift slides the shape between rows and segments (a look choice of the Swift app) and, from wave 8, fades in place under Reduce Motion |
| motion/spring.rs | scale: springs | `DesignSystem/Motion.swift`, `SidebarMotion.list/joinSettle` | **full**: not a component (springs; Reduce Motion is the callers') |
| motion/timing.rs | scale: durations | `DesignSystem/Motion.swift` (`timings`) | **full**: not a component (durations) |
| palette.rs | components: palette surface, search row, footer, labels, results with marked text | `PaletteUI/` (`PaletteView`, `SearchRow`, `ResultList`, `PaletteField`) | **full**: surface, search row, footer, labels with count, results: selected `list.selected` with its text and muted, hover `state.hover` (none on the selected), marked text in the primary or the selected text. Swift `ResultRow` present. Missing: `detail_color` (a result's own detail colour; Swift's detail is always muted) |
| panel_list.rs | helper/layer: virtualised panel lists, scroll edges | `PanelUI/*ListView.swift`, `SelectableListSplit.swift` | **full** (wave 8): `scroll_edges`' hairline under the top while rows are scrolled up beneath it: WRITTEN as `scrollEdge()` (`DesignSystem/ScrollEdge.swift`) on the sidebar's list and `PanelScroll` (Changes, History, Notes, Project, Usage, Files: GPUI's users). Its scrollbar is the system's overlay scroller in both. Virtualised rows: SwiftUI's lazy stacks, not compared |
| panel.rs | component: the panel's cards | `PanelUI/Shell/PanelCard.swift`, `CardBand.swift` | **full**: panel card and its segments: stateless; present |
| panes.rs | components: pane header, divider look, dim of an unfocused pane, focus line | `PanesUI/Panes/PaneHeader.swift`, `PaneDividerView.swift`, `PaneFrame.swift` | **full**: pane header focused text colour and line, outline (focused `faint`, else `border`), divider rest / hot / dragging (`resize.handle.*`), drop zones and the drop hint: present (`PaneHeader`, `PaneDividerView` `DividerLook`, `PaneDropZones`, `PanesText.dropHint`) |
| path_crumb.rs | component: a path as the title row writes it | `DesignSystem/PathCard.swift`, `PanesUI/Title/FolderCrumb.swift` | **full** (wave 8): the crumb's icon and parents take the text colour under the pointer: WRITTEN (`FolderCrumb.swift`, was only under a tint). The name medium in the text colour, the parents giving way first: present |
| popover_layer.rs | layer: GPUI-drawn popover | AppKit popovers (`DesignSystem/PopoverRecord.swift`) | **full**: GPUI-drawn popover; Swift uses AppKit popovers (owner's choice); left |
| popover.rs | components: popover row, heading, rule | SwiftUI content of the native popovers (`PanesUI/Tabs/ProfileMenuItems.swift`, `SidebarUI/Group/GroupPopover.swift`) | **full**: popover row (button with spoken label, hover, the current one semibold), heading, rule. Swift: the popovers' rows are each popover's own SwiftUI (`GroupPopover`, `ProfileMenuItems`) or native menu items (owner's choice for menus): not a shared row; the current item is a checkmark in native menus |
| preview.rs | components: Settings preview frame, surface, stage | `SettingsUI/Preview/PreviewScene.swift`, `PreviewStrip.swift` | **full** (wave 8): the stage's Increase Contrast `card.edge` outline: WRITTEN on `PreviewStrip` (was only on the git scene). `preview_surface` (border under Increase Contrast) and `preview_ring` (solid under it) belong to the motion and sidebar scenes: not compared scene by scene |
| profile.rs | components: profile avatar, chip, edge, choice row | `PanesUI/Tabs/ProfileChip.swift`, `ProfileMenuItems.swift`, `SettingsUI/Controls/ProfileChip.swift` | **full**: avatar, chip (button, spoken, hover `band.hover`), edge, choice row (radio, focus ring). Swift `ProfileChip` hover `band.hover`, spoken label: present; the choice rows are a native menu (owner's choice) |
| reorder.rs | components: drag ghost, lift, split mark, arrange row | `SidebarUI/Drag/*`, `LiftedPicture.swift`, `SettingsUI` arrange list | **full** (wave 8): arrange row hover fill: WRITTEN (`PartsList.swift`). Drag ghost, lift, split mark, drop target and drop line: the sidebar's (`SidebarUI/Drag/*`). GPUI has no keyboard path; Swift adds VoiceOver Move up / Move down. LEFT: the lifted row's 40 % (`DRAG_LIFT_OPACITY`) in the arrange list and the notes' lists: SwiftUI's `onDrag` says nothing when a drag is dropped outside, so a row would stay held back |
| rolling_number.rs | component: digits rolling | `WindowUI/StatusBar.swift` (`statusRoll`) | **full**: digits rolling; Swift `statusRoll` in the status bar and `numericText` in `CountBadge`; present |
| row_keys.rs | component: a setting row's config key (hover, click copies, menu items for VoiceOver) | `SettingsUI/Rows/SettingRow.swift`, `SettingsText.swift` | **full**: the row's menu (Copy Key, Copy as JSON, Reset) IS there (`SettingRow.swift` context menu, the key button's spoken label) — wave 7's "not found" was wrong. LEFT: ⌃↩ on a control opening the row's menu, and `own_menu` (a text field keeps its own menu): the system context menu is all there is |
| rows/mod.rs | doc root of the rows (square, inset 16, lead column) | `DesignSystem/Row.swift` | **full**: `flush_row` (selected fill and text, no hover on the selected) and `lead`: Swift `PanelRow` present |
| rows/card.rs | components: sidebar card stretch, Settings nav row | `SidebarUI/Row/TerminalRowView.swift`, `SettingsUI/Nav/SettingsNav.swift` | **full**: card segments (selected fill/edge, hover), lifted card, folder wash; `nav_row` (listbox option, selected, focus ring, hover, selection fade, selected count badge): Settings' nav is the system's list (owner's choice) |
| rows/commits.rs | components: two-line rows (feature, action, commit) | `PanelUI/Shell/TwoLineRow.swift`, `History/` | **full**: two-line row hover; commit row button with expanded (selected fill, subject semibold in the tint's text, hash in the primary or the tint's text): Swift `CommitRowView` present (expanded as a value) |
| rows/detail.rs | component: second line rolling | `SidebarUI/Row/SecondLineView.swift` | **full**: second line rolling (crossfade, stops under Reduce Motion): present |
| rows/files.rs | components: Changes file row, folder row | `PanelUI/Changes/`, `Files/` | **full**: file row (selected muted colours, name semibold, renamed colour), folder row: Swift `FileRowView` present (marked rows too). LEFT: the open-in-editor button's `spoken` name (GPUI core `spoken::open_file`: the tooltip and the file name) needs a bridge function and a `PanelHost` seam; the shared button takes `spoken` now |
| rows/session.rs | components: terminal row and tab, name fade | `SidebarUI/Row/TerminalRowView.swift`, `PanesUI/Tabs/TerminalTabView.swift`, `NameText.swift` | **full**: terminal row: selected (name semibold in the tint's text, tone colours on the tint), hover (none on the selected), name fade under the pointer, keycap: `TerminalRowView`/`RowLines` present |
| scale.rs | scale: sizes, type roles, borders | `DesignSystem/Scale/*` | **full**: not a component (sizes, roles, borders); the radius table's rule (chip-shaped = `radius_chip`) now holds for `CountBadge` |
| section.rs | component: item row with hover actions and grip, section state | `PanelUI/Notes/NoteRowView.swift`, `TaskRowView.swift`, `RowActions.swift` | **full**: item row hover, active (being edited) fill, hover actions, lifted opacity while dragged; Swift `TaskRowView`/`NoteRowView` present but the lift (see reorder.rs) |
| setting_group.rs | components: group head, blurb, group card | `SettingsUI/Pages/PageScaffold.swift`, `Rows/` | **full**: group head, blurb, card with Increase Contrast `card.edge`: present (`SettingGroup.swift`) |
| setting.rs | components: stepper, toggle, setting row states (changed, project, disabled), reset arrow, link button, slider control, shortcut recorder | `SettingsUI/Rows/SettingRow.swift`, `Controls/ValueStepper.swift`, `SettingToggle.swift`, `ActionButton.swift` (`LinkButton`), `SettingSlider.swift`, `ShortcutRecorder.swift` | **full**: row states (changed dot, project tint chip, disabled faint, out of reach), reset arrow (shared icon button), stepper (`control.hover`, Increase Contrast), recorder: present; toggle, link button, slider: the system's (owner's choice) |
| size_readout.rs | component: size chip | `TerminalUI/Scaling/SizeReadoutChip.swift` | **full**: tooltip colours, opacity from the caller (cut under Reduce Motion): present |
| slider.rs | component: slider (disabled at half opacity) | `SettingsUI/Controls/SettingSlider.swift` (system) | **full**: the system's slider (owner's choice); disabled at half opacity is the system's look; left |
| splits.rs | components: chain mark, split line | `SidebarUI/Row/SplitLine.swift`, `StudioUI` `DropsSpecimen` | **full**: chain mark with its tooltip, split line: present (`RowMarks`, `SplitLine`) |
| status_shift.rs | motion: a status change's clock and poses | `SidebarUI/Tile/StatusPlay.swift` | **full**: a status change's clock and poses; shown only when the variant has one, not under Reduce Motion: `StatusPlay` present |
| status.rs | components: status glyph, agent logo, status bar items, attention, separator | `DesignSystem/StatusDot.swift`, `WindowUI/StatusBar.swift` | **full** (wave 8): the needs-you button's hover is the attention ring at 30 % on chip corners (was `band.hover` on control corners): WRITTEN (`StatusBar.swift`). Status glyph still under Reduce Motion, agent logo, items with hover and ring, separator: present |
| submodule.rs | component: submodule row | `PanelUI/Changes/SubmoduleRowView.swift` | **full**: submodule row (selected colours, enter button with tooltip): present; the enter button is now the shared icon button (through `BandButton`) |
| tab_selection.rs | component: the selection shape springing tab to tab | `WindowUI/PanelTabBar.swift` | **full**: the selection springs tab to tab, none under Reduce Motion: present |
| tabs.rs | components: terminal tab, tab divider, group switcher | `PanesUI/Tabs/*` | **full** (wave 8): the ⌘N hint is the shared `KeyCap` (`control.background`, `text.primary`; was `border`/`textMuted`, GPUI `keycap`). Tab hover (none on the selected), close on hover, name fade, group switcher hover `band.hover`: present |
| text_field.rs | component: flat text field (+ multiline) | `DesignSystem/FlatField.swift` (WRITTEN, gallery), Settings keeps the system's rounded field (owner), `PanelField`/`PaletteField` (panel draws its own), `PanelUI/Diff/CommentEditor.swift` and `SettingsUI/Panels/ProfileControl.swift` multiline wells | **full**: flat field (well, hover, ring, Increase Contrast edge, prefix, clear, disabled): `FlatField`, gallery only, left on the owner's decision whether a drawn field replaces system fields; multiline: none |
| text_scale.rs | scale: Settings/Studio text size step | `SettingsUI/SettingsMetric.swift`, `ThemeSync.swift` | **full**: not a component (text size step); present |
| tile/mod.rs | component: the status tile | `SidebarUI/Tile/StatusTile.swift`, `PanesUI/Panes/TerminalTile.swift` | **full**: light and edge stand still under Reduce Motion, a status change's play not under it: present |
| tile/badge.rs | component: tile badge (needs input, done, failed, bell, unread) | `SidebarUI/Tile/StatusTile.swift` | **full**: needs input, done, failed, bell, unread: present |
| tile/edge.rs | motion: amber breathing edge | `SidebarUI/Tile/TileLight.swift` | **full**: amber breathing edge, still under Reduce Motion: present |
| tile/face.rs | helper: face resolved against the agent registry | `SidebarUI/Tile/` + core agent registry | **full**: not a component (face from the agent registry) |
| tile/orbit.rs | motion: the working light | `SidebarUI/Tile/TileLight.swift` | **full**: working light, a ring under Reduce Motion: present |
| tile/ring.rs | helper: light and edge geometry | `SidebarUI/Tile/TileLight.swift` | **full**: not a component (geometry) |
| tile/transition.rs | motion: tile during a status change | `SidebarUI/Tile/StatusPlay.swift` | **full**: as status_shift.rs: present |
| tint.rs | helper: tint across a bar's left | `WindowUI/TitleTintLine.swift` | **full**: not a component (a tint across a bar's left); `TitleTintLine` present |
| title_drag.rs | helper: title bar of an app-drawn window | system title bar / `DesignSystem/SecondaryWindow.swift` | **full**: not a component (the system drags the title bar) |
| window_sketch.rs | component: window sketch for the first run | `StudioUI/Gallery/SketchSpecimen.swift`, `AuxBridge/TailorBridge.swift` | **full**: Increase Contrast border, the sketch's fills from the theme: `WindowSketch` present (outline under Increase Contrast) |
| window_surfaces.rs | layer: translucent or opaque surfaces for app-drawn windows | `DesignSystem/SecondaryWindow.swift`, `Theme/SystemLook.swift` | **full**: opaque surfaces under Reduce Transparency: `SecondaryWindow`/`SystemLook` present |

Count: 81 table lines for the 81 files of `crates/wormhole/src/style/` (checked with `find ... -type f | wc -l`).

#### X05: what GPUI's Reduce Motion cuts, against Swift's consumers

GPUI's consumers (`grep reduce_motion`, non-test): menu bar preview and status item animation, hotkey preview, Settings motion preview, sidebar example rows, sidebar status motion, the row plan, a room joining, the intro, the tab selection spring, the status glyph and tile light/edge, the notice chip's entrance, the cursor and text blink, the overlay scrollbar reveal, the zoom and size readout, the window slide, `plays_motion`, the welcome answers' handover. Swift has a reader for each of them (blink: `TerminalMetal/CursorBlink.swift`, `TextBlink.swift`; scrollbar `TerminalScroller.swift`; readout; notice timing; intro `Handover.swift`, `IntroStyle.swift`; slide `WindowMotion.swift`; status item `StatusItemController.swift`; tab spring `PanelTabBar.swift`; tile `StatusTile.swift`, `TileLight.swift`, `StatusPlay.swift`). What Swift still animated with no reader, now written: the group chevron and hover, the row's pin pop-in and state/hint animations, the second line's roll, the drag-drop settle spring, the title tab alert dot and the tile corner dot (scale), the context meter, the review bar and the pane bar sliding in, the diff review and folder colour folds. NOT written: scroll-to animations in Settings and the Studio (`withAnimation(Motion.layout)`, three places), the panel and Studio opacity-only transitions (a fade is what Reduce Motion asks for).

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X05 | System Settings, Reduce Motion on; fold a group, pin a terminal, change a terminal's status under Line rolls, drop a row, open a review | no rotation, scale, roll, slide or spring: a short fade or nothing | `SidebarUI/Group/HeadControls.swift`, `Row/RowLines.swift`, `Row/SecondLineView.swift`, `Drag/SidebarView+Drag.swift`, `PanesUI/Panes/TerminalTile.swift`, `PanesUI/Tabs/GroupSwitcherButton.swift`, `PanesUI/Title/GaugeChip.swift`, `PanelUI/Diff/ReviewBar.swift`, `TerminalUI/Pane/PaneOverlays.swift` |
| X01 | any chip: the font tag, the row's Beta tag, a port or pull request chip, the branch chip, a commit's branch ref | looks exactly as before; Increase Contrast outlines a filled one; the Studio's Chips specimen shows rest, selected, outlined, disabled | `DesignSystem/Chip.swift` and the five callers |
| X01 | the right panel's tab with a count, selected and not | one badge type; selected is the stronger tint | `DesignSystem/CountBadge.swift`, `WindowUI/PanelTabBar.swift` |
| X01 | Tab to a sidebar row, a group head, a panel row, the title row's status; then put the window behind | the ring is the theme's focus colour and width; it is gone while the window is behind; it follows the theme under Increase Contrast | `DesignSystem/FocusRing.swift` (`keyboardFocusRing`), `SidebarUI/Group/GroupHeadView.swift`, `Row/TerminalRowView.swift`, `PanelUI/Shell/WalkRing.swift`, `WindowUI/Focus/FocusRing.swift` |
| X01 | the review comment box and the profile's environment box: click in, then disable | soft primary ring while focused (solid under Increase Contrast); held back when disabled | `DesignSystem/FocusRing.swift` (`flatFocusRing`), `PanelUI/Diff/CommentEditor.swift`, `SettingsUI/Panels/ProfileControl.swift` |

## Lane platform, wave 7

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|-----|-----------|------------------|-------------------|
| PL03 | Post a needs-input banner for the selected terminal while the app is in the background, then click the Dock icon (or Cmd-Tab back) | the banner goes at once; a banner for a terminal that is not selected in the key window stays | `Notices/NoticeFeed.swift` (`lookOnFocus`: didBecomeActive, didBecomeKey; the core's `Seen` through `observe`) |
| PL03 | Two windows in two profiles with different `notify_*` settings; finish a command in each | each banner follows its own profile's settings (not the shown profile's) | `NoticeFeed.swift` `noticeTabs` (`tab.settings`), `NoticeMapping.swift` |
| PL01/O01/O03 | TEST: `profile-new Work`, open a window in Work, open a folder there, Open Recent in each window; Cmd-J in each | each window lists its own profile's recents; the folder picker's Recent rows likewise; GPUI's rows `recent_projects` and `recent_folders` in each profile's database are the same ones | `Menus/RecentProjects.swift`, `Platform/FolderPicking.swift`, `crates/wormhole-ffi/src/recent_projects.rs`, `folder_picker/mod.rs` |
| A05/A07 | TEST: two windows in two profiles, each with a waiting agent question; edit profile B's `config.json` `agent_access` to `off` | B's question is refused, A's still waits (also a Rust test: `hook_server/tests/profiles.rs`) | `Settings/ProfileLook.swift` (`settingsChanged(profile:)`) |
| A01 | TEST: first run, pick a folder with a `.wormhole/` folder (a subfolder of it too), go to the agents step, press Trust | the card is for the folder's project, the button reads "Waiting for approval..." while Touch ID is up, the card goes after approval; cancel brings the button back; no folder picked and no front project: no card; asked as the window's own profile | `AuxBridge/OnboardingBridge.swift`, `AuxModel/Onboarding/OnboardingSession.swift`, `AgentsStep.swift`, test `testTheTrustCardWaitsWhileApprovingAndGoesOnceTheProjectIsTrusted` |
| PL01 | Open the Theme Studio, make an edit, look at Edit | the menu says "Undo Change ..." / "Redo ..." while the Studio is the key window and plain "Undo" in a workspace window | `Menus/MenuController.swift`, `Platform/StudioLauncher.swift` (`editTitles`), `StudioWindowController.windowDidBecomeKey` |
| O01 | Open the palette (Cmd-Shift-P) in a repo, type `>` commits scope / search a subject; checkout another branch in the History tab first | the commits are HEAD's log of the terminal's folder (not the History tab's branch or path filter); they appear a moment after opening; History off: none | `Platform/PaletteCommits.swift`, `PaletteSourcing.swift` (`opened`), `PaletteView.onAppear` |
| PL01 | Split two panes, open the Notes tab, edit a note, press Option-Cmd-Up / Down | the note moves (the Select Pane items are greyed while a list or note editor holds the keyboard); in the terminal the same keys select the pane | `WindowUI/Hub/WindowHub+Menu.swift` |
| A04 | A host that runs and answers refused / too old: open Session Hosts | the card shows the error's detail line | `WorkspaceBridge/HostConnect.swift`, `HostBook.swift`, `RustWorkspaceBackend+HostProblems.swift` (`unavailableDetail`), `AuxBackends.hostAttachments` |
| A03 | Check for Updates... in a build with no feed | Settings opens on Updates and says "This is a development build; it doesn't update itself." | `Aux/AuxController.swift` (`showUpdates`), `swift-aux.ftl` `update-no-feed` |
| PL02 | Settings window key: press Cmd-Shift-N, Cmd-1 | they run as in the front workspace window (a quick panel still reads its own keys) | `Actions/ExtraKeys.swift` |
| O05 | Clear Buffer (Cmd-K) in a terminal of a host older than protocol minor 2 | NOT A GAP: `clearHistory()` returns true there and `onHistoryKept` posts the notice; nothing new written. Needs an old host to see it | `TerminalMetal/Pointer/TerminalSelection.swift:34`, `TerminalOverlay+Clipboard.swift:23`, `interaction/pointer.rs` `clear_history` |
| PL03 | NOT WRITTEN: a dev build without a bundle posting through the platform's script | - | dev only |
| PL01 | NOT WRITTEN: Show Project greyed when project folders are off | - | the action is hidden by `featuresOff` in that case; the difference is not visible |

## Lane settings-studio, wave 7

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| ST03 | TEST. Settings > Notifications with macOS "not asked": press Allow notifications, answer in the system prompt | the row leaves "Not asked yet" as soon as the answer is given (no reopening of the page); no polling | `Notices/NoticeFeed+Test.swift` `askPermission`, `Settings/SettingsPlatform.swift` |
| ST03 | In a bundled app (not a test instance) press Send test with the permission denied by the system's add (or force an error) | the row says "macOS refused the notification: <the system's sentence>"; allowed: "Sent" only after the system took the request | `NoticeFeed+Test.swift` `finishTest`, `SystemNotifier.deliver`, `Notifying.deliver`, `NoticeCoordinator.deliverTest` |
| ST03 hardening | TEST. `cargo test -p wormhole-ffi agent_logins` | a login folder under `WORMHOLE_AGENT_HOME` is written; `<home>/../real/.claude` and `<home>/link/.claude` (link out) are refused and nothing is written outside | `crates/wormhole-ffi/src/settings/agent_logins.rs` `resolved_within`, `tests` |
| SD01 | Studio: select a metric (`metrics.radius`) | under the stepper a "Preview at N px" label, "n uses", and a sample row (count badge, key cap, chip, status badge) drawn with the draft | `StudioUI/Editor/MetricEditor.swift` `preview` |
| SD01 | Studio: type `mix(pri` in a colour's field | a list with `primary...` colours and `mix(a, b, 30%)` functions marked "function"; Up/Down move, Tab/Return take the row and the caret goes after it, Esc closes the list before it closes the window; a finished name offers nothing | `StudioUI/Editor/ExpressionField.swift`, `StudioBridge/FfiStudioFiles.swift` `completions`, `studio_edit.rs` |
| SD01 | Make `text.muted` too dark in a theme | the card lists the pair with core's fix (`readable(<what you typed>, ...)`); a token that only follows a failing one says "Follows X: its fix clears this too" | `StudioDocument+List.swift` `contrastIssues`, `LowContrastCard.swift`, `studio_edit.rs` |
| SD01 | Open a pair (a theme with Light and Dark) | every token row has the appearance chip (both / light / dark); a click moves the value | `StudioUI/List/TokenRowView.swift`, `TokenListView.swift` |
| SD01 | TEST. Save a theme twice in the Studio; Browse All Versions; `swift test --filter ThemeFilesFFITests` | the first text is kept as a version (a hand-written first file as the pinned original); the folder, caps and thinning are core's | `StudioBridge/FfiThemeFolder.swift`, `studio_folder.rs`, `ThemeFilesFFITests` |
| SD02 | Studio gallery > Tiles | a flare and a nudge are drawn a third of the way through working to needs input; the other variants play on press; counts still cannot be held | `SidebarUI/Samples/TileStatesSample.swift`, `Tile/StatusPlay.swift` `statusCaught` |
| SD02 | Studio gallery: scroll | specimens are built as they come near the viewport (LazyVStack); selection outline still follows | `StudioUI/Gallery/GalleryView.swift` |
| ST02 | Reduce Transparency or Increase Contrast on; Appearance page | the opacity slider is off and its value faint, with the note | `SettingsUI/Controls/SettingSlider.swift` `offWhenOpaque` |
| ST02 | Terminal page > Default shell | the menu shows each choice's command line under its name ("Login shell: /bin/zsh"); the button shows the name | `SettingsUI/Controls/SubtitledPicker.swift`, `host.rs` `shell_options` |
| ST02/ST09 | Fonts page: open the Code font picker while the Terminal tab is shown, hover a family | the specimen switches to the Code tab and shows that family at the code size | `SettingsUI/Preview/FontsScene.swift`, `Rows/FontControl.swift`, `SettingsModel/FontPreview.swift` |
| STX2 | Project folders page with a project selected: edit `<project>/.wormhole/config.json` by hand with the window in front | the page reads again within about a second, without the window coming forward | `project_watch.rs`, `ProjectsModel.select`, `RustProjectsBackend.follow` |
| SD01 | Open the Studio, switch profile (chip) with it open, then Save as / list themes | the Studio's theme list, versions and saves use the new profile's themes folder (the open draft's own file stays where it was read) | `StudioLauncher.swift` `live`, `FfiThemeFolder(folder:)`, `FfiStudioFiles(folder:)` |
| ST03 | NOT DONE: the permission row following a change made in System Settings while the page is up | nothing arrives (the notification centre has no change event); the page reads on open | - |

## Lane sidebar-window, wave 7

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| W05 | Two profiles, a window of profile B in front, Profile chip or File menu: Open Window in A, then in B; then ⌘N | each opens a window of the profile picked (the chip says it); ⌘N opens one in the front window's profile | `Settings/ProfileChipBinding.swift openWindow(inProfile:)` (plans on `hostBeside(id)`, the shown profile's host included: `profileHosts` holds it), `AppController+WindowOpening.swift openNewWindow` |
| W06 TEST | A window of profile A with a trusted project; Move Window to B; open Project commands for it | the project reads untrusted (B never approved it); moving it back reads trusted again | `ffi/src/files_panel/project/service.rs follow_dir`, test `trust_follows_a_window_to_the_profile_it_moved_to` |
| W06 | Move Window to B with persistent terminals on, `new` in it | a host row in B's database (wave 6 test passed); Access question for B's agents goes to a B window; Project commands window names B's approvals | `ProfileChipBinding.swift move` (already re-seats the hosted desk, project, usage, files, worktrees; access desks are per profile and judged by the window's own profile; Imports is Settings', which follows the shown profile as GPUI's does) |
| W12 | A window of profile B (other theme than A), start a terminal whose program asks OSC 11 at once (`printf '\e]11;?\a'; read`) | it is answered with B's background, not A's | `TerminalFactory.make` (`setEngineColors` before the view exists), `TerminalCore/Palette.swift engineSlots` |
| W01 | Motion feature on, Reduce Motion off: ⌘D in a lone terminal, ⇧⌘D in a split's pane; `wh`/devctl `perf` or a PTY resize log while it plays | the new pane slides in from the far end over ~0.4 s on the core's spring, the old pane is cut back to what the edge leaves; each terminal's grid size changes at most once (at the end; the lone pane once more at the start when its header appears); no divider until it ends | `PanesUI/Panes/PaneArea.swift` (`placement`, `held`, `TimelineView`), `SplitOpeningTracker.swift`, `PanesModel/Geometry/SplitOpening.swift`, `WorkspaceBridge/CoreSplitMotion.swift`, `ffi/src/split_opening.rs`, core `Opening::of_rects` |
| W01 | The same with Reduce Motion on, and with the Motion feature off | the split lands at once (a cut), no spring, no fade | `PaneArea.swift` (`.animation(playing != nil || change.isCut ? nil : …)`) |
| W01 | Click the new pane's header and type while the motion plays; press ⌘D twice quickly | the new pane takes the keys; a second split during the first draws the layout as it is (a cut), nothing stuck half-open | `SplitOpeningTracker.observe`, `PaneArea.stillPlaying` |
| S03 TEST | Group "g" with no folder: press its +, delete the group before answering the question; also rename it | the terminal opens loose (in no group); the answer is not applied to a group of another name | `SidebarUI/Group/GroupFolderQuestion.swift openTerminal` (three callers) |
| S09 | ⌃⌥] / ⌃⌥[ / ⌃⌥1–9 and the switcher's search, in the tab bar layout | as before: empty sets passed over, Ungrouped last, search finds sets then terminals | `PanesModel/Tabs/TabSets.swift` (rules installed by `CoreWindowRules.install`), `WorkspaceBridge/CoreTabSetRules.swift`, `ffi/src/tab_sets.rs`, test `TabSetsFFITests` |
| W03 | Status bar on with terminals needing input, working, done, failed; narrow the window under 1100 and 720 pt | the button is the longest-waiting one; counts in the bar's order; words go, then the size | `WindowUI/StatusSummary.swift`, `WorkspaceBridge/CoreStatusRules.swift`, `ffi/src/status_counts.rs`, test `StatusRulesFFITests` |
| W12 | NOT DONE: a B terminal that has no view yet still starts with the shown profile's colours and hears its theme changes | - | the engine's look is one global (`ffi/src/engine_look.rs`, read by `launch.rs`); per profile it needs the profile at `Terminal::from_parts` (three start paths) |
| S08 | NOT DONE, needs a real Mac: whether the group card animates the insert pose | - | `SidebarUI/Group/GroupRows.swift:45` |

## Lane style, wave 8

Written without compiling: expect compile errors.

X01 against audit 5's list; the 81-line table of `crates/wormhole/src/style/` above (wave 7, continued) is corrected in place:
every line is now a full read of the GPUI file's states, parts and behaviours against its Swift counterpart.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| X01 | a terminal in a repository on a branch with a short name, the title row wide | the branch chip's wash fills its planned width (as before wave 7), the text at its start | `DesignSystem/Chip.swift` (`width`), `PanesUI/Title/BranchCrumb.swift` |
| X01 | the right panel's Changes tab with files, selected and not; a panel band with a count; a folded group in the sidebar | one count badge, chip corners (not a capsule) everywhere; selected is the stronger tint | `DesignSystem/CountBadge.swift`, `PanelUI/Shell/CardBand.swift`, `PanelUI/Diff/SubmoduleCard.swift`, `SidebarUI/Group/FoldedSummary.swift` |
| X01 | hold ⌘ with terminals shown as tabs | each tab's ⌘N hint is the shared key cap (`control.background`, `text.primary`) | `PanesUI/Tabs/TerminalTabView.swift` |
| X05 | Reduce Motion on; fold a group, fold a section label; pick another row in a `Row` list (Studio tokens, font picker) or another segment (diff Split/Unified) | the chevron swaps at once (no turn); the selection fades in place (no slide) | `DesignSystem/Motion.swift` (`motionTurn`, `slidingSelection`), `SidebarUI/Group/HeadControls.swift`, `SectionLabel.swift`, `Row.swift`, `Segmented.swift` |
| X03 | VoiceOver on: the find bar's Previous / Next, the title row's Split right / Split down, the sidebar and panel toggles, a band's + whose tooltip names a key (New note) | the name is the words alone ("Next match"), the key follows as the hint in words ("Shift Command G"); the tooltip still shows the key; "Close find (esc)" keeps its words, as GPUI's rule | `DesignSystem/SpokenName.swift`, `IconButton.swift`; test `Tests/UITests/SpokenNameTests.swift`, scenario `Tests/EndToEndTests/SpokenNameTests.swift` |
| X01 | hover the find bar's steps and close, then its three options; turn one option on | `band.hover` and `text.primary` under the pointer; an option that is on wears the selected tint and its text; VoiceOver hears a toggle | `TerminalUI/Find/FindBar.swift`, `FindToggle.swift` |
| X01 | the title row's Split right / down (one greyed where there is no room), Open in…, the sidebar and panel toggles, the sidebar's +; Tab to the title row; make the panel wait for room | 24 pt faces on 28 pt hits, `band.hover`, faint when there is no room; the ring round the face where the keys are; the waiting dot on the panel toggle; the column glyph fills its strip | `WindowUI/TitleButton.swift`, `PanesUI/Title/SplitButtons.swift`, `OpenInButtons.swift`, `DesignSystem/IconButton.swift` |
| X01 | a panel band's +, a note's send and delete, a file row's open-in-editor, the diff header's buttons; turn the diff's wrap on | 22 pt faces on rows, 24 in bands, `band.hover`; wrap on wears the selected tint (was 60 % opacity off) | `PanelUI/Shell/BandButton.swift`, `PanelUI/Diff/DiffHeader.swift` |
| X01 | History filtered to a file (from a file's menu) | the filter chip is filled with `chip.selected` and its text, chip corners, no hover change | `PanelUI/History/HistoryView.swift` |
| X01 | scroll the sidebar's list, then each panel tab's list (Changes, History, Notes, Project, Usage, Files) | a hairline under the top while rows are scrolled beneath it; none at rest; none with Borders off | `DesignSystem/ScrollEdge.swift`, `SidebarUI/SidebarView.swift`, `PanelUI/Shell/PanelCard.swift` (`PanelScroll`) |
| X01 | a `Row` list (Studio tokens, font picker, onboarding steps): hover the selected row, then another | at least 28 pt tall; no grey over the selection; the selected row's text in the selection's colour | `DesignSystem/Row.swift` |
| X01 | the diff's Split / Unified: hover the segment that is not chosen | its text turns `text.primary` | `DesignSystem/Segmented.swift` |
| X01 | hover a group head with terminals, and a pinned empty group | the + is a 22 pt face on a 28 pt hit with `band.hover`, turns a quarter on a click | `SidebarUI/Group/HeadControls.swift` |
| X01 | hover the title row's folder | its icon and parents take the text colour | `PanesUI/Title/FolderCrumb.swift` |
| X03 | Settings: a row with an (i); hover it, open it, VoiceOver to it | hover fill, text colour while hovered or open; VoiceOver reaches it as "More information, button" | `SettingsUI/Controls/RowAccessories.swift` |
| X05 | Settings with Increase Contrast on: any page with a preview | the stage has its `card.edge` outline | `SettingsUI/Preview/PreviewStrip.swift` |
| X01 | Settings → Appearance → Title row: hover a part | the row takes the hover fill | `SettingsUI/Rows/PartsList.swift` |
| X01 | a terminal needs input with the status bar on: hover its button | the attention ring at 30 % on chip corners (was `band.hover`); the keyboard ring has the chip's corners | `WindowUI/StatusBar.swift`, `ChromeColours.swift` |
| X01 | Studio gallery, Chrome | icon buttons: a title row face, a row face, a toggled option, a filter with its dot, one with the keyboard ring | `StudioUI/Gallery/ChromeSpecimen.swift` |

Not written, and why:
- The open-in-editor button's `spoken` name on file rows (GPUI core `spoken::open_file`: tooltip and file name): needs an FFI function and a `PanelHost` seam; the shared button takes `spoken` now.
- The panel task box as a checkbox role: `accessibilityRepresentation` would do it, but retires `notes-task-not-done` from use (a catalog change).
- The lift opacity of a carried row (notes, arrange list): SwiftUI's `onDrag` never says when a drag is dropped outside, so the row would stay held back.
- `IconButton`'s `narrow` + `menu_chevron`: the two split menus (Open in…, the Studio's Save) draw their own `Menu`.
- `Chip`'s hover: GPUI's only hovering chip is an inactive `pill`, which no screen of GPUI shows (gallery only). `FlatField`: left on the owner's decision.
- Seven hand-drawn keyboard rings (settings, studio, onboarding card, find field, sidebar `ringWhileMenuOpen`) are still not on `keyboardFocusRing`.

## Lane studio, wave 8

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| SD01 | Open the Studio, set `text.muted` to `#333847` on a theme over Nord; look at the row, the Low segment, the editor's Contrast section and the card | the row's warning mark, the Low count and filter, the section's failing line and the card's pairs all show the same pairs (one core report, asked once per change); with Increase Contrast on they follow the high-contrast variant | `studio_edit.rs studio_contrast`, core `ThemeTokens::contrast_checks`, `StudioDocument.refreshContrast`, `ContrastCheck.swift` (`ContrastReport`); tests `the_failing_checks_are_the_warnings_and_the_issues`, `testCoreOffersFunctions…AndFixesAPairThatFallsShort`, `ContrastTests` |
| SD01 | Select a metric (`metrics.radius_chip`) and a colour (`border`) | "Used by" and the metric's "n uses" are core's `TOKEN_USES` (dialog and tooltip listed); the gallery outlines what the Swift views draw (the neutral, tint and branch chips and commit refs for `radius_chip` again) | `studio_token_uses`, `FfiTokenUses.swift`, `GalleryKnowledge(uses:)`, `TokenKnowledge.drawn`, `GalleryView` `drawn(with:)`, `gen-gallery-uses.py` (the shared `Chip` rule); test `testUsedByIsCoresTableAndTheOutlinesAreWhatTheViewsDraw` |
| SD01 TEST | Open your own theme of profile A in the Studio, edit, switch to profile B, Save | the file in A is written; nothing (not even `.versions`) appears in B's themes folder; Versions lists the kept original | `FfiThemeFolder.home(of:)`, `FfiStudioFiles.newID` (looks in the file's own folder); test `testADraftOpenBeforeAProfileSwitchIsNeverSavedIntoTheOtherProfilesFolder` |
| SD01 TEST | A theme file with no `id` (older format, or version 1 without one): rename it in the Studio and save; open Versions | the earlier saves are listed (the pinned original too); the file now has the `id` it answered to before the rename. Same in the GPUI Studio | core `saved_versions::{file_id, version_id, save_id}`, GPUI `themes.rs`, `convert.rs file_to_write`, FFI `studio_new_id`, `studio_folder.rs version_id`, Swift `StudioDocument+Save.swift writeOwn`; tests `a_renamed_theme_without_an_id_keeps_its_earlier_saves` (core), `a_renamed_theme_without_an_id_still_lists_its_versions` (FFI); end-to-end `studio()` |
| SD01 | Nothing to do: a Studio folder call from a background thread | waits for the main thread instead of trapping | `StudioLauncher.swift` `live` |
| SD02 | Gallery > Status tiles | under the flare and nudge held a third of the way, cross-fade, badge pop, wash and line roll each shown before and after with a caption; under the count badges 12→13, 19→20, 20→19 before and after with a caption | `SidebarUI/Samples/TileStatesSample.swift beforeAfter`, `StudioUI/Gallery/TilesSpecimen.swift countsBeforeAfter` |
| X12 | `studio`, then `state`; `studio-frames 40 tick`, `studio-frames 40 scroll`, `studio-frames 40`; `studio-scroll 600` twice | `state` ends with `studio dirty= asking= subject= undo= redo= format= shown= update= modal= inuse= token= origin= reach=`; each frames line is `studio-frames <mode> n=40 avg= max= first=` (also in `frames.txt`); the second scroll replies `at=` the same offset as the first (an offset, not a step) | `Driving/AppController+ControlStudioState.swift`, `Driving/AppController+ControlStudioFamily.swift` |
| SD02 | NOT DONE by the owner's decision: tooltip and dialog specimens marked `tooltip` / `dialog` | macOS draws both; "Used by" still names them (core's table), the gallery outlines nothing for them | `SystemSpecimen.swift` |
| X12 | NOT DONE: `modal=versions` in the studio `state` line | the Versions sheet is the window's own `@State` (`StudioView.showsVersions`), not seen from the control folder | - |

## Lane platform-settings, wave 8

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| ST03 | Real bundle: Settings > Notifications, Open System Settings, turn Wormhole's notifications off, come back to Wormhole | the row says off at once (no restart); turning them on again likewise; also when only the Settings window becomes key | `Wormhole/Notices/NoticeFeed.swift` `lookOnFocus`, `NoticeFeed+Test.swift` `rereadPermission`, `SystemNotifier.readPermission` |
| ST03 | Real bundle: notifications turned off in System Settings while Wormhole runs, then Send test | the row says denied, never "Sent" | `NoticeFeed+Test.swift` `runTest` (reads the permission before it decides) |
| A05/A07 TEST | Two profiles; a terminal of A runs an agent with access Ask and a question waiting; switch to B; edit A's `config.json` `agent_access` to `off` by hand | the waiting question is refused and A's streams under approvals end at once, without coming back to A; a change of another key settles nothing; `swift test --filter ParkedAccessTests` | `Wormhole/Settings/ParkedAccess.swift`, `ProfileChipBinding.swift` (`followParkedAccess` after a switch and an end), `hook_server/core.rs` `settings_changed` |
| A01 | First run with a project folder that has commands: approve the project in another window's Project tab (or by the trust window) while the intro shows its card | the card goes within about a second | `AuxBridge/OnboardingBridge.swift` `followTrust`, `aux_backends/trust.rs` `follow` (now also the profile's `trusted-projects.json`) |
| A01 | The trust window (Project commands) open; trust or revoke the same project from Settings > Project folders | the window reads again (it follows the approvals file too now) | `aux_backends/trust.rs` `follow` |
| PL01 | Feature flag project folders off; palette, ⌃⌘5 | Show Project is neither offered nor run (hidden and greyed, as GPUI's) | `Wormhole/Menus/RecentProjects.swift` `menuState(for:)` |
| PL01 | A sheet up on a workspace window (close question, new worktree, a Settings sheet): press ⌃⌥2, ⇧⌘N, ⌘= | nothing runs behind the sheet; with no sheet they run as before, also from Settings and the Studio | `Wormhole/Actions/ExtraKeys.swift` |
| STX2 | Settings > Project folders open (any project or the defaults); trust or revoke a project from another window's Project tab, the trust window or `Wormhole` on the command line, with the Settings window in front | the trusted list and the selected project's standing change within about a second | `files_panel/project/settings_page.rs` `watch_trust` (and `trusted_projects`/`project_page` read the file fresh), `settings/project_watch.rs` `watch_approvals`, `RustProjectsBackend.followTrust`, `ProjectsModel.followTrust`, `ProjectsPanel` `.task` |
| O04 | Menu bar, a terminal with `/bin/cat` (no shell integration), Edit | Previous Prompt, Next Prompt, Select Output, Copy Output, Copy Command, Rerun Command greyed; with a shell that has integration, enabled after its first prompt; `swift test --filter EndToEndTests/blockItemsFollowShellIntegration` | `WindowUI/Menu/MenuStateMaker.swift`, `TerminalOverlay+Folds.swift` `syncFolds`, `Tests/EndToEndTests/MenuBarTests.swift` |
| O04 | Persistent terminals on in the shown profile, an odd window of another profile (flag off there) in front | Wormhole > Quit and End All Terminals and Session Hosts follows the shown profile's switch, as GPUI's | `Wormhole/Menus/RecentProjects.swift` (`state.persistent`) |
| PL04 | Record a new global hotkey and a new jump key in Settings, then open the status item's menu without any terminal changing | Show Wormhole and Jump to Next Waiting show the new keys at once | `Ambient/AmbientController+Settings.swift`, `Platform/JumpKey.swift`, `AmbientController.pushStatus` |
| ST03 TEST | `WORMHOLE_AGENT_HOME=<home>/../<home dir name>` (or a link to the home) | Settings > Coding agents refuses to write there, as for the home itself | `settings/agents.rs` `is_real_home` |
| ST03 | NOT DONE, the owner's decision: a symbolic link as the final `settings.json` of an agent login folder | today the core writes through it (`agents/integrations/files.rs` `write_text`, `LinkPolicy::Follow`, on purpose for dotfiles setups), in both apps | see the report: refuse, follow or ask |

## Lane window-hosting, wave 8

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| T20 TEST | `cargo test -p wormhole-ffi ending_at_quit_ends_a_host_still_starting` | a host still starting when the desk quits with End all is ended too (none left), no row names it; a desk made after it in the same process starts and keeps a host as any | `ffi/src/hosting/quit.rs` (`QuitGate`), `desk.rs` (`start`/`attach` counted, `taken` lets go or ends by the gate's `fate`, `finish` waits) |
| T20 TEST | `swift test --filter HostingFFITests` and `--filter HostedWindowsTests`, then `pgrep -fl wormhole-host` | both pass with no `HostConnect.launched()`; after the run no `wormhole-host` of their `/tmp/Wormhole-swift-host*` folders runs (each tear-down ends every host it saw and any other live in its folder, by id through `HostedTerminals.endHost`, and asserts none is left) | `Tests/WorkspaceBridgeTests/HostingFFITests.swift`, `Tests/WindowsBridgeTests/HostedWindowsTests.swift` |
| T20 | Persistent terminals on: ⌘T then at once Quit and End All Terminals | no `wormhole-host` of the data folder is left; the quit may take up to a few seconds longer while a host is still starting | `RustWorkspaceBackend+Hosting.swift finishQuit` (no process-wide flag any more), `HostConnect.swift`, ffi `HostedTerminals.finish` |
| T20 TEST | `cargo build -p wormhole-host` into `crates/wormhole-ffi/target`, then `swift test --filter EndToEndTests/hostedTerminalOutlivesTheApp`, `/endAllLeavesNoHost`, `/hostsListKillAndDeath` | a hosted `cat` comes back attached to the same host with `remember-me` after a quit that keeps hosts; End all leaves no live host in the scratch `hosts/`; `hosts-list` names both hosts, `host-kill` ends one, a SIGKILLed host turns its terminal `lost` and `pane-bar` says `exited host-lost`. The harness's last check (every scenario) finds no live host after `quit-end-all` | `Tests/EndToEndTests/HostingTests.swift`, `Instance.swift` (`with(hosts:)`, `hostProgram`, `hostsLeft`, `finish`), `Scratch.swift` (`giveHosts`, `liveHosts`), `WindowHub+Reading.swift paneBarSummary` |
| W12 TEST | `cargo test -p wormhole-ffi each_profiles_terminal_answers_with_its_own_colours` | a terminal of each of two profiles answers OSC 11 with its own profile's background before any view; a change of b's theme reaches b's terminal only | `ffi/src/engine_look.rs` (per-profile `LOOKS`, `TERMINALS` with the profile), `launch.rs` (`current(profile)`), `terminal.rs` (`options_for` returns the profile, `from_parts(.., profile, ..)`), `hosting/desk.rs taken`, `launch_tests.rs` |
| W12 | Profiles A and B with different themes, a window of B open beside A's; in B start `printf '\e]11;?\a'; cat -v` | the answer is B's background; change B's theme in its Settings: B's terminals hear it, A's do not; switch the shown profile to one with the same theme as the one left: new terminals of it still answer in its colours | `Look/TerminalLook.swift` (`engineProfile`, `shareEngineColours`), `Look/EngineColors.swift`, `AppController.swift terminalLook`, `Settings/ProfileLook.swift`, `ProfileChipBinding.swift switchProfile` |
| W01 | Motion on: ⌘D, ⇧⌘D, the title row's split buttons, the pane menu's and the row menu's Split right; then drag a terminal from the list onto a pane's edge, and drop a pane's header on another pane's edge | the splits play the opening; the drop and the join land at once (no spring) whatever room the new pane takes | `WindowHub.swift send` (`splitCame`), `WindowHub+Panes.swift opensSplit`, `PanesUI/Panes/PaneInfo.swift splitCame`, `PaneArea.swift`, `SplitOpeningTracker.swift` (`split`, `weighed`), test `SplitOpeningTrackerTests.testOnlyThePaneASplitOpenedPlays` |
| S09 | Terminals in tabs: two groups (one folded) and loose terminals; a terminal of the other group needs input, another fails | the sets are the groups (the folded one with all its terminals) then Ungrouped (in the language's word); the switcher's dot is amber for needs-you, red when only a failure is elsewhere; each card's alert likewise | core `workspace/snapshot/tabs.rs` (`tab_sets_view`: `tab_sets`, `current_set`, `others_need_you`), ffi `workspace_tabs.rs tab_sets`, `WorkspaceBridge/TabSetsMapping.swift`, `WorkspaceModel/TabSetFacts.swift`, `WindowHub+Sidebar.swift tabBar`, `TabBarModel.make(alerts:)` |
| W03 | Status bar with 1, 3 and 5 terminals needing input; hover the needs-you button | "a · go there"; "a, then b, c · go to the first"; "a, then b, c and 2 more · go to the first" (the names and the count are the core's `needs_you_named`) | core `view/layout/status_bar.rs` (`NEEDS_YOU_NAMED`, `Summary::needs_you_named`), ffi `status_counts.rs` (`named`, `more`), `CoreStatusRules.swift`, `WindowUI/StatusSummary.swift` |
| S08 | Classic, Drawer and Tuck insert poses: + on a group's head; move a loose terminal into an open group | the new row plays the insert pose inside the card as a loose one does (read: `GroupRows`' `ForEach` owns the transition, keyed by the terminal's id, which does not change; entering a group is a removal from the loose card and an insertion in the group's rows) | `SidebarUI/Group/GroupRows.swift:45`, `Row/RowPoseTransition.swift` |

## Lane panel, wave 8

Written without compiling: expect compile errors.

P06, every order of events a notes.md merge can meet, as the code now handles it (`NotesCoordinator.sync/land/store`, `NotesModel.rebase`,
`NotesMapping.applying`; the core's base is set in `notes_sync.rs` `step`). "Stored" is the app's database, "base" the core's.

| order of events | what happens | lost or doubled? | base against stored |
|---|---|---|---|
| shown, nothing done during the merge | the answer is applied as it comes: notes keep their ids by the core's slots, steps by their words | no | equal |
| shown, a step ticked or unticked meanwhile | the person's tick wins; the file's own ticks and lines come in; a step the file removed stays when ticked meanwhile | no | stored = base + the tick, which the next merge writes to the file |
| shown, a step or note added meanwhile | kept once, where it was put; the file's additions go after the item they follow in the file, else before the next one there, else last | no | the addition goes to the file next |
| shown, a step or note deleted meanwhile | stays deleted (one the file kept is not brought back) | no | the deletion goes to the file next |
| shown, steps or notes moved meanwhile | the person's order; the file's additions follow their item | no | the order goes to the file next (the core's merge follows the app's order) |
| shown, a note rewritten meanwhile | the new words once; when the file rewrote the same note too, both versions stay (the core's rule for two edits), with two ids | no | in step after the next merge |
| shown, an editor opened after the send | the model is replaced but every item keeps the id it has on screen, so the editor stays on its row; a note the file rewrote keeps its id (its text changes under the editor, as GPUI's backstop) | no | as above |
| not shown | sent as read from the database, landed on a second read (all ids new): told apart by words and place among equals, nothing twice | no | equal |
| switched away during the merge | the switch saved the panel's model; the merge lands on that, stored for that terminal; the one shown now is untouched | no | equal plus what the person did, sent next |
| switched away and back | lands on the reloaded model (new ids): same rule | no | as above |
| closed during the merge | the base it just set is forgotten; nothing stored | no | none |
| moved to another window during the merge | nothing stored here (the other window shows what it read and would save over it); the base is forgotten (one store per profile now), so the other window's next merge keeps both sides | nothing lost; a line the file deleted in this one merge can come back once | none, then agreed afresh |
| window moved to another profile during the merge | the merge lands on what the new profile's database holds (the move copied it), stored there; the new profile has no base for it (as GPUI after a move): the next merge keeps both sides; the old profile keeps a stale base, as GPUI | no | agreed afresh |
| trust revoked | the core answers nothing, keeps the base | no | unchanged |
| the database cannot be read (not shown) | not sent; at landing: nothing stored, base forgotten | no | agreed afresh |
| the database cannot be read (shown) | the panel shows nothing, saves nothing, merges nothing; read again each sync, with what was typed meanwhile after what was saved | no | unchanged until read |
| two items that say the same | told apart by their place among equals (the core's `match_exact`) | no | equal |
| the file changes the same item | a tick both changed: the person's; words both changed: both kept | no | in step after the next merge |
| two windows of one profile | one base store per profile folder in the process (was a copy per window, each writing the whole `notes-sync.json` over the other's); trimmed at a window's start from the app's id counter, not the window's own highest id | no | one store |

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| P06 | TEST. `swift test --filter NotesMergeSavedTests`, `--filter NotesRebaseTests`, `--filter NotesMappingTests` | all pass; the fake store makes every item anew on each read, as the real one | `Tests/WormholeTests/NotesMergeSavedTests.swift`, `Tests/PanelTests/NotesRebaseTests.swift`, `Tests/FilesBridgeTests/NotesMappingTests.swift` |
| P06 | TEST. `cargo test -p wormhole-ffi the_windows_of_a_profile_merge_a_terminal_against_the_same_base` | passes: the second window's merge reads the file's deletion as the file's | `ffi/src/files_panel/project/hub.rs shared_bases`, `notes_sync.rs` |
| P06 | TEST (e2e). `swift test --filter EndToEndTests/notesSync` | two terminals of a trusted project: a tick in one reaches notes.md and the other, a line added to the file reaches both, nothing twice | `Tests/EndToEndTests/NotesSyncTests.swift` (new `Instance.with(auth: "allow")`) |
| P06 | Two windows of one profile, each with a terminal in one trusted project with a notes.md; edit notes in both; quit; delete a line from notes.md; launch | the deleted line stays deleted in both terminals (both bases were kept in `notes-sync.json`) | `hub.rs shared_bases`, `notes_ids_start_at` + `windows/registry.rs next_terminal_id` |
| P06 | A step's editor open on the shown terminal while notes.md gains a line | the editor stays on its step | `NotesMapping.applying`, `NotesRebase.swift` |
| P05 | TEST. `cargo test -p wormhole-ffi a_stored_list_that_cannot_be_read_is_never_written_over` | passes | `ffi/src/git/worktrees.rs edited` |
| P08 | Trusted project with an action key (`cmd-ctrl-t`): open the palette and press the key; the shortcut sheet; the new-worktree sheet; a diff line's comment | nothing runs while any is up; with none up it runs | `Wormhole/Panel/ProjectActionKeys.swift hasActiveDialog` |
| P02 | A terminal in a folder that is no repository, Changes read once; click its branch chip (where one shows) | no picker opens then or later when another list comes up | `WindowUI/Hub/WindowHub+Branch.swift` |
| P07 | TEST (hostile input). `cargo test -p wormhole-ffi a_folder_that_is_not_text_reads_back_as_its_bytes`, `a_project_in_a_folder_whose_name_is_not_text` (the second does nothing on APFS/HFS+, which refuse such names: run it on Linux or a FAT/exFAT volume) | pass | `ffi/src/files_panel/names.rs folder`, `tests/project.rs` |
| P07 | On a volume that allows a non-UTF-8 folder name (exFAT stick): a git repository with a `.wormhole/` in such a folder, a terminal `cd`'d into it | Files lists it, Changes and History read the repository, the Project tab finds the project and its trust, notes.md syncs; the row and title show U+FFFD | `ffi/src/workspace_cwd.rs`, `WorkspaceModel/Terminal.swift readPath`, `WindowUI/Chrome.swift selectedPath`, `git/service.rs`, `files_panel/project/*` |
| P07 | NOT DONE: the Settings window's project folders page, the recent projects menu, the trust window, a worktree's own folder and a new terminal opened in such a folder still carry the path as text | - | `settings_page.rs` roots, `RecentProjects.swift`, `AuxController.swift`, `git/worktrees.rs GitWorktree`, the workspace's `newTerminalIn` |
| P01 | VoiceOver on the right panel's strip | each tab is read as a radio button (a tab) with its words and counts, "n of 6", the shown one selected; activating one shows it; nothing drawn changes | `WindowUI/PanelTabBar.swift accessibilityRepresentation` |
| P05 | `wormhole --worktree` request whose window closed before it arrived | the question comes up in the first window; a second request to that window is still accepted once the sheet is gone | `Wormhole/Hooks/HookHost.swift` |

## Lane e2e-a, wave 8

Written without compiling and not run: expect compile errors and first-run failures. Scenarios for the rows T01 to T31 (not T20, the
window-hosting lane's), S01 to S10, W01 to W12, X03, X04 and X08, in `macos/swift/Tests/EndToEndTests/` (`swift test --filter
EndToEndTests/<name>`). Line numbers below are this file's, as it stood before this section. Harness additions: `Instance.end(by:)`
(SIGKILL as a crash, SIGTERM as `kill`), `InstanceTerminal.swift` (`shell`, `emit` for bytes `/bin/cat` writes back, `discardLine`,
`markPrompts` for OSC 133 blocks in `zsh -f`, `terminalFrames`/`cellPoint` from `a11y`, `asked`, `live(n)`, `flags`, `pasteboard`,
`plain` without Fluent's isolation marks), `Listing` with a group's count and pinned, a row's pinned, `Panes` with each pane's size.

| scenario | file | rows | lines of this file | count |
|---|---|---|---|---|
| copyAndPaste | TerminalInputTests | T04, T07, T15 | 18, 26, 30, 667 | 4 |
| optionAsMeta | TerminalInputTests | T07 | 17 | 1 |
| links | TerminalInputTests | T08, T15 | 25 | 1 |
| textZoom | TerminalInputTests | T17 | 33 | 1 |
| commandBlocks | TerminalBlocksTests | T13, T15 | 28 | 1 |
| foldAndExport | TerminalBlocksTests | T15, T25 | 31, 37 | 2 |
| programNotices | TerminalBlocksTests | T31 | 108, 109, 111 | 3 |
| colourQueries | TerminalProgramsTests | T28 | 92, 93 | 2 |
| windowOperations | TerminalProgramsTests | T29 | 95, 96, 97, 665 | 4 |
| programClipboard | TerminalProgramsTests | T16 | 32, 57, 661 | 3 |
| copyInAnotherPane | TerminalProgramsTests | T16 | 662 | 1 |
| scrollbackSize | TerminalProgramsTests | T28 | 91 | 1 |
| savedText | TerminalRestoreTests | T30 | 99, 102 | 2 |
| textAfterCrash | TerminalRestoreTests | T07, T30 | 19, 100, 101, 500 | 4 |
| profileEnvironment | TerminalRestoreTests | T27 | 85, 89 | 2 |
| closeWhatRuns | SidebarCloseTests | S01, W08 | 158, 313, 317 | 3 |
| closeInFoldedGroup | SidebarCloseTests | S01, W08 | 185, 314, 315 | 3 |
| closeAgent | SidebarCloseTests | W08 | 312, 316 | 2 |
| groupFolderQuestion | SidebarGroupsTests | S01, S03 | 163, 821 | 2 |
| groupGoneBeforeTheAnswer | SidebarGroupsTests | S03 | 1163 | 1 |
| groupKeys | SidebarGroupsTests | S03, S08, S09 | 167, 177, 208, 1164 | 4 |
| rowMenu | SidebarRowsTests | S01 | 165 | 1 |
| showBesideNeedsRoom | SidebarRowsTests | S01 | 164 | 1 |
| renameInPlace | SidebarRowsTests | S02, S03 | 166, 168 | 2 |
| windowTitle | WindowTests | W09 | 813 | 1 |
| viewSwitchesSurvive | WindowTests | S09, W03 | 175, 810 | 2 |
| quitBySignal | WindowTests | W08 | 321 | 1 |
| splitTwiceAtOnce | WindowTests | W01 | 1162 | 1 |
| fullScreenComesBack | WindowTests | W05 | 214 | 1 |
| titleRowFacts | WindowTests | W02 | 803, 804, 809 | 3 |
| keysInTheirWindow | WindowControlTests | X04, X08 | 212, 747 | 2 |
| splitFromCommandLine | WindowControlTests | X08 | 746 | 1 |
| worktreeFromCommandLine | WindowControlTests | X08 | 748 | 1 |
| profileWindows | WindowControlTests | W05, W12 | 811, 812, 1156 | 3 |
| listeningPort | WindowControlTests | W02 | 205 | 1 |
| keyboardAreas | KeyboardTests | S06, X04 | 183, 211 | 2 |
| terminalAccessibility | KeyboardTests | X03 | 63 | 1 |

Covered in part (the rest is a person's or waits for a command): 17 (which Option key: a person), 30 (the menu in the alternate screen
and in a hosted terminal), 63 (line ranges and the selection: VoiceOver), 85 (`$0` and the login shell: the harness's shell), 89 (the
git identity is not read), 97 (14 t, 19 t, 11 t not asserted), 111 (looked at, notifications off: a background app never looks),
166 and 168 (Esc and a blank name: an injected Esc never reaches SwiftUI's `onKeyPress`), 214 (the second display), 314 (⇧⌘W in a
second window), 315 (the pane's ×), 316 (an agent beside a second program: one shell for every terminal of an instance), 667 (⌘⌫ ⌘← ⌘→
and a rebound key: see the driver below), 1162 (a click on the header while the motion plays), 1164 (the switcher's search).

What waits for a command or a harness change (b), most lines first:

| needed | what it would give | lines waiting |
|---|---|---|
| a session host in the harness (`WORMHOLE_HOST_BIN`, the window-hosting lane's) | persistent terminals in a scenario | 105, 193, 194, 319, 320, 1158 |
| `status-bar` | the bar's items: the attention button and its tooltip, the counts, the state words, the size item, at a width | 195, 196, 197, 198, 806, 1165 |
| `switcher [open\|search <q>\|press <card>]` | the group switcher's cards, footer, search and its popover | 174, 176, 491, 492, 824 |
| an instance whose `default_shell` and shell keys are honoured (the harness sets `WORMHOLE_SHELL`, and allows only cat and `zsh -f`) | which shell a terminal runs | 86, 87, 90 |
| `WORMHOLE_AUTH=allow` as a harness option | trusting a project, approving a Probe | 88, 828, 1157 |
| `tab-bar` (tabs, their ⌘ keycaps and alert dots) and `tab-click <n>` / its + | the tab bar without a pointer | 172, 822, 825 |
| `plus-menu` | the + menu (New terminal with) of the title row and the sidebar | 179, 182 |
| `sidebar-tip` | the sidebar's "No coding agents yet" note and its buttons | 181, 827 |
| `scroll-state` (the top line as drawn, folds counted) | what the wheel and ⌘↑ do around a fold | 663, 664 |
| a Changed page Copy JSON command | the button, then `a11y said` | 755, 943 |
| the focus detail naming the list's and the tab strip's item | where the list cursor and the tab ring are | 826, 833 |
| `pointer-shape x y` | the cursor a view sets | 23 |
| `other-click x y <button>` | back/forward mouse buttons | 24 |
| `find-state` | the find bar's query, count and options (⌘F's selection seed) | 27 |
| `minimap` | the minimap's fold ticks; a press on it | 29 |
| the drop tint in `drag`'s reply or `state` | the pane's tint while a file hovers | 35 |
| `group-popover <id>` | open a group's popover (then `popover` reads it) | 169 |
| `panes` in the window's points | a divider to drag and cancel with Esc | 201 |
| the document-edited mark in `windows` | the close button's dot | 318 |
| `update restart` without an installed update in a test instance | the restart question | 322 |
| a panel switch command (diff split, wrap) | a B window's panel switch and its file | 497 |
| the exited words in `pane-bar` | "fish exited" | 660 |
| the centre title's state words (`title` has the facts, not the words) | "working · 4m" with the bar off | 805 |
| `pane-menu` | the pane header's menu and its keys | 829 |
| a command-line run of the app with a deadline (`Spawn.run` waits without one, and a wrong answer would launch a window) | `Wormhole --json list` | 936 |

The driver itself: `key cmd-left|cmd-right|cmd-backspace` reach the router, which has no handler for the keymap's "viewed" actions
(`lineStart`, `lineEnd`, `deleteLine`: `ActionRouter.viewed`), so nothing happens; the real app gives the key to the terminal's
`performKeyEquivalent` first (`AppController+ControlKeys.swift` `sendKey`). And `key esc` into a field goes through `interpretKeyEvents`,
which SwiftUI's `onKeyPress(.escape)` (`RenameField`, `FindField`) never hears.

For a person (`MANUAL.md`, c): 20, 21, 34, 36, 64, 98, 106, 107, 159, 160, 161, 162, 170, 171, 173, 184, 199, 202, 203, 204, 206, 207,
209, 486, 487, 488, 489, 490, 493, 494, 495, 496, 499, 511, 512, 513, 515, 516, 666, 752, 753, 754, 807, 808, 814, 815, 816, 817, 818,
819, 820, 823, 830, 832, 934, 1160, 1161, 1167 (hover never shows in a background instance; looks, colours and motion; VoiceOver's own
reading of SwiftUI; a real drag, panel, editor, Colours window or system setting; z-order and focus events a background window never
gets).

Stale or nothing to check (d): 22 (OSC 12 is built, line 93), 65 (built since, 755), 94 (FrameWorkGuardTests, a unit test), 178 (built
since, 488 to 493), 180 (built since, 486), 200 (built since, 494 to 499), 210 (not built, the settings lane's), 212's wording (the
driver limit is gone: `cmd<n>` sends keys to the n-th window; the line is covered), 501, 502, 503, 1166 (NOT WRITTEN / NOT DONE
notes), 670 (none written), 834 (NOT REAL), 835 (written since, 1160), 836, 935 (nothing to wire).

Scriptable but not written yet (w): 110 (needs an OSC 7501 fixture), 498 (Zen with an A and a B window), 514 and 1159 (a B window's
colour answers: B's theme set in its own `config.json`), 831 (a worktree shared with a terminal in a folded group).

Counts, of the 200 lines of these rows: 71 covered by a scenario (some in part, above), 58 for a person, 16 stale, 50 waiting for a
command or a harness change, 5 scriptable and not written.

Found while reading the code for the assertions:
- ⌥⌘C does not copy (line 18 promises it): `TerminalMetal/Pointer/TerminalShortcuts.swift` `shortcut(for:)` refuses any ⌘ key that
  has ⌥ or ⌃ before it asks the keymap or tries the key with ⌘ alone; GPUI's `platform_command` (`terminal_view/input.rs:105`) tries
  the plain ⌘ key for any key that is not an app key, so ⌥⌘C copies there. `copyAndPaste` asserts it and is expected to fail.
- Line 822's "Ctrl-Opt-N" (and the comment at `WindowUI/Hub/WindowHub+Sidebar.swift:139`) names a key that neither keymap has (no
  ⌃⌥N row in `keymap/table.rs`): only an empty pinned set opened by ⌃⌥1–9 or the switcher reaches `newTerminal(inGroup:)`.

## Lane e2e-b, wave 8

Written without running: no scenario below has been run. The rows of this lane (P01 to P09, ST*, STX*, SD*, A*, O*, PL*, X07, X12
to X15) were read line by line, 460 table lines in the sections above; each became a scenario's assertion, a line waiting for a
command, a line for a person (MANUAL.md), a stale line, or none of these (a static check, a unit test, a note). The numbers below
are this file's line numbers above this section. The notes merge (lines 228, 426, 787, 952 to 954) is the panel lane's scenario
and is left to it.

Harness added: `Instance.with(auth:config:)` (the owner's answer, a config.json before launch), `Instance.cli`/`cliRun`
(`Client.swift`: the app's own command line as a client, waited on by its exit event), `later` (a command that answers after its
batch: `project …`, `update`, `studio-save`, `history-copy`), `flags`, `setting`, `terminal(in:)`, `screen(showing:)`, `count`,
`signal`, `Scratch.project`, `Scratch.rawFile`, `Spawn.start`.

| scenario (`EndToEndTests/<file>`) | rows | lines | the list's lines |
|---|---|---|---|
| `untrustedProject` (ProjectTests) | P08, STX2 | 1 | 145 |
| `trustedProject` (ProjectTests) | P08, STX2 | 2 | 222, 875 |
| `actionKey` (ProjectTests) | P08 | 3 | 424, 615, 959 |
| `projectFromASubfolder` (ProjectTests) | P07, P08, X12 | 4 | 226, 473, 893, 896 |
| `projectTheme` (ProjectTests) | X14 | 1 | 293 |
| `accessAsked` (AgentAccessTests) | A05, X12 | 2 | 639, 720 |
| `accessRefused` (AgentAccessTests) | A05, X12 | 2 | 640, 719 |
| `accessModes` (AgentAccessTests) | X12 | 1 | 692 |
| `accessWidenedOutside` (AgentAccessTests) | A07, X12 | 3 | 352, 642, 721 |
| `accessGrantedScripts` (AgentAccessTests) | X12 | 1 | 641 |
| `settingsSavedAndReset` (SettingsPagesTests) | ST01, ST03, X12 | 4 | 122, 124, 139, 912 |
| `settingsSearchAndReveal` (SettingsPagesTests) | ST01 | 1 | 864 |
| `profiles` (SettingsPagesTests) | ST02, X12 | 4 | 130, 131, 132, 898 |
| `commandLineTool` (SettingsPagesTests) | ST03, X12 | 2 | 141, 913 |
| `importSettings` (SettingsPagesTests) | ST03, X12 | 2 | 140, 909 |
| `shortcutRecorders` (SettingsPagesTests) | PL02, X12 | 3 | 147, 149, 911 |
| `fontPicker` (SettingsPagesTests) | ST09, X12 | 2 | 144, 910 |
| `notificationKinds` (SettingsPagesTests) | ST03 | 1 | 844 |
| `settingsRowMenuAndKeys` (SettingsPagesTests) | ST01, STX3, PL02 | 3 | 125, 146, 1122 |
| `studioThemes` (StudioTests) | SD01, X12 | 9 | 275, 279, 282, 283, 284, 291, 644, 645, 1140 |
| `userThemeFile` (StudioTests) | SD03 | 2 | 273, 274 |
| `studioPair` (StudioTests) | SD01 | 3 | 280, 1138, 1139 |
| `studioOlderAndNewerFiles` (StudioTests) | SD01 | 2 | 278, 285 |
| `filesRename` (FilesTests) | O05, P07 | 2 | 55, 243 |
| `filesPinsAndMenus` (FilesTests) | O01, O05, P07, X12 | 5 | 59, 234, 247, 690, 916 |
| `filesRawName` (FilesTests) | P07 | 1 | 609 |
| `changesFromASubfolder` (GitPanelTests) | P01, P02, P03 | 4 | 239, 242, 472, 765 |
| `compareFollowsEachTerminal` (GitPanelTests) | P02, P03, X12 | 3 | 232, 768, 891 |
| `historyDetails` (GitPanelTests) | O01, P04, X12 | 6 | 233, 249, 464, 731, 892, 1118 |
| `worktreePort` (WorktreeNotesTests) | P05 | 2 | 38, 250 |
| `worktreeRefusedUntrusted` (WorktreeNotesTests) | P05 | 1 | 777 |
| `notesTools` (WorktreeNotesTests) | P06, X12 | 3 | 230, 888, 889 |
| `featureSwitchesAndMenus` (PanelMenusTests) | PL01, X12 | 3 | 48, 712, 917 |
| `panelTabKeys` (PanelMenusTests) | PL01, P01 | 3 | 713, 780, 781 |
| `overlaysGiveTheKeysBack` (PanelMenusTests) | O02 | 1 | 323 |
| `openedFromTheSystem` (PlatformTests) | PL07, PL09 | 2 | 348, 351 |
| `sigtermQuits` (PlatformTests) | PL09 | 1 | 350 |
| `ambientRows` (PlatformTests) | PL04, PL05, PL06 | 2 | 49, 50 |
| `noticesAndBanners` (PlatformTests) | X12 (O05, PL03) | 3 | 899, 900, 901 |
| `hostsLicensesUpdates` (AuxWindowsTests) | A03, X12 | 3 | 903, 904, 1121 |
| `firstRunAndWelcome` (AuxWindowsTests) | X12 (A01) | 2 | 906, 907 |
| `toolingPictures` (AuxWindowsTests) | X12 | 2 | 643, 905 |
| `controlFilesPerWindow` (DevToolsTests) | X12 | 4 | 680, 681, 691, 921 |
| `readAndDriveByRow` (DevToolsTests) | X12 | 6 | 685, 686, 687, 688, 689, 937 |
| `inspector` (DevToolsTests) | X12 | 1 | 635 |
| `pseudoLanguage` (DevToolsTests) | X07 | 2 | 68, 750 |
| `folderPicker` (FolderPickerTests) | O03, X12 | 2 | 60, 897 |
| `groupTools` (GroupToolsTests) | O04, X12 | 4 | 326, 894, 895, 915 |

Missing commands, and the lines that wait for each (most lines first):

| what is missing | lines |
|---|---|
| `settings-page`: the page on show as text: its banners and notes (config.json unreadable, profiles rebuilt, Zen mode, the hotkey's conflict, the test row's status, the permission row), the rows' captions, swatches and enabled state, the nav's marks | 20: 127, 128, 148, 445, 717, 845, 851, 852, 853, 854, 855, 856, 857, 858, 865, 866, 867, 1143, 1144, 1145 |
| `changes-menu <n>`, `history-menu <n>`, `files-row-menu <path>`: a panel row's context menu kept for `menu` and `menu-pick` (Copy Path, Open, Reveal, File History, Open in ▸, a commit's copies, ⌘C in a list) | 9: 52, 235, 238, 333, 334, 335, 766, 767, 773 |
| `diff-spans <line>` (and the viewer's): a line's colour runs by token | 8: 578, 579, 580, 581, 582, 583, 586, 588 |
| `theme-cards`: the Appearance page's cards (names, shelves, warning marks, the project notice, Hidden themes) and a card's menu | 7: 449, 868, 869, 870, 871, 872, 873 |
| a session host in the harness (`WORMHOLE_HOST_BIN` beside the built app, Persistent terminals on, an older wire version): not a command, a fixture | 6: 56, 361, 637, 638, 729, 1120 |
| `onboarding-state`: the first run's step, its trust card, Recents, the primer's lines, and a Look card's pick | 6: 276, 722, 723, 724, 725, 1116 |
| `diff-header` and `diff-step next|prev`: what the diff's header says (File N of M, Renamed from, the commit's hash, a submodule's card) and its arrows; the sideways extent | 6: 240, 241, 468, 469, 470, 770 |
| `settings-press <row id>`: a page's button pressed (Copy as JSON, Theme Studio, Import theme, Choose app…, Reset tips, Install/Repair/Remove hooks) | 5: 123, 133, 134, 135, 138 |
| a feed the instance may be pointed at without the network: core's `check_url` refuses `file://` and `--update-feed` wants HTTPS, so no update scenario can run on a file the test writes | 5: 143, 357, 358, 360, 908 |
| `project-layout` (the Project tab's Open terminals) and `project-prompt-note <id>` (Copy into a new note) | 4: 224, 227, 790, 958 |
| `usage`: the Usage tab's meters, reset words and Updated line | 4: 252, 791, 961, 962 |
| `files-viewer`: the viewer's lines, its picture's state, and its Open in / Reveal buttons | 4: 244, 245, 246, 418 |
| `recents [folders|projects]`: the recent folders and projects a profile's database holds | 4: 61, 73, 732, 1114 |
| `panel-kept`: what the Files, Changes and History stores keep per terminal, and whether Files reads and watches | 3: 422, 612, 613 |
| `history-copy` that copies (the chip, the instance's pasteboard), not only says the text | 2: 53, 54 |
| an agent fixture that draws a prompt box, for `stash` and ⌥⌘S | 2: 229, 431 |
| `studio-complete <text>`: the expression field's completion list | 2: 287, 1137 |
| `history-parent <n>` (a parent link) and the picker's own first rows (HEAD's words, Loading branches…) | 2: 248, 465 |
| `branch-chip [pane]`: a click on the title row's branch chip | 2: 769, 960 |
| `comment-at <file line>`: the comment dialog opened on a diff line by number (no click at a point) | 2: 771, 890 |
| `title-menu`: the title row's own right-click menu | 2: 330, 331 |
| `agent-caps <approval> <capability> on|off`: Agent access's per-approval ticks | 1: 860 |
| `settings-records`: the Coding agents page's records (login folders, skills) | 1: 861 |
| `trust-state`: the project commands window's text | 1: 730 |
| whether the palette or the shortcut sheet is up, in `state` | 1: 325 |
| `project-state` with the stores' problem and the folder's name (`.andromeda`) | 1: 425 |
| `files-icons`: the icon each Files or Changes row draws | 1: 294 |
| `history-state` saying which of the open commit's files are present (the open icon) | 1: 463 |
| `notes` saying whether a draft step is open | 1: 788 |
| `palette-key <keys>`: the palette's own keys on a row (⌘⌫ on a terminal, ⌘C on a setting) | 1: 58 |
| a marking shell fixture (OSC 133) in the harness, for `blocks` | 1: 682 |

For a person (MANUAL.md): 66, 69, 70, 129, 136, 137, 142, 150, 236, 237, 288, 289, 290, 324, 327, 349, 356, 359, 363, 371, 387,
388, 389, 390, 391, 392, 393, 420, 421, 429, 430, 432, 446, 447, 448, 461, 462, 466, 467, 471, 533, 536, 537, 538, 539, 540, 541,
542, 543, 544, 545, 546, 587, 605, 606, 607, 608, 610, 611, 631, 632, 633, 634, 646, 648, 715, 716, 726, 728, 751, 772, 783, 784,
785, 786, 789, 846, 848, 859, 862, 863, 874, 878, 919, 920, 938, 944, 1112, 1117, 1119, 1133, 1134, 1136, 1141, 1142. Real
banners, menus, the menu bar, the status item, the Dock, the hotkey against another app, signed bundles and their updates, Touch
ID, VoiceOver, hover and the real pointer (SwiftUI pop-ups, reset arrows, path cards), animation and timing, and looks (gallery
specimens, wrapping, fonts).

Stale (a later section says what is true now): 51, 71, 126, 698, 699, 700, 701, 702, 703, 727, 733, 734, 847, 850, 879, 880. 51 is
answered by 326 (a native menu); 71 and 727 by 1121 (Check for Updates opens Settings on Updates, the update window is deleted);
126 by 446; 698 to 703 by the passes that wrote those commands; 733, 734, 847, 850, 879 and 880 by wave 7's lines.

Not an app behaviour (a static check, a unit-test or cargo run, a build step, a note of something not written or kept): 67, 72,
295, 296, 355, 394, 400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 450, 451, 452, 534, 535, 547, 553, 554, 555, 556, 557, 558,
584, 585, 649, 650, 651, 652, 653, 654, 749, 792, 793, 794, 795, 902, 914, 922, 923, 924, 925, 963, 964, 1123, 1124, 1125, 1135,
1148.

A script could check them; not written in this wave (most need two profiles, a session host, a trusted project with a worktree
setup, or a minute's wait): 223, 225, 231, 251, 277, 281, 286, 292, 328, 329, 332, 353, 354, 362, 369, 417, 419, 423, 427, 428,
460, 614, 636, 647, 683, 684, 711, 714, 718, 774, 775, 776, 778, 779, 782, 849, 876, 877, 918, 939, 940, 955, 956, 957, 1113,
1115, 1146, 1147.

Counts: 460 lines. Covered by a scenario: 126. Waiting for a command or fixture: 115. For a person: 95. Stale: 16. Not an app
behaviour: 54. The panel lane's: 6. Not written yet: 48. Scenarios: 48.

Found while reading the code for the assertions (the list promises it, the code does otherwise):
- `onboarding <step>` ignores its step: `runWindow` sends every `onboarding …` to `runAux("onboarding")`, which opens the intro at its first step,
  and `onboarding close` opens it too. Only `onboarding-step <step>` walks (and closes). Line 906 and `onboarding-tailor`'s own error say
  `onboarding look` (`AppController+ControlWindow.swift`, `AppController+ControlPages.swift`).
- `project-init` says `creating <terminal's folder>/.wormhole` while the store makes it at the band, the repository's root, from a subfolder
  (`AppController+ControlNotes.swift`, `ProjectStore.create`/`act`).
- `folder-color` without a path colours the terminal's own folder (`project.folder`); the Project tab's pick colours the band (the
  repository's root): from a subfolder the tool and the tab colour different folders (`AppController+ControlSidebar.swift folderColor`).
- `history 1` a second time does not close the commit (line 892): only `history 0` or `history` alone closes it (`AppController+ControlPanel.swift`).
- `control --from <tab> …` in Ask mode answers "not carried here" and drops the question (`control_line.rs` `Answer::Ask`): line 692's
  "answer with agent-ask" has nothing to answer. The scenarios ask through the app's own command line instead (`Instance.cli`).
- No update scenario can use a feed the test writes: `--update-feed` needs HTTPS and core's `check_url` refuses `file://`; `update-installed`
  needs the real updater, which a test instance has only with a feed (line 908).
- A command that answers from a task (`project …`, `update`, `probe`, `studio-save|use|menu|open`, `studio-do <session verb>`,
  `history-copy`, `studio-out`) writes its answer after the batch's `sync` marks: `reply` gives it to the wrong command. `later` waits for it.

## Lane hosting, wave 9

Written without compiling: expect compile errors.

| row | what to do | what must happen | where the code is |
|---|---|---|---|
| T20 TEST | `cargo test -p wormhole-ffi ending_at_quit_keeps_each_terminal_with_its_notes_tasks_and_text ending_a_terminal_that_stays_keeps_its_row` (real `wormhole-host`, scratch folder) | after `finish(true)` the hosted terminal's row is there with its name, note, task and saved text and no host; the next launch plans nothing to attach, a start draws the saved text and keeps name, note and task; `end_keeping_row` keeps the row | `ffi/src/hosting/desk.rs` (`finish`, `taken` `Fate::Ends`, `seed`, `finish_restore(listed)`), `registry.rs` (`unname_hosts`, `remember` keeps the stored row), `slot.rs` (`kept`, `persisted_host`, `end_keeping_row`), `windows/registry.rs record_of_window`, `hosting/tests/mod.rs`, `tests/support.rs` (`row`, `name_row`, `save_notes`) |
| T20 TEST | `cargo test -p wormhole-ffi a_terminal_outlives_the_app ending_at_quit_ends_a_host_still_starting marking_the_quit_waits_for_nothing` | the old three still pass (no row names a host after End All; `finish_restore(vec![7])`); `mark` returns at once with a start under way and the fate is `Ends`; `quit(true)` returns when the start ends | `ffi/src/hosting/quit.rs` (`mark`, test), `hosting/tests/mod.rs` |
| T20 TEST | `cargo build -p wormhole-host`, then `swift test --filter EndToEndTests/endAllKeepsTheTerminals` (and `/endAllLeavesNoHost`, `/hostedTerminalOutlivesTheApp`) | a hosted cat with a note and typed text, `quit-end-all`: no host left; started again: the same terminal id, its screen shows the text above `restored`, the Notes tab has the note, neither `sessions` nor `hosts-list` names the old host | `Tests/EndToEndTests/HostingTests.swift endAllKeepsTheTerminals`, `RustWorkspaceBackend+Hosting.swift` (`quitEndingHosts`, `beginQuit`: hosted text saved before the end) |
| T20 | Persistent terminals on, a terminal with notes: Restart its shell (the exited bar's Restart, or a host that died then New) | the terminal starts again in a new host with its notes, tasks and name; the next launch has them too | `RustWorkspaceBackend+HostProblems.swift startAgain` (`endKeepingRow`), `RustWorkspaceBackend+Hosting.swift connected` (a start overtaken by a restart: `endKeepingRow`; by a close: `close`) |
| T20 | Persistent terminals on, a saved terminal whose attach fails (`failure`) or whose start falls back to the app's process at launch, with notes | its row, notes and text stay after the launch's pruning | `RustWorkspaceBackend+Hosting.swift finishRestore` (`listed: live`), ffi `desk.rs finish_restore` |
| T20 | Persistent terminals on, two windows, a host still starting (⌘T then at once Quit and End All Terminals; also with the quit question's box ticked); press ⌘Q again and type in a window during the wait | no window freezes: the windows draw and take keys while the hosts end; the second ⌘Q does nothing more; the app quits when they have; no `wormhole-host` of the folder is left; the next launch brings every terminal back with its text and notes, none attached | `AppController+Quit.swift` (`HostsEnd`, `goesNow`), `AppController.swift` (`hostsEnd`, `applicationWillTerminate` skips `finishQuit` after it), `AuxWindows/Questions/QuitFlow.swift` (`decide` returns whether to go now), `WorkspaceModel/Backend.swift quitEndingHosts`, ffi `desk.rs quitting` |
| T20 | Log out (or `kill <pid>`) while Quit and End All waits for its hosts | the app ends once they have, the logout is not cancelled | `AppController+Quit.swift` (`.terminateLater` while `underWay`, then `reply(toApplicationShouldTerminate: true)`) |
| W12 | Profiles A (dark theme) and B (light theme), A shown, no B window yet: Open Window in B whose saved terminal runs `printf '\e]11;?\a'; cat -v` at once | the answer is B's background, not the engine's default nor A's | `AppController+Windows.swift makeSession` (`profileLook(for:)` before `WorkspaceHost.make`) |
| W12 | A shown with a terminal running a program that follows mode 2031 (or `printf '\e[?2031h'; cat -v`), switch to B (other theme), change B's theme, switch back | A's parked terminal reports nothing while hidden (no light/dark flip) and keeps A's colours; back in A its windows wear A's look again | `ProfileChipBinding.swift switchProfile` (`(leaving + back).forEach(dressForProfile)`), `ProfileLooks.swift` (`ownSessions` note) |
| W12 | A shown; switch to B (other theme) whose saved window restores a terminal that asks OSC 11 at once | the answer is B's background, never A's | `ProfileChipBinding.swift switchProfile` (B's look made before the switch when it has no window), `Look/TerminalLook.swift shareEngineColours` (from the theme as it is now) |
| T07 | A shell without integration (`/bin/cat`): ⌘↑ and ⌘↓ | no beep; nothing is typed; with integration (zsh, blocks), they jump between prompts as before | `TerminalMetal/Keyboard/KeyDown.swift takesGreyedPromptKey`, `WindowUI/Menu/MenuStateMaker.swift` (the six items stay greyed) |

Who may remove a terminal's saved row (item 1's inventory), after this wave:
- Asked by the person, kept: `Event::Closed` (⌘W, a group's or row's Close, `wh close` after its question, End host then
  close) → ffi `workspace.rs close_what_closed` → `Terminal.close` → `HostSlot.closing` → `Registry.forget`; a window closed
  (after its question) → `windowClosed` → views' `shutdown` → `close`, and `Windows.forget` → `Store.delete_window`; Delete
  profile → `end(profile:)` (same two); the window saver's `Store.save_window` drops the sessions the model no longer lists
  (closed, or moved to another window, which saves them there); a start overtaken by a close (`connected`, stale token, not
  `live`) → `close`.
- Not asked, now kept: Quit and End All (`finish(true)`: `forget_all` replaced by `unname_hosts`, `close` by `end_keeping_row`);
  a start or attach that ends during that quit (`taken` `Fate::Ends`); Restart and the host bar's New (`startAgain`); a start
  overtaken by a restart (`connected`, still `live`); the launch's pruning (`finish_restore`) for a failed attach, a start that
  fell back to the app's process or failed (they are `listed`); a host that died (`Lost`: no close; its row stops naming it,
  `persisted_host`); a hosted start writing its row (`remember` wiped the name, notes and tasks of a saved terminal: it now
  writes the stored row with the host).
- Left as it was: the dev-only idle check (`--idle`) stops the selected terminal's source (`AppController+Idle.swift`), which
  forgets a hosted terminal's row; `finish_restore` still forgets a hosted row no listed terminal has (a workspace with no record
  of its own opened the saved terminals under new ids).
- W12 residue 4 (a window moved to another profile: terminals with no view stay under the old profile in the engine until a
  view dresses them) was not worked.
