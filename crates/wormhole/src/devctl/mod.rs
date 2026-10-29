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
//!                   the clipboard (OSC 52), a notice (a program's write to the clipboard or a refused Put at
//!                   Prompt, which the bar says only with the notice chip off), or `none`
//!   clipboard-answer <once|always|deny>   the bar's three buttons
//!   close-answer <close|cancel>   answer the question before closing a terminal or window (or quitting) as
//!                   its buttons do, through the native sheet on this window; `wh close` and ⌘W ask it, and an
//!                   instance without a person to click would otherwise leave it up
//!   images          the active terminal's inline images (kitty, iTerm2, Sixel), one a line: id, size,
//!                   decoded bytes or `decoding`, and each placement's cells
//!   state           dump terminals, statuses, cwds and notes; `lights-look` names the
//!                   appearance the traffic lights are drawn in (the theme's, not the system's) and
//!                   where the close light and its title bar sit
//!   type <text>     send raw input to the active terminal
//!   paste <text>    paste (bracketed if enabled) into the active terminal
//!   drag <x> <y> <path>   hover a file over the window at (x, y), as when dragging from Finder
//!   drop <x> <y>          release it there (send `shot` between the two to see the drop tint)
//!   drag-end              abandon the drag
//!   resize <w> <h>        set the window's content size (logical pixels; the window minimum still applies)
//!   scaling               the window's resize steps and animation, the shown terminal's cell, grid and spare
//!                         room (Motion feature); `settle` ends a live resize as the system would;
//!                         `reveal-test` draws the window's first frame by hand, as opening one does
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
//!   folder-picker open|close|type <text>|tab|right [mid]|left [start|mid]|accept-ghost|up|down|pageup|
//!                   pagedown|enter|pick|forget|
//!                   click <line>|crumb <n>|scroll <lines>|state   the folder picker (⌘J) over this window:
//!                   open it at the terminal's cursor (the Folder picker flag must be on: `set
//!                   feature_flags={"folder_picker":true}`), type a path or part of a name into its field
//!                   (src/, ../ and ~/ walk; pr/te finds Projects/terminal), tab goes into the selected
//!                   folder, right (at the end of the text; `right mid` with the caret in the middle, which is the field's
//!                   own) and accept-ghost take the grey completion, pick is enter, left
//!                   goes up one level (`left start` with the caret at the start, `left mid` in the middle
//!                   of the text, which is the field's own), up and down move the selection, enter inserts
//!                   the selected folder's path at the terminal's cursor and closes (the terminal's screen
//!                   then shows it), forget forgets a selected recent folder, click <line> picks the line,
//!                   crumb <n> goes to a step of the breadcrumb, state lists the text, the ghost, the
//!                   crumbs, the lines (badges, chevrons, recent rows, what ↩ and ⇥ do) with the selection
//!                   marked, the panel's place and, for a native panel, its material, frame and rows. The
//!                   queries run off the main thread: ask `state` again while it says `busy true`. In a
//!                   background-mode instance the panel is never shown (`visible false`, `key false`): it
//!                   takes no keystroke from the person's own apps, and everything else works as it would
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
//!   close <n>       close the terminal in row n at once, without the question the × asks
//!   new-shell <key> [dir]   a new terminal running the shell `key` names (`shells` lists the keys;
//!                   `-` is the login shell); `state` shows each terminal's key and what it runs
//!   shells          the shells on offer, one a line: key, name, what it runs, `default` on the default
//!   agents          what programs that ask to control this instance are up to: `access <off|ask|open>`, one
//!                   `approved <memory:id|stored:id> <name> <session|quit|always> <kind>` line per program let
//!                   in, and one `asking <id> <name> <shown|waiting> <buttons>` line per question waiting
//!   agent-ask <deny|session|quit|always>   answer the question that is up as that button does (a test
//!                   instance shows no sheet; `WORMHOLE_AGENT_ASK=hold` or `agent-script hold` leaves
//!                   questions waiting for this)
//!   agent-script <hold|deny|session|quit|always>   what answers questions from now on
//!   agent-grant scripts [quit|always]   let Scripts in, as an answer to its question does
//!   agent-hide <n> [on|off]   hide the terminal in row n from the programs that control Wormhole ("Hide
//!                   from Agents"), or show it again; `state` marks it `hidden`. A question about a terminal
//!                   the program did not open is answered with `agent-ask` too (no Always for it);
//!                   `agents` lists it as `asking … about=terminal:<id>`, and each approved program with its
//!                   `caps=read,type,run,open`
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
//!   group <name> <rename|pin|ungroup|close|move|new|only-in-folder|any-folder|color:<name|none>|folder:<path|none>>   what the group's
//!                   menu does: make its name a field on its head (`text <new name>` then `key enter`
//!                   types and keeps it, `key escape` drops it), pin or unpin, take apart, close (asks
//!                   if an agent works in it), move to a window of its own, colour it (grey, blue, red,
//!                   yellow, green, pink, purple, cyan or orange), or set the folder its new terminals
//!                   start in (an absolute path without spaces, `~/` the home folder; `none` clears it,
//!                   as Set folder… and Clear folder do; `rows` shows it as `folder=…`), start a terminal in
//!                   it as its + does (`new`), or switch "Only terminals in this folder" on or off
//!                   (`only-in-folder`, `any-folder`). A click on the
//!                   group's name (`click <x> <y>`) opens its popover (`popover`), a right-click its
//!                   menu (`menu`)
//!   group-ask <use [move]|later>   answer the question a + on a group without a folder asks (a sheet,
//!                   when the group's terminals point to a folder to suggest): `use` is Use Folder,
//!                   `use move` also ticks "Only keep terminals in this folder in the group",
//!                   `later` is Not Now (`rows` shows `folder-declined`). `group <name> new` is that +
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
//!   onboarding-tailor <n|back|over|ok>   in the Look step's "How much to show" walk: answer with number n
//!                   (from 1, as the digit keys do), go Back, Start over, or accept the summary ("Looks good")
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
//!   import-scan [<fixture home>]   Settings → Import over a folder standing in for the home (`~/…` is the
//!                   instance's; `tools/visual/fixtures/import/`; never the real one), shown and open; without a folder the page scans the
//!                   instance's own home. Shot it with settings-shot in a later batch
//!   import-open <ghostty|iterm2|…>   open that terminal's card on the Import page
//!   import-apply <ghostty|iterm2|terminal-app|alacritty|kitty|wezterm|warp|zed|vscode>   press "Import
//!                   selected" on that terminal's card, rows ticked as shown (settings change, colours are saved
//!                   as theme files in the instance's data dir)
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
//!   probe-approve <id>   Settings → Extensions: open the approval sheet for that Probe and press Approve with
//!                   every permission on, as a person would (a Touch ID level one asks `WORMHOLE_AUTH`;
//!                   open the page first with `settings-section extensions`; needs the `probes` flag, set with
//!                   `set feature_flags {"probes": true}`); `state` lists each Probe as `probe <folder> <state>`
//!   probe-off <id>  turn that Probe off, with no question: its approval and its copy go
//!   settings-scroll <px>   scroll the settings page (negative = up)
//!   settings-resize <w> <h>   the settings window's size (a tall one shows a whole page in one shot)
//!   settings-select <scrollback|icon-theme>   open that row's select in the settings window
//!   settings-reset-confirm   press Advanced → Reset to defaults… (shows its question, resets nothing)
//!   settings-cli-tool [install|uninstall]   Coding agents → Command line tool: show it, or press its
//!                   Install / Uninstall (into the instance's own cli-bin folder, `WORMHOLE_CLI_DIR`)
//!   panel-tab <changes|history|notes|usage|project|files>   show that tab of the right panel
//!   files           the Files tab as text: its folder (following the terminal, or chosen), the pinned
//!                   folders and each row (`v` open folder, `>` closed, `~` link, `*` selected)
//!   files-root <path>   the tree is of that folder, as chosen in the path menu (an absolute path)
//!   files-follow    Follow Terminal: the tree is of the terminal's folder again
//!   files-expand <relative path>   open that folder and the ones above it
//!   files-menu [model]   open the path menu, drawn in the window for `shot`; `model` only records it
//!                   for `menu` and `menu-pick`
//!   files-pin <path>   pin that folder, or unpin it; says which
//!   files-open <relative path>   show that file in the viewer, as a click on its row does (`files` then
//!                   lists a `viewer:` line: what was read, and whether it is colored yet)
//!   files-rename-start <relative path>   turn that row's name into its rename field (a `renaming:` line in
//!                   `files`), for a `shot`; renames nothing
//!   files-rename <relative path> <new name>   rename that row as typing the name and pressing ↩ does. The
//!                   path has no space; the name is the rest of the line. Refused unless this is a test
//!                   instance whose data dir and folder are both inside a temp folder: never a real one
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
//!   commit <n> | commit-file <n>   open or fold the n-th commit's files / open the n-th file's diff
//!   commit-details   the open commit's details block as text: subject, body, author, committer (when it
//!                   differs), co-authors, parents, refs and the size of the change; `loading` or the
//!                   reason it `failed`
//!   commit-more     "Show more" / "Show less" under the open commit's message (`commit-details` says
//!                   whether it is `compact` or `whole`; another commit opens compact)
//!   commit-copy <hash|short|message|subject|author> [n]   the commit menu's copy item, for the open
//!                   commit or the n-th of the log (the message and author are read first when need be)
//!   clipboard       the text the app's last copy put on its clipboard (`empty` when there is none): a
//!                   background instance's own pasteboard, since every copy of the app goes through
//!                   `platform::clipboard` and none of a test instance's touches the system's
//!   diff-scroll-x <px>  scroll the open diff's code sideways (negative = back)
//!   frames <n>      draw the window n times (the workspace is not marked dirty) and append the timings
//!                   to <dir>/frames.txt (out.txt is rewritten per batch); run it once the tab
//!                   under test has finished loading
//!   refresh         drop every view GPUI keeps between frames, so the next draw builds the whole window
//!                   (`shot` after it, against a `shot` before it, shows whether a part replayed from
//!                   an earlier frame has gone stale); `state` ends with how many times each of the
//!                   window's cached parts (sidebar, panel, title row, status bar) has been built
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
//!   pointer | pointer <x> <y> | pointer off   print where the pointer is; or say where it is (points
//!                   from the primary screen's top left):
//!                   a terminal dragged by its row out of the window with `mouse-down`, `mouse-drag` and
//!                   `mouse-up` is let go there (the Tear off feature); `off`: where it really is
//!   tear-off <row> <x> <y>   the terminal in that row is let go outside the window with the pointer
//!                   at that point of the primary screen (as `pointer` + a drag's release): a window of its
//!                   own opens there, or the Wormhole window under it takes the terminal (its list gets
//!                   it last); nothing without the feature, or for the last terminal of a window
//!   notice <text>   put a notice (the chip that drops from the top edge of a terminal) in this window, about its
//!                   selected terminal, that says <text>
//!                   (the Notices feature flag must be on: `set feature_flags={"notices":true}`)
//!   notice sticky [<text>]   put a sticky notice: no life, it stays until closed, and while it shows the
//!                   panel takes clicks. Bare, it is the real warning that a terminal's helper (session
//!                   host) couldn't start, with its own words, its ⓘ explanation and a sample error;
//!                   with <text>, a warning that says that, which has a × and no ⓘ. Dismissed once, the
//!                   same one is not said again, except by `notice sticky again`
//!   notice-click [close|info|body]   click this window's chip as a person does (the click goes over the
//!                   panel's own channel): the × (the default) dismisses a sticky notice, the ⓘ and the
//!                   capsule open its explanation (the `popover` command shows it; in a background instance
//!                   it is recorded and not shown), a copy's chip takes no click
//!   notices         each window's chip, a line each: its number, phase (Hidden, Showing, Leaving), words,
//!                   `info` and `close` (the buttons it shows), `info-open` (the explanation is open),
//!                   `sticky` (one is up), `clickable` (it shows the sticky one's own words) and `hidden` (the
//!                   sticky one waits out of sight: its terminal is off screen or an overlay covers the window), `look <appearance> [plain]` (what the chip was
//!                   made in; `plain`: no transparency), the
//!                   `terminal x y w h` it is about (its pane, in the window's points) and, while the
//!                   native panel is up, `frame x y w h` (the chip at rest, in the window's points),
//!                   `material` (glass, frosted or solid), `mouse` (the panel takes the mouse: only while
//!                   the chip is clickable), `panel <n>`, the panel's window number, and
//!                   `window <n>`, the app window's: the chip is no part of a `shot`, so capture either
//!                   with `screencapture -l <n> -o file.png`
//!   notice-material glass|frosted|solid|system   make the next chip of that material (`system`: as the
//!                   Mac allows), to see all three on one Mac
//!   grow-hold <t> | grow-hold off   a window growing from a torn-off terminal's chip stands at that
//!                   point of its growth (0.0 the chip, 1.0 the window) and stays, for a `shot`; `off`:
//!                   it grows on. Send it before the `tear-off`: the new window reads its own commands
//!                   only on its tick, after the growth is over
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
//!   zen [on|off|toggle]   View → Zen Mode: the title row down to the name and the splits, no status bar,
//!                   minimap or column lines, and no right panel, in every window, with nothing saved;
//!                   without one, whether it is on (`state` ends each window's layout line with `zen=true|false`)
//!   reduce-transparency [on|off|system]   say Reduce Transparency is on or off (every window
//!                   opaque, no blur, the note in Settings), or follow the system again
//!   increase-contrast [on|off|system]   say Increase Contrast is on or off (the theme's
//!                   high-contrast variant, opaque windows, a 3 pt focus ring), or follow the system
//!   language [<tag>|system]   speak that language (`en`, or `en-XA`, the pseudo-language: accented,
//!                   40% longer, in `⟦ ⟧`, so a label with no accent or bracket is hard-coded and a cut
//!                   closing bracket is a truncation; it exists in devctl builds only) or the system's
//!                   again, and redraw every window; without one, the language in use (`state` names it too)
//!   a11y [live|said]   the accessibility tree VoiceOver gets (woken first, as an assistive app does),
//!                   one node a line: role, label, states, `*` on the focused one; `live`: one frame
//!                   drawn as the display would, replaying what GPUI keeps, and its tree (after an
//!                   `a11y`); `said`: the announcements made lately (`!` before one that interrupts)
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
//!   delay <ms>      the rest of the batch waits that long, this window drawing a frame every 16 ms,
//!                   so a `shot` after it catches an animation part of the way through: `hook 2 Stop`,
//!                   `delay 80`, `shot mid`
//!   sync <token>    write `sync <token>` to the results file once everything before it in the
//!                   batch is done (after `wait-idle`, `bounds` and `settings-shot`; not `frames`),
//!                   as the last line: a script waits for it before reading the results

mod a11y;
mod agents;
mod ambient;
mod batch;
mod blocks;
mod commands;
mod control;
mod files;
mod flow;
mod hosts;
mod input;
mod language;
mod menus;
mod notices;
mod panels;
mod probes;
mod profiling;
mod scaling;
mod sessions;
mod settings;
mod shots;
mod sidebar;
mod studio;
mod tear_off;
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
