//! Dev-only control hook for automated checks (cargo feature `devctl`).
//!
//! Run with `WORMHOLE_DEVCTL=<dir>`, then write commands, one per line, to
//! `<dir>/cmd`. The file is consumed on the next tick; results go to
//! `<dir>/out.txt`. `\n`, `\r`, `\e` and `\a` (the bell) in arguments are unescaped. Every
//! window has its own pair: the second window reads `cmd2` and writes `out2.txt`,
//! and so on, in the order the windows were opened. `WORMHOLE_AUTH=allow|deny|cancel`
//! answers the Touch ID prompt (for project trust) without showing it, and
//! `WORMHOLE_BACKGROUND=1` keeps the instance behind the other apps' windows, never taking the
//! focus (`crate::background`; `scripts/devctl-instance.sh` starts one so); both exist only in
//! devctl builds.
//!
//! Files that commands name (`shot <name>`, …) go into `<dir>`, and their names take letters,
//! digits, `.`, `_` and `-` only.
//!
//!   shot <name>     render the last frame to <dir>/<name>.png
//!   screen          dump the active terminal's visible text
//!   blocks          the active terminal's command blocks (a shell with integration), one a line:
//!                   its lines, how its command went and the command
//!   block <n> <select|copy-output|copy-command|to-prompt>   the Edit menu's action on the n-th block as
//!                   `blocks` lists them (`key cmd-up` / `key cmd-down` jump between the prompts)
//!   pane-bar        what the bar over the active terminal says: a program's question about reading
//!                   the clipboard (OSC 52), a notice (a copy it made, a refusal), or `none`
//!   clipboard-answer <once|always|deny>   the bar's three buttons
//!   images          the active terminal's inline images (kitty, iTerm2, Sixel), one a line: id, size,
//!                   decoded bytes or `decoding`, and each placement's cells
//!   state           dump terminals, statuses, cwds and notes
//!   type <text>     send raw input to the active terminal
//!   paste <text>    paste (bracketed if enabled) into the active terminal
//!   drag <x> <y> <path>   hover a file over the window at (x, y), as when dragging from Finder
//!   drop <x> <y>          release it there (send `shot` between the two to see the drop tint)
//!   drag-end              abandon the drag
//!   resize <w> <h>        set the window's content size (logical pixels; the window minimum still applies)
//!   click <x> <y>         left-click at a point in the window (logical pixels, as in a `shot` at 1x)
//!   double-click <x> <y>  two clicks there, the second a double-click (a resize handle's reset)
//!   key <keystroke>       press a key combination, e.g. `ctrl-cmd-2`, `shift-cmd-]`, `f6` or `tab` (the
//!                         bindings and a project's keys run as they would from the keyboard;
//!                         `state` says where the focus is after it: the area, the item its keys
//!                         are on, and `ring` while the focus ring shows)
//!   menus           the menu bar as AppKit has it, each item with its key and check mark
//!   menu | menu-pick <n|title> | menu-close   the last context menu (a right-click, ⌃↩, a menu button):
//!                   its rows one a line, the items numbered (`- title [x]` checked, `[off]` disabled,
//!                   `# heading`, `> submenu` over its indented items); choose an item by number or
//!                   title (`Submenu > Title`), as a click would; or dismiss it as Esc does. These menus
//!                   are native: no `shot` shows them and no `click` reaches them, and in a background
//!                   instance none is shown at all (a real one would hold the instance in AppKit's
//!                   loop), so a scenario right-clicks, reads `menu`, and picks or closes. A row of
//!                   colour dots (the group menu's) is a line `~ Grey | Blue [x]` (`[x]` the ringed one)
//!                   and is chosen from with `menu-pick colour <n|name>` (`menu-pick colour blue`)
//!   menu-native [press <n>]   the open menu built as the real AppKit NSMenu, not shown: its items and,
//!                   for the colour dots, each dot's tooltip and VoiceOver label; `press <n>` presses
//!                   that dot as VoiceOver does and prints which one the menu took as chosen
//!   popover | popover-press <title> | popover-close   the last popover (the folder's hover card, a group's
//!                   name): its rows one a line (`# path`, `field "name" (focused)`, `buttons: [Open in
//!                   Finder] [Copy path -> Copied]` the `->` being what a button says after a press,
//!                   `colours: Grey | Blue [x] | … | custom`, `- action`); press a button or an action row
//!                   by its title as a click does, or close it as a click outside does (the field's text
//!                   is kept). In a background instance (and in tests) it is not shown: a real one is a
//!                   window of AppKit's with its own keyboard
//!   popover-type <text> | popover-key <enter|escape>   type into its field (replacing what is there);
//!                   ↩ keeps it and closes, ⎋ drops it and closes
//!   popover-colour <name|#rrggbb>   a dot of the colours by name (the popover stays), or a colour from
//!                   the Colours window (`#rrggbb`: the last dot, or the window that `Other colour…` in a
//!                   group's menu asked for)
//!   popover-native [name]   the open popover built as the real AppKit controls (not shown) and drawn to
//!                   <dir>/<name>.png as AppKit lays them out, in the theme's light or dark
//!   popover-real | popover-real-close   open the recorded popover for real (an `NSPopover` at the window's
//!                   top left) and close it again: the check of AppKit's side that a background instance
//!                   can make; use it with a hover popover (`path-card open`), which takes no keys
//!   palette [query] | palette close   open the command palette over this window with `query`
//!                   typed (a prefix picks the scope: `@` terminals, `>` actions …; in an open palette
//!                   it replaces what is typed), or close it; it opens after this batch
//!   sheet [filter] | sheet close   the keyboard-shortcuts sheet over this window, filtered
//!   action <name>   dispatch a registered action by name (`workspace::ToggleSidebar`) through this
//!                   window, as a click on its menu item does
//!   available <name>   whether the focused element's path handles that action (app-wide handlers
//!                   like Hide's don't count here, though the menu counts them)
//!   about | licenses | about-shot [name] | licenses-shot [name]   the About and Licenses windows
//!   intro [style]   play the intro around this window: emerge, vortex, collapse, ftl, aurora, tentacles,
//!                   voyage or glow (as asked, whatever Reduce Motion says)
//!   intro-frames <style> [count]   draw count frames (8) off screen to <dir>/intro-<style>-<n>.bmp
//!                   (they open on the next tick: send the shot in a later batch)
//!   licenses-search <query> | licenses-select <name>   filter the Licenses list / open a package
//!   trust-window | trust-shot [name]   the window listing everything the active terminal's project
//!                   wants to run (the trust notice's "View all"); it opens on the next tick
//!   text <text>           type into whatever has the focus (a task's or note's editor), one key per
//!                         character, as the keyboard would (not only into the terminal like `type`)
//!   new [dir] | activate <n> | note <text> | task <text> | stash | send | send-submit   (`new` opens
//!                   the new terminal in `dir` when given, `~/` being the home folder)
//!   new-shell <key> [dir]   a new terminal running the shell `key` names (`shells` lists the keys;
//!                   `-` is the login shell); `state` shows each terminal's key and what it runs
//!   shells          the shells on offer, one a line: key, name, what it runs, `default` on the default
//!   hosts           the session hosts of this data folder, one a line: id, pid, shell pid, start (UTC),
//!                   version and `live` or `stale` (with why), as `Wormhole sessions list` prints them
//!   host-kill <id>  end that host as `Wormhole sessions end <id>` does (`Kill` over its socket, SIGTERM
//!                   when that fails; a stale one is refused); prints what happened
//!   detach          let go of every hosted terminal of this window, as quitting does: the programs
//!                   go on running in their hosts, the views keep their last screen; prints how many
//!                   (`state` ends each terminal's line with `hosted <id>` and what the host is doing
//!                   (`connecting`, `headless`, `detached`, `lost`), or `local`)
//!   reattach        attach those terminals to their hosts again, as a launch does (on screen with a
//!                   repaint, the rest headless); prints their ids
//!   quit-keep       quit as ⌘Q does: every hosted terminal is let go of and goes on running for the
//!                   next launch (`quit` is the same quit)
//!   rename <n> <name>   name the n-th terminal as a project's layout would (as `state` numbers them)
//!   group-new <name> <row>…   a new group of the terminals in those rows (as `state` numbers them);
//!                   prints its id
//!   sidebar-fold <group>   fold or open the group with this name
//!   group <name> <rename|pin|ungroup|close|move|color:<name|none>|folder:<path|none>>   what the group's
//!                   menu does: make its name a field on its head (`text <new name>` then `key enter`
//!                   types and keeps it, `key escape` drops it), pin or unpin, take apart, close (asks
//!                   if an agent works in it), move to a window of its own, colour it (grey, blue, red,
//!                   yellow, green, pink, purple, cyan or orange), or set the folder its new terminals
//!                   start in (an absolute path without spaces, `~/` the home folder; `none` clears it,
//!                   as Set folder… and Clear folder do; `rows` shows it as `folder=…`). A click on the
//!                   group's name (`click <x> <y>`) opens its popover (`popover`), a right-click its
//!                   menu (`menu`)
//!   to-group <row> <name|none>   put the terminal in that row in the group, or take it out of its group
//!   rename-edit <row>   the terminal's name becomes a field, as Rename… in its menu makes it
//!   pin <n>         pin the n-th terminal (as `state` numbers them) to the top of its group, or unpin it
//!   folder-color [path] <name|#hex|none> colour the folder (the active terminal's without a path; `~/`
//!                   is the home folder) red, orange, yellow, green, teal, blue, purple or pink, or
//!                   take its colour away, as the Project tab does: its terminals' title row is
//!                   tinted; the list of every window, in `config.json`, or the
//!                   project's `project_color` in its `.wormhole/config.json` when the folder's
//!                   project has one (refused while `project_color_locked`); prints what it did
//!                   (with `project` and `locked` where the project holds it, `title` shows the tint)
//!   hold-cmd | release-cmd   press ⌘ alone and keep it held (the ⌘1–9 keycaps show after 250 ms;
//!                   `shot` in a later batch), or let go
//!   right-click <x> <y>   a right-click there (a sidebar row's or group's, a pane's header or the + opens its
//!                   context menu: read it with `menu`)
//!   sidebar-scroll <px>   scroll the terminal list (negative = up)
//!   rows            the sidebar's list as it is drawn, one a line: each group, and each terminal's row
//!                   with its number as `state` has it, id, name, place in its group, ⌘ number and its
//!                   split's marks: `bracket=start|mid|end`, `chain`, `on_screen` (shown beside the
//!                   selected one), `selected`
//!   split-join <row> <pane> <left|right|top|bottom>   the terminal in that row joins the split of
//!                   the terminal `pane` (its id; made if it stands alone) on that side of its pane,
//!                   as a row dropped on a pane's edge does; it leaves the split it was in and is
//!                   selected. Prints `refused: …` when the room is too small
//!   split-leave <row>   the terminal in that row leaves its split and stands alone; the split
//!                   dissolves into a plain row when one terminal is left
//!   row-drop <row> <onto> <row|before|after>   drop the terminal in the first row on the middle
//!                   of the second's (`row`: it joins its split, or makes one), its top quarter
//!                   (`before`) or bottom quarter (`after`: a reorder, or leaving its split), as a
//!                   drag in the sidebar does; prints what it did
//!   pane-split <right|down> | pane-close [id] | pane-zoom   the selected terminal's split:
//!                   split the focused pane (a new terminal joins the split, made if need be), close
//!                   a pane's terminal (the focused one without an id), zoom the focused pane or unzoom
//!   pane-focus <id|left|right|up|down>   select that pane's terminal and give it the keyboard, or
//!                   the pane in that direction
//!   layout <columns|rows|grid|single>   View → Layout: the selected split as 2 columns, 2 rows, a
//!                   2×2 grid or a single pane; new panes are new terminals, surplus ones
//!                   leave the split
//!   panes           the selected terminal's panes: id, rectangle (in the panes' area, logical pixels),
//!                   `focused` and `zoomed`; then the dividers
//!   title           what the terminal's title row draws: the path as it is cut (and how many ways it
//!                   could be), the status words, the branch chip, whether Open in… and the splits are
//!                   one ⋯ menu, the name's width, the project's tint, the app Open in… opens and the
//!                   apps found, and the extras (the branch's
//!                   status, the agent's model and context, the ports, the pull request: what each
//!                   says, or that the row gave it up). A test instance has no `gh` unless
//!                   `WORMHOLE_GH_BIN` names a stand-in (a pull request is asked only in a
//!                   trusted project); a status line is `Wormhole statusline` run with
//!                   `WORMHOLE_SOCKET` and `WORMHOLE_TAB_ID` set to the instance's and the tab's
//!   path-card <open|close>   open or close the hover card (a popover: read it with `popover`) under
//!                   the title row's folder at once, as resting the pointer on it does after a moment
//!                   (`hover <x> <y>` does too)
//!   path-menu       open the folder's Finder-style menu as a right-click on it does (read it with
//!                   `menu`, pick with `menu-pick "Copy path"` or a folder's name; ⌃↩ on the title strip
//!                   opens it too)
//!   toggle-sidebar | toggle-panel   hide or show the sidebar / the right panel
//!   profile <id> | profile-new <name> | profiles   switch Wormhole to a profile (its windows show, the
//!                   shown profile's hide with their terminals running), make a profile (it is not
//!                   switched to) and print its id, or list the profiles (`*` marks the shown one)
//!   profile-open <id> | profile-move <id> | profile-delete <id>   open a window in a profile (the odd window
//!                   while Wormhole is in another), move the front window to one, or delete one
//!   new-window | close-window   open another window / close this one (`state` lists them)
//!   onboarding <welcome|folder|look|agents|primer|close>   open the intro window at that step over this
//!                   window with nothing played around it, whether or not it ran before (`close` closes
//!                   it without remembering, and presses Esc in a terminal whose welcome runs: a fresh
//!                   data dir's first launch opens the terminal welcome while its flag is on)
//!   welcome <cursor|portal|logo> [hidden]   replay the terminal welcome: the window leaves into the wormhole and
//!                   comes back with a new terminal running `Wormhole welcome`, the start animation
//!                   handing over to it as that hand-over has it (the ship docks on the mark's `>`, on
//!                   a block-art logo's chevron, or there is no ship); it plays even in a test instance; `hidden`
//!                   starts the run as a first launch's, over a window that was never shown
//!   welcome-dock [name]   the ship drawn at the welcome terminal's dock, over the window's picture, at
//!                   four moments of the landing: <dir>/<name>-<n>.png (`frames 2` first, and the
//!                   welcome's page should be in the window)
//!   welcome-probe <tab|-> <token> <op as JSON>   a welcome request as the program sends it, from that
//!                   tab (`-`: from outside any); the reply's JSON line is appended after the batch
//!   menu-new-window | menu-updates   the Shell > New Window / Check for Updates… menu actions,
//!                   dispatched through this window like a menu click
//!   settings | settings-section <name> | settings-shot <name>   (the settings window)
//!   update-installed <version> [pending]   Settings → Updates as after an install: Restart, or with
//!                   `pending` the line "Restarts when your terminals are idle."
//!   font-picker <interface|code|terminal> [query]   expand a font picker (open settings first)
//!   picker <compare|history> [query]   open the branch select in the Changes / History tab
//!   settings-project-menu [query]   open the project select in Settings → Project folders
//!   settings-search [text]   type into the settings search (empty clears it; open settings first)
//!   settings-record <id> [keys]   start a shortcut field (hotkey) recording, and press keys into it
//!   settings-reveal <id>   open the settings window at that row, scrolled to it and marked (what
//!                   the palette does for a setting)
//!   settings-preview <regular|compact|dense|off>   Terminals & sidebar's preview of a density, as on hover
//!   profile-sheet   the New profile sheet over the settings window (shot it with settings-shot next batch)
//!   settings-scroll <px>   scroll the settings page (negative = up)
//!   settings-resize <w> <h>   the settings window's size (a tall one shows a whole page in one shot)
//!   settings-select <scrollback|icon-theme>   open that row's select in the settings window
//!   settings-reset-confirm   press Advanced → Reset to defaults… (shows its question, resets nothing)
//!   settings-cli-tool [install|uninstall]   Coding agents → Command line tool: show it, or press its
//!                   Install / Uninstall (into the instance's own cli-bin folder, `WORMHOLE_CLI_DIR`)
//!   panel-tab <changes|history|notes|usage|project>   show that tab of the right panel
//!   project-init    what the Project tab's Create project folder button does for the active
//!                   terminal's folder: `.wormhole/config.json` made off the UI thread (a tick
//!                   later `title`, `rows` and the tab show the project); prints the folder
//!   comment <text>  the open comment dialog of a diff line (click a line first): its field set to
//!                   <text> and saved, as Add Comment does; prints the terminal's review summary
//!   project-trust   trust the active terminal's project as its trust button does, with Touch ID
//!                   answered by `WORMHOLE_AUTH` (start the instance with `allow`); prints the folder
//!   note-seed <count> <lines> | note-add <text> | note-toggle <n>   fill the terminal with long
//!                   notes / add one note at the end, open / fold or unfold the n-th note
//!   task-seed <open> <done> | task-done <n>   that many next steps (the done ones spread among
//!                   the open ones) / tick or untick the n-th
//!   panel-scroll <px>   scroll the Notes / Changes / History list (negative = up)
//!   changes | diff <n> | split   the Changes tab / open its n-th file's diff / toggle Split ↔ Unified
//!   compare [branch]   compare the Changes tab with a branch (none, or HEAD, to stop comparing)
//!   history [branch]   the History tab, on that branch (HEAD: the current one); a branch that
//!                   doesn't exist shows git's error, as a deleted one does
//!   diff-scroll-x <px>  scroll the open diff's code sideways (negative = back)
//!   frames <n>      redraw the workspace n times (as a scroll tick does) and append the timings
//!                   to <dir>/frames.txt (out.txt is rewritten per batch); run it once the tab
//!                   under test has finished loading
//!   paints          terminal grids drawn since the last `paints` (or launch) and per second: the
//!                   frame rate on screen, where a program in the terminal only sees what it sent
//!   perf-profile [seconds] [name]   draw this window again and again for that long (5 s) and record
//!                   what each draw cost (time, its terminal's parts, allocations) and what the
//!                   terminals' readers and locks cost meanwhile: <dir>/<name>.json and <name>.txt
//!                   (`perf-profile`), and the table in the results; the rest of the batch waits
//!                   (`scripts/profile-app.sh` runs it under a fixed load and compares)
//!   hover <x> <y>   move the pointer there (no click)
//!   scroll <x> <y> <dy>   turn the wheel over that point, dy logical pixels down (negative = up),
//!                   in steps with a frame between them, as a trackpad's scroll arrives: a long
//!                   list's end is reached (unlike `panel-scroll`). The rest of the batch waits
//!   mouse-down <x> <y> | mouse-drag <x> <y> | mouse-up <x> <y>   press the left button, move with
//!                   it held (e.g. to drag a task or note to a new place; `shot` in between shows
//!                   the drop line), release it. A terminal is dragged the same way, by its row or
//!                   its pane's header, onto a pane's edge or a row: send each move in a batch of
//!                   its own, so a frame is drawn between them (the overlay over the panes appears
//!                   after the first move and hears the next ones; `panes` has the pane rectangles)
//!   inspect         toggle the inspector (⇧⌥⌘I); `hover` highlights, `click <x> <y>` pins
//!   bounds <x> <y> [levels]   the element at a point (logical pixels): its bounds, where it is
//!                   built and its id path, then `levels - 1` of its parents. It opens the inspector
//!                   meanwhile (and widens the window by the panel, so the layout stays the same)
//!   studio | studio-select <token> | studio-set <value> | studio-scroll <y> |
//!   studio-save <name> | studio-shot <name>   (the Theme Studio window; it opens
//!                   on the next tick, so send the other studio commands in a later batch).
//!                   `studio-set` types the value into the token editor's field and presses
//!                   Return: one that doesn't work stays there with its error.
//!                   `studio-save` without a name is ⌘S: a name that is taken asks first (a
//!                   sheet, which no shot shows; `state` says `asking=true`)
//!   studio-type <text>   type into the token editor's field without Return: what it offers
//!                   for the name so far shows under it
//!   studio-filter <all|changed|low> [text]   what the token list shows, and its filter text
//!   studio-area <all|sidebar|panel|chrome|settings|ambient>   the gallery's filter, and the token
//!                   list's with it (only what that part of the app draws with)
//!   studio-frames <n> [tick|scroll]   draw the Studio window n times and append the timings to
//!                   <dir>/frames.txt (like `frames`); with `tick`, each draw follows an animation's frame
//!                   in the gallery, with `scroll` a wheel tick over it (40 px further down); run it
//!                   once the window has drawn
//!   studio-resize <w> <h>   the Studio window's size (the window minimum still applies)
//!   studio-menu [save|copy|switch|json|project]   open the Save theme menu, or pick Save
//!                   theme, Save Copy, Save Copy and Switch, Copy JSON or Use in this project from it
//!   hook [--agent <id>] <tab> <event> [message]   an agent's hook event for a tab (ids as `state`
//!                   lists them; the agent defaults to claude), e.g. `hook 1 Notification Allow?`.
//!                   With `/bin/cat` for a shell the status lasts until the next agent check (~2 s)
//!                   finds no agent: use a copy of cat named like the agent to keep it
//!   control [--from <tab>] <words…>   a control request as `Wormhole <words…>` sends it (e.g.
//!                   `list --json`, `send-text --tab 3 "hi" --submit`, …), as from the tab `--from`
//!                   names or from outside; the reply's JSON line is appended after the batch
//!   reduce-motion [on|off|system]   hold animations still as the system's Reduce Motion does (the
//!                   spinner's three dots, the steady needs-you halo, cuts for slides and folds),
//!                   for screenshots that don't depend on when they were taken, or follow the system
//!   scroller overlay|legacy|system   say the system shows scroll bars only while scrolling
//!                   (overlay: thin thumbs that fade) or always (legacy: a gutter), or follow it
//!   appearance [light|dark|system]   say the system is in Light or Dark Mode (Match system shows
//!                   that pick), or follow the system again; without one, what it is
//!   reduce-transparency [on|off|system]   say Reduce Transparency is on or off (every window
//!                   opaque, no blur, the note in Settings), or follow the system again
//!   increase-contrast [on|off|system]   say Increase Contrast is on or off (the theme's
//!                   high-contrast variant, opaque windows, a 3 pt focus ring), or follow the system
//!   a11y [said]     the accessibility tree VoiceOver gets (woken first, as an assistive app does), one
//!                   node a line: role, label, states, `*` on the focused one; `said`: the
//!                   announcements made lately (`!` before one that interrupts)
//!   keyboard-navigation [on|off|system]   say macOS's Keyboard navigation is on or off (⇥ then reaches
//!                   the title row's buttons too), or follow the system again
//!   window-front <on|off|system>   say this window is in front as far as the focus ring goes (it
//!                   hides in a window that isn't), to shoot it without taking the front
//!   front           bring this window to the front with the keys, background mode or not: only for
//!                   a check that truly needs it, as it takes the front from the app in use
//!   menubar | dock  the status item's icon and menu / the Dock's badge, bounces and menu
//!                   as text, as they would be built now (appended after the batch)
//!   menubar-icons [prefix]   the status item's icon in every state, as TIFFs in <dir>
//!   notify-sim <needs-you|agent-done|agent-failed|command-done|command-failed|bell|notice> <n> [post]
//!                   what the notification rules decide for that event in the n-th terminal
//!                   (delivery, level, sound, bounce, the words); `post` also delivers it. Run the app with `WORMHOLE_NOTIFY=log` so
//!                   nothing reaches the notification center, and read it back with `notify-log`
//!   notify-log      the notifications the `log` route kept, and the permission macOS reports
//!   notify-permission <not-asked|allowed|denied|outside|checking>   pretend macOS said so (the
//!                   welcome's notification step shows Allow only while it hasn't been asked; asking
//!                   is counted in `state` as `permission_asks` and taken as answered yes)
//!   quick-panel [close|where] | quick-panel-shot <name> | quick-panel-key <keys> | quick-panel-text <text>
//!                   open or close the hotkey's quick panel, draw it to <dir>/<name>.png,
//!                   press a key in it (`down`, `cmd-enter`, `escape`), type into its field
//!   jump            ⌥⌘J: go to the terminal that has waited longest (again: the next one)
//!   wait-idle [ms]  the rest of the batch waits until this window has no git status or history
//!                   refresh in flight and every window draws the same picture three times in a
//!                   row (default 5000 ms), and prints `idle` or `timeout: <why>`
//!   sync <token>    write `sync <token>` to the results file once everything before it in the
//!                   batch is done (after `wait-idle`, `bounds` and `settings-shot`; not `frames`),
//!                   as the last line: a script waits for it before reading the results

mod a11y;
mod ambient;
mod batch;
mod blocks;
mod commands;
mod control;
mod flow;
mod hosts;
mod input;
mod menus;
mod panels;
mod profiling;
mod sessions;
mod settings;
mod shots;
mod sidebar;
mod studio;
mod title;
mod welcome;

use std::path::Path;

use anyhow::{Result, anyhow};
use gpui_kit::*;

pub use batch::install;
#[cfg(test)]
pub(crate) use batch::poll_dir;

use crate::window::Workspace;

/// What a command can reach: the window it runs in and the control directory.
struct Ctx<'a, 'b> {
    workspace: &'a mut Workspace,
    window: &'a mut Window,
    cx: &'a mut Context<'b, Workspace>,
    /// `WORMHOLE_DEVCTL`: screenshots and frame timings go here.
    dir: &'a Path,
    /// This window's results file (a few commands report on their own, later).
    out: &'a Path,
    /// Set by a command the rest of the batch waits for (`wait-idle`, `bounds`): the next line
    /// runs once it resolves, and what it resolves to is that command's output.
    wait: Option<Task<String>>,
    /// Tokens `sync` writes once the batch is done.
    sync: Vec<String>,
}

/// A command: what it prints when it works (usually nothing), or why it did not. A failure
/// is reported as `<command> failed: <why>`.
type Command = fn(&mut Ctx, &str) -> Result<String>;

/// A line is a command name, then (after the first space) its argument, in which `\n`,
/// `\r` and `\e` stand for the control characters.
fn split_command(line: &str) -> (&str, String) {
    let (name, arg) = line.split_once(' ').unwrap_or((line, ""));
    (name, arg.replace("\\n", "\n").replace("\\r", "\r").replace("\\e", "\x1b").replace("\\a", "\x07"))
}

/// `<x> <y>` in logical pixels.
fn coordinates(arg: &str) -> Result<Point<Pixels>> {
    let mut numbers = arg.split_whitespace().filter_map(|v| v.parse::<f32>().ok()).map(px);
    numbers.next().zip(numbers.next()).map(|(x, y)| point(x, y)).ok_or_else(|| anyhow!("expected <x> <y>"))
}

/// The name of a file a command writes into the control directory (`shot <name>`, …), `default`
/// without one: letters, digits, `.`, `_` and `-` only, so no name reaches outside it.
fn file_name<'a>(arg: &'a str, default: &'a str) -> Result<&'a str> {
    let name = arg.trim();
    let name = if name.is_empty() { default } else { name };
    if name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        Ok(name)
    } else {
        Err(anyhow!("a file name takes letters, digits, `.`, `_` and `-` only, not {name:?}"))
    }
}

/// An optional 1-based index, as the `n`th thing is meant (missing or bad = the first).
fn nth(arg: &str) -> usize {
    arg.trim().parse::<usize>().unwrap_or(1).saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::{coordinates, file_name, nth, split_command};

    #[test]
    fn a_line_splits_at_the_first_space_and_unescapes_the_argument() {
        assert_eq!(split_command("screen"), ("screen", String::new()));
        assert_eq!(split_command("type ls -la\\n"), ("type", "ls -la\n".to_string()));
        assert_eq!(split_command("paste a b  c").1, "a b  c");
        assert_eq!(split_command("type \\e[A\\r").1, "\x1b[A\r");
        assert_eq!(split_command("type \\a\\r").1, "\x07\r");
    }

    #[test]
    fn coordinates_take_two_numbers_and_ignore_the_rest() {
        let at = coordinates(" 12.5   40 ignored").expect("two numbers");
        assert_eq!((f32::from(at.x), f32::from(at.y)), (12.5, 40.0));
        assert!(coordinates("12").is_err());
        assert!(coordinates("").is_err());
        assert!(coordinates("x y").is_err());
    }

    #[test]
    fn file_names_stay_in_the_control_directory() {
        assert_eq!(file_name(" home-1.v2_b ", "shot").ok(), Some("home-1.v2_b"));
        assert_eq!(file_name("", "shot").ok(), Some("shot"), "the default without a name");
        for bad in ["../x", "a/b", "/tmp/x", "a\\b", "a b", "~x", "n\u{e9}e", "x\0"] {
            assert!(file_name(bad, "shot").is_err(), "{bad:?}");
        }
    }

    #[test]
    fn an_index_is_one_based_and_defaults_to_the_first() {
        assert_eq!((nth("3"), nth(" 1 "), nth(""), nth("x"), nth("0")), (2, 0, 0, 0, 0));
    }
}
