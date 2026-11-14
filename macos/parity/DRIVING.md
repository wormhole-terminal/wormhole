# Driving a background instance

What a script can do to a `--background` Swift instance through its control folder, and what still needs a person. For the
lanes checking their rows and for the verification wave. The commands follow GPUI's devctl
(`crates/wormhole/src/devctl/mod.rs`) where the Swift app has the same thing.

## Setup

```sh
WORMHOLE_DATA_DIR=/tmp/Wormhole-swift-<x> WORMHOLE_SHELL=/bin/zsh WORMHOLE_AUTH=cancel WORMHOLE_DEVCTL=<ctl> \
  macos/swift/.build/release/Wormhole --background &
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
| `scroll x y dy [dx]` | `scroll 700 400 -400`, `scroll 700 400 0 600` | the wheel there, dy points down (negative up), dx right (negative left), in steps of 20 a frame apart |
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

Added by round S2 (overlays): `pasteboard` prints what the instance "copied" and `pasteboard-set <text>` sets what a paste finds: a
`--background` instance's clipboard is a file in its data folder (`DesignSystem.Pasteboard`), never the person's, and every copy and paste
site goes through it. `open-folder <path>` answers a pending Open folder… (⌘O, the palette, the status item), which a test instance
shows no panel for. `row-menu [terminal]` is ⌃↩ on a sidebar row (SwiftUI's own key handling never receives a key from outside): the native
menu is kept for `menu` and `menu-pick`. `key backspace|delete`, `key cmd-+` and the ⌃⌥ keys work; ⌘C and ⌘⌫ in a text field are the field's
(the palette's own), and `key cmd-c|x|v` refuse in a field (the system's would use the person's clipboard). `keys` also names the
first responder. A `--background` instance holds questions for the control folder only when it has one (`WORMHOLE_DEVCTL`). With
`WORMHOLE_SHELL` set, core runs that program whatever shell a terminal asks for: use `--shell` to try New terminal: bash.
`evidence/s2-overlays/` has the fixture (`fixture-e.sh`: three agent-named programs, a trusted project, hooks), `pair.py` and `menu-keys.py`.

Landed by other lanes on `exp/swift`: `question`, `question confirm|cancel|also` (a background instance never shows a
question sheet; the control folder answers it), `studio-out`, `intro`, `welcome`, `onboarding-step`, `tear-off`, `tear-off-at`,
`profile-menu`, `minimise`, `place`, `hold-cmd`, `release-cmd`, `look-keys`. Injected clicks never need a sheet on screen.

Added by wave 5 (dev tooling, GPUI devctl's remaining commands; written without compiling, see TO-VERIFY.md):

| Command | Example | Does |
|---|---|---|
| `inspect` / `inspect on\|off` | `inspect on` | inspect mode (⇧⌥⌘I in a debug build): the Inspector panel opens beside the window and every element the design marks reports where it is |
| `inspect x y [levels]` | `inspect 150 66 3` | pins the marked element at a point: its note (`File.swift:line  id`, the line of code in a debug build, `W×H · style`, `tokens: …`), then `levels - 1` parents |
| `inspect parent` / `inspect pick` / `inspect copy` | `inspect copy` | walk out to the element it stands in; pick again (then `hover x y` highlights and `click x y` pins, as in GPUI); the panel's Copy (read it with `pasteboard`) |
| `inspect nudge <property> <+\|->` / `inspect reset` | `inspect nudge padding x +` | debug builds: steps the scale value that part of the pinned element's style names (4 pt, a radius 2 pt), for the session; it shows wherever that value is used, and the note gets `change: padding x _1 → _2`. Reset takes every step back |
| `detach` / `reattach` | `detach` | let go of the front window's hosted terminals, the views kept (`detached <n>`); attach them again as a launch does (their ids). `window-state` shows `detached(hostID:)` between |
| `agents` | | the script in force, each approval (`approved stored\|memory:<id> <label> <how> caps=…`), how many questions wait and the one that is up with its buttons |
| `agent-ask <deny\|session\|quit\|always>` | `agent-ask session` | answers the question that is up as that button does (a button it does not offer comes to Quit, else Deny); for the question about a changed mode, deny is Don't Keep |
| `agent-script <hold\|deny\|session\|quit\|always>` | `agent-script quit` | what answers programs' questions from now on, with no sheet; `hold` leaves them for `agent-ask` |
| `agent-grant scripts [quit\|always]` | `agent-grant scripts` | lets Scripts in, as an answer to its question does (until quit, or stored for the profile) |
| `menubar-icons [prefix]` | `menubar-icons bar` | the status item's glyph in its nine states as `<ctl>/<prefix>-<state>.tiff` (a background instance has no status item to look at) |
| `studio-select <token>` / `studio-set <value>` / `studio-type <text>` | `studio-set #ff8800` | select a token; put a value in its field with Return (`ok=`, `field=` the refusal); type without Return (what the field says) |
| `studio-save [name]` / `studio-rename <name>` / `studio-undo` / `studio-redo` / `studio-revert` / `studio-use` | `studio-save Mine` | ⌘S (a name first for a copy without a file); the title row's and Edit menu's verbs |
| `studio-filter <all\|yours\|base\|low> [text]` / `studio-area <area>` | `studio-filter low` | what the token list shows; the gallery's part of the app |
| `studio-appearance light\|dark` / `studio-add-appearance [light\|dark]` / `studio-chip [token]` | `studio-chip primary` | the appearance of a pair being edited; add the one it lacks; move a value between both appearances and this one |
| `studio-update-from [apply\|cancel]` / `studio-versions [select <n>\|restore [<n>]\|close]` | `studio-versions select 1` | the Update from sheet; the saves behind the file (row 0 is the file now): preview one in every window, restore one as unsaved edits |
| `studio-menu [save\|duplicate\|json\|project]` / `studio-theme-menu` / `studio-open own:<stem>\|builtin:<name>\|from:<name>\|new\|project` | `studio-open builtin:Nord` | the Save menu's entries (`json` writes `<data dir>/studio-out/pasteboard.txt`); what the theme menu offers; choose in it |
| `studio-resize <w> <h>` / `studio-scroll <y>` / `studio-shot <name>` / `studio-frames <n> [tick\|scroll]` | `studio-frames 40 tick` | the Studio window: its size, the gallery scrolled to y points from its top (by the wheel; `at=` where it got to), `<ctl>/<name>.png`, n timed draws to `frames.txt` in GPUI's words (`studio-frames <mode> n= avg= max= first=`, the first three left out of avg and max): `tick` lays the window out before each draw, `scroll` turns the wheel 40 points first. `state` ends with a `studio dirty= asking= subject= undo= redo= format= shown= update= modal= inuse= token= origin= reach=` line while the Studio is open |

A `studio-*` line that may ask (saving over a file, opening over unsaved edits) replies when it is done; answer its question with
`question confirm|cancel` in a later batch. `perf-profile` now also prints draws per second, the Rust side's share of a fetch
(snapshot, pack), where the window's terminals run, and the reader's bytes, speed and lock use: the last only from a bridge built
with its `perf-counters` feature: `WORMHOLE_FFI_PERF_COUNTERS=1 macos/scripts/build-ffi.sh` (off by default; otherwise it says the reader was not counted). `agent-*` and `inspect` never reach the control
socket: they are the control folder's alone.

Added by wave 6 (dev tooling; written without compiling, see TO-VERIFY.md): **each window has its own control files**, as in GPUI:
`cmd` acts on the window in front (the key one, else the one opened or used last) and is answered in `out.txt`; `cmd1`, `cmd2`,
`cmd3` … are the windows in the order they were opened, whatever is in front, answered in `out1.txt`, `out2.txt` … A line of `cmd<n>` acts on that window, keys included (`key`, `keys`, `text`, `state`, the window commands).

| Command | Example | Does |
|---|---|---|
| `blocks` | `blocks` | the selected terminal's command blocks, oldest first, one a line: `n: lines a..=b output c..=d \| state \| command` |
| `block <n> <select\|copy-output\|copy-command\|to-prompt\|fold>` | `block 2 copy-output` | the Edit menu's action on the n-th block of `blocks` (`block` alone still says which block the menu would act on) |
| `images` | `images` | the pictures the last frame placed: a total, then each picture (key, size, bytes or `decoding`) and its placements |
| `pane-bar` | `pane-bar` | what the bar and banner over the selected terminal say (a clipboard question, a notice, a host's trouble), or `none` |
| `panes` | `panes` | the shown split's panes (`pane id x y w h [focused] [zoomed]`) and dividers, and the layout in force |
| `layout <columns\|rows\|grid\|single>` | `layout grid` | View ▸ Layout |
| `shells` / `new-shell <key>` / `new <dir>` | `new-shell fish` | the shells on offer (`-` is the login shell); a terminal running one; a terminal in a folder (`~/` expands) |
| `close <n>` / `agent-hide <n> [on\|off]` | `close 2` | row n (1-based, as the sidebar lists) closed at once, without the question; Hide from Agents |
| `zen [on\|off\|toggle]` | `zen on` | Zen mode (nothing saved); says whether it is on |
| `files` / `files-root <abs path>` / `files-follow` / `files-pin <abs path>` / `files-rename <path> <name>` | `files-root /tmp/x` | the Files tab as text (`v` open folder, `>` closed, `~` link, `*` selected) and what its path menu, pins and rename do |
| `delay <ms>` / `sync <token>` | `delay 300` | the rest of the batch waits; `sync` answers `sync <token>` once every line before it has run |
| `control [--from <tab>] <words…>` | `control --from 3 list --json` | a request as `Wormhole <words…>` sends it, as from inside tab 3 (none: outside the app); the reply's JSON line. Reads, waits, runs and streams are not carried here (use the socket) |

Added by wave 6, second pass (dev tooling, GPUI's remaining commands; written without compiling, see TO-VERIFY.md):

| Command | Example | Does |
|---|---|---|
| `note-seed [count] [lines]` / `note-toggle <n>` | `note-seed 50 12` | that many notes of that many lines (GPUI's text); fold or unfold the n-th note |
| `task-seed [open] [done]` / `task-done <n>` | `task-seed 38 12` | that many next steps, the done ones spread among the open; tick or untick the n-th |
| `send` / `send-submit` / `stash` | `send-submit` | the focused note (the walker's, else the newest) goes to the terminal (and Return); the prompt typed in an agent's box becomes a note |
| `comment <text>` | `comment too long` | the comment dialog a click on a diff line opened: text set, saved, closed; says the review's summary (`1 comment`) |
| `changes` | `changes` | the Changes tab on show (GPUI's `changes`) and what its store holds: `changes phase=loaded files=1 open=README.md diff=shown hunks=1`, then a line a file in the list's order, `  M README.md +1 -0` (the kind's letter, the path, lines added and removed, `-` where git counts none) |
| `compare [branch]` / `picker [history\|compare] [query]` | `compare main` | the Changes tab compared with a branch (none or `HEAD` stops); the revisions the picker offers for the query, one a line |
| `commit-details` / `commit-more` | `commit-details` | the open commit's block as text (`none` when none is open); Show more / Show less |
| `project-init` | | the Project tab's Create project folder button: the project is made in the folder the tab is for, a repository's root when the terminal is in one, and that folder is named (`creating <dir>/.wormhole`); `folder-color` without a path colours the same folder |
| `onboarding <welcome\|folder\|look\|agents\|primer\|close>` | `onboarding look` | the intro window at that step, or closed, as the GPUI app's devctl (`onboarding-step` is the same; a bare `onboarding` is Help ▸ Welcome) |
| `history <n>` | `history 1` | a click on the n-th commit of the log: it opens, and closes when it is the open one; `history 0` folds whatever is open |
| `group-ask [use\|use only\|later]` | `group-ask use only` | the question a terminal opening in a group without a folder puts up (from the sidebar's +, the tab bar's +, an empty set or Ctrl-Opt-N; held for the control folder in a test instance): without an argument the folder, the group and how many terminals would leave; `use` is Use Folder, `use only` also ticks "Only keep terminals in this folder", `later` is Not Now. There is no Cancel; the terminal opens after the answer |
| `rename-edit <n>` | `rename-edit 2` | row n's name becomes a field, as Rename… in its menu makes it |
| `folder-color [path] <name\|#rrggbb\|none>` | `folder-color ~/Projects/x teal` | colours the folder (the terminal's without a path) as the band's menu does; says `<colour> <folder>` |
| `folder-picker open\|close\|type <text>\|tab\|right [mid]\|left [start\|mid]\|accept-ghost\|up\|down\|pageup\|pagedown\|enter\|pick\|forget\|click <line>\|crumb <n>\|scroll\|state` | `folder-picker type src/` | the folder picker (⌘J) over the terminal: open it (the Folder picker flag must be on), type into its field, press its keys, pick; `state` lists text, ghost, `busy`, crumbs, facts, lines (`> ` marks the selection) and the panel. The queries run off the main thread: ask `state` again while it says `busy true`. `scroll` is a no-op here |
| `profiles` | `profiles` | the core's profile list (what Settings' Profile page reads, so it works with one profile), one a line: `* id "name" "colour"`, `*` for the one Wormhole is in |
| `profile-open <id>` / `profile-move <id>` / `profile-delete <id>` | `profile-open work` | a window in that profile (the odd window when Wormhole is in another); the window in front goes there; delete as Settings does after its question (TEST: scratch data dir only) |
| `notice sticky [text]` | `notice sticky` | a warning that stays until clicked: the session host's, with `text` in its explanation (the chip's own words are that warning's) |
| `notice-click [close\|info\|body]` | `notice-click info` | a click on the chip: the × dismisses a sticky one, the ⓘ and the capsule open its explanation (or dismiss it where it has none); a copy's chip takes none. `error: no chip to click` without a chip |
| `notify-sim <kind> <n> [post]` | `notify-sim needs-you 1 post` | what the core's rules decide for that event in row n (`needs-you`, `agent-done`, `agent-failed`, `command-done`, `command-failed`, `bell`, `notice`): `-> Banner\|Nothing level sound bounce title subtitle body`; with `post` also delivered to the instance's log (`notice-log`). `notices` keeps its Swift meaning (the logged banners); GPUI's per-window chip listing is `notice chip` |
| `notify-permission <not-asked\|allowed\|denied\|outside\|checking>` | `notify-permission denied` | pretends macOS said so, for the welcome's and Settings' pages (a test instance never asks the system) |
| `hosts-toggle [id]` / `hosts-scroll <points>` | `hosts-toggle` | open or fold a host's card in the Hosts window (the first without an id); scroll its list to that offset from the top (the window must be open: `hosts`) |
| `licenses-search <query>` / `licenses-select <name>` | `licenses-select serde` | filter the Licenses list (says the count and the selected package); open a package (a search that hides it is cleared) |
| `intro-frames <style> [count]` | `intro-frames voyage 8` | `count` frames (8, at most 60) of the intro drawn off screen over a stand-in window, to `<ctl>/intro-<style>-<n>.bmp` |
| `onboarding-tailor <n\|back\|over\|ok>` | `onboarding-tailor 2` | in the Look step's "How much to show" walk: answer n (from 1), Back, Start over, or Looks good (`onboarding look` first) |
| `welcome-probe <tab\|-> <token> <op as JSON>` | `welcome-probe - T {"op":"state"}` | a welcome request as the first-run program sends it, from that tab (`-`: from outside any); the reply's JSON line (TEST of the token's rules: a wrong token must be refused) |
| `update-installed <version> [pending]` | `update-installed 9.9.9 pending` | Settings → Updates as after an install: Restart, or with `pending` the line that it restarts when the terminals are idle (the real updater only; the next change it reports replaces it) |
| `import-scan [<fixture home>]` | `import-scan ~/fixture` | Settings → Import over a folder standing in for the home (`~/` is the instance's, never the real one), shown; waits for the scan and lists what it found |
| `font-picker <interface\|code\|terminal> [query]` | `font-picker code mono` | opens that font picker with the query typed (the Fonts page on show: `settings-section fonts`); says the entries and the family previewed |
| `settings-record <row id> [keys]` / `settings-reveal <id>` | `settings-record hotkey cmd-k` | a shortcut row's recorder started (a click on its row) and keys pressed into it; open Settings at a row, scrolled and marked |
| `settings-reset-confirm` / `settings-cli-tool [install\|uninstall]` | `settings-cli-tool install` | Advanced → Reset all settings pressed (its question is held: `question`, then `question cancel`; nothing resets); Coding agents → Command line tool: what it says, or Install / Uninstall into the instance's `WORMHOLE_CLI_DIR` |
| `menu-native [press <n>]` | `menu-native` | the kept menu (a right-click's, a key's) with each item's key, marks and tooltip, and a row of colour dots with their names; `press <n>` presses the n-th dot as a click on it does (the menus are native already: `menu`, `menu-pick`, `menu-close` read and press them) |
| `path-menu` / `files-menu [model]` | `path-menu` | the folder's menu (title row) / the Files tab's path menu, kept unshown for `menu` and `menu-pick` and printed |
| `title-menu` | `title-menu` | the title row's own menu (a right-click on the row, not on its folder), kept unshown for `menu` and `menu-pick`; the first line is the row on screen (`row=folder,branch,…`) |
| `available <action>` | `available toggleSidebar` | whether the bar's item for the action is on for the window in front (an action with no item: whether the action table runs it): `<action>: true\|false` |
| `popover` / `popover-press <title>` / `popover-close` | `popover-press Ungroup` | the open popover (the group's name popover; the folder's hover card is `path-card`): its rows one a line (`field "Group name" (focused)`, `colours: No colour \| Red [x] \| … \| custom`, `- action`); press an action by its title as a click does; close it |
| `popover-type <text>` / `popover-key <enter\|escape>` / `popover-colour <name\|none\|#rrggbb>` | `popover-type Build` | type into its field (replacing what is there); ↩ keeps and closes, ⎋ closes (the Swift popover keeps the name either way, as its own close does); colour the group by name or hex |
| `popover-native [name]` / `popover-real` / `popover-real-close` | `popover-native card` | the open popover's content drawn to `<ctl>/<name>.png` as AppKit lays it out; opened for real as an `NSPopover` at the window's top left, and closed |
| `drag <x> <y> <path>` / `drag-end` | `drag 600 400 /tmp/a.png` | a file hovering over the window at a point, as from Finder: the terminal there tints (`window-shot` in a later batch shows it); `drag-end` abandons it. The drop is `drop-files <path>` (pastes the paths as a drop does). GPUI's `drop <x> <y>` keeps its Swift meaning (a sidebar drop, see `drop`/`plan`) |
| `close-window` | `close-window` | the window the command acts on closes as its red button does: it asks first while something runs (a test instance holds the question: `question confirm`), and the last window ends the app |
Added by wave 9 (lane commands; written without compiling, see TO-VERIFY.md): **read-outs** of what a view holds, for the
end-to-end tests. Each answers in one shape, `DesignSystem/Driving/Readout.swift` (`Readout.line`), which the tests' own
`Tests/EndToEndTests/Readout.swift` parses: a line a thing, `kind word… "text"… key="value"… flag…`, a line under another
indented by two spaces. A quoted text never holds a straight quote or a line break (written `”` and `⏎`). GPUI's devctl has
none of these; `status-bar` is the bar's switch, so its items are `status-items`.

| Command | Reply, line by line |
|---|---|
| `settings-page` | `settings-page <section\|results> "<title>" query="<q>"`; a `nav <id> [badge="<n>"] [attention] [dim]` per page of the nav; `banner <info\|warning\|danger> "<text>"` per banner over the page; `group "<title>"`; `row <id> "<label>" value="<words>" [chip="<tag>"] [project="<name>"] [group="<g>"] [changed] [off] [found] [revealed]` (on the Changed page and in results, every row); under a row: `caption "<text>"`, `note <info\|warning> "<text>" [button="<title>"]`, `swatches <#hex>… chosen="<#hex\|->" [well]`, `problem "<text>"` (a shortcut's), `option "<label>" "<detail>"` (a menu's subtitled choice) |
| `theme-cards [filter]` | `theme-cards mode="<theme_mode>" filter="<f>" placeholder="Filter N themes"`; `project "<name>" "<notice>"`; per shelf `shelf <dark\|light> "In Dark Mode" total="<n>" collapsed="4"` and under it `card "<band label>" value="<value>" [newer="<n>"] [file="<path>"] [picked]`; `none "No themes match the filter."`; `hidden "<name>" kind="<Dark\|Light\|->" value="<value>"` |
| `status-items` | `status-items detail="<words\|numbers\|lean>" width="<pt>" <shown\|hidden>`; `profile "<name>"`; `attention <id> "<title>" words="needs input · 2m" tooltip="…" [spoken="…"]`; `count <activity> <n> first="<id>" words="<2 working\|2>" [spoken="…"]`; `probe "<text>" tooltip="…"`; `state "<working · 4m>"`; `hosting "<word>" tooltip="…"`; `size "<program> · <cols>×<rows>"` (the bar's order; at any width, the bar on or off) |
| `tab-bar` | `tab-bar set="<label>" alert="<none\|attention\|failed>" <shown\|hidden> [switcher]`; `tab <id> "<name>" activity="…" alert="…" [number="<n>"] [spoken="…"] [selected]`; `plus "New terminal"`; `tabs <on-top\|in-sidebar>` |
| `tab-click <n\|+>` | `tab-click <n>` once the n-th tab (from 1) is clicked (its terminal selected and given the keys), `+` the bar's + (the group folder question first, as the sidebar's); `error: no tab <n> of <m>` |
| `switcher [query]` | `switcher query="<q>" placeholder="…" [hidden]`; without a query `card <index> "<label>" count="<n>" tiles="<id,…>" activities="…" alert="…" spoken="…" [add="New terminal in <g>"] [current] [pinned] [group]`, `  empty <index> "No terminals"`, `footer "New group" "Show all groups in the sidebar" keys="⌃⌥[ / ] previous, next group" range="⌃⌥1–9"`; with one `hit <set\|terminal> <index\|id> "<label>" note="…" mark="<dot\|tile\|none>"` or `none "No matches"` |
| `onboarding-state` | `onboarding-state <welcome\|folder\|look\|agents\|primer> number="<n>" folder="<~/…\|->" [first-run] [last]`; `card "<title>" "<intro>" counter="<n of 3>"`; `button <back\|main\|skip> "<title>"`; `place <recent\|place> "<name>" detail="<~/…>" [picked]`; `agent <id> "<name>" hooks="<…>"`; `agents "Looking for agents…"`; `trust "<has>" "<waits>" button="<title>" later="Later" root="<path>" [approving]`; `primer "<thing>" keys="<glyphs>"`; `problem "<text>"` (error: the intro window is not open) |
| `onboarding-pick <path>` / `onboarding-press back\|main\|skip` | a folder dropped on the card (`onboarding-pick <path>`); the card's buttons (main: Continue, Start on the last step) |
| `changes-menu <n\|path>` / `history-menu <n\|details>` / `files-row-menu <path>` | the menu a right-click on that row opens, found at the row's own frame and kept for `menu-pick`: `menu:` then `AppController.rows(of:)`'s lines (`n. title [key] [x] [off]`, `---`, a submenu's items indented). The row must be on screen (its tab shown): ask again until it answers `menu:`; `error: no row … on screen` before |
| `diff-header` | `diff-header <changes\|history> "<path>" [position="<i> of <n>"] [commit="<short>"] [counts="+a −r"] [split] [wrap] [gone]` (`"none"` with no diff open); `badge "<words>"` per badge in the header's order; `out-of-date "<why>"`; `submodule old=… new=… ahead=… behind=… listed=… [nothing=…] [inside=…]` |
| `diff-step next\|prev` | the header's arrows: `diff-step <dir>` |
| `diff-spans [from] [count]` | `diff-spans unified rows="<n>" [colored-by-core] [no-highlight]` (or `diff-spans "none" phase=…`); then from row `from` (0) `count` rows (60): `hunk <row> "<@@ …>"`, `line <row> <context\|added\|removed> old="<n\|->" new="<n\|->" text="<line>" spans="<start>-<end>:<token>,…"` (Characters of the text; empty: plain) |
| `settings-press <id> [menu]` | a click at the centre of the Settings control recorded under `id` (`settings-rows` lists them: a row's control by the row's id; `changed.copy`, `import-theme`, `open-studio`, `reset-tips`, `open-in.choose`, `zen.leave`, `theme-card.<value>`, `agent.<id>.hooks`, …), the way `click` makes one: `settings-press <id>: <press>, <release>`; with `menu` a right-click there: `menu:` and its lines, kept for `menu-pick` |
| `settings-page` (wave 9, second pass) | also, under the title row's row, `  part <value> "<label>" [note="…"] <shown\|hidden> [hideable]`; on Fonts `specimen <terminal\|code\|interface> "<facts line>" family="…" size="<pt>"` per tab and `preview <slot> family="…" size="…"` while a picker looks at a font |

Also wave 9: `key cmd-left|cmd-right|cmd-backspace` (any key the keymap gives a view's own action, `ActionRouter.viewed`) goes the
keyboard's way, to the terminal's `performKeyEquivalent` before the menus; `key esc` in a field goes through the window first, so
SwiftUI's `onKeyPress(.escape)` hears it, and the field's own Esc binding follows only while the field still has the keys; `a11y`
first sets `AXEnhancedUserInterface` and `AXManualAccessibility` on the application, as an assistive client would, so SwiftUI's
hosting views may build their nodes (unverified: see "What still needs a person").

Proof runs: `macos/parity/evidence/s-tooling/` (`pointer.txt`: row click, sidebar drag reorder, a held drag shot mid-way,
column edge, row context menu, rename typed into a SwiftUI field, the wheel; `T15.txt`; `T18.txt`; `hosts.txt`;
`keys-a11y-perf.txt`).

## Tips

- A sheet over the window (the new-worktree sheet, an alert) is the window's own: `a11y` lists it (`Sheet "alert"` with its words) and
  keys and text reach its field (`text feature-x`, `key return` makes a worktree), but a click is the main window's, and `key return`
  does not dismiss an alert: restart the instance after one. `window-shot` does not draw a sheet (the window under it is dimmed).
- Hover never shows in a background instance (the window is not active, so `onHover` does not fire): hover icons (a row's open icon, a
  diff line's) are for a person to check.

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

## End-to-end tests

`macos/swift/Tests/EndToEndTests` is the first suite that uses the app instead of its parts: short scenarios a person would do
(a first window, new/select/rename/close, splits, groups and tabs, the palette, Settings, the right panel, notes, quit and come back,
a worktree, an agent's status, two windows, the Theme Studio's contrast and versions), each asserted on what the app reports through this control folder, never on pixels. One
`Instance` (`Instance.swift`) starts the built `Wormhole` with `--background` on a scratch folder, sends a batch (a `sync <token>`
after every command says it is done: the wait is on the output file's change events and a deadline, never a pause), and always ends
what it started (`quit-end-all`, by its own pid if that does not answer), checks that no session host of its folder runs on (each
host's `hosts/<id>.meta` names its process), and removes the folder.

Persistent terminals: `Instance.with(hosts: true)` links the built session host into the scratch `bin` (the app is told where by
`WORMHOLE_HOST_BIN`; the build's own folder is never touched) and turns the feature's flag on in the scratch `config.json`. The host is
`$WORMHOLE_E2E_HOST`, else `crates/wormhole-ffi/target/debug/wormhole-host` (`cargo build -p wormhole-host` there, as `swift.yml`'s
"Session host for the hosting tests" does); the hosting scenarios (`HostingTests.swift`) skip without it.

Run them where the bridge is linked (`macos/scripts/build-ffi.sh swift`, then in `macos/swift`):

```sh
swift test --filter EndToEndTests               # builds the app too; WORMHOLE_E2E_APP=<path> runs another build of it
swift test --filter EndToEndTests/firstWindow   # one scenario
```

The suite skips itself, saying why, when no `Wormhole` is built. A failure attaches the control files and the app's output to the
test's result; with `WORMHOLE_E2E_ARTIFACTS=<folder>` set (CI does) it also copies them, with `config.json` and the logs, there before
the scratch folder goes. `swift.yml` runs the suite as a step of its own after the other tests and uploads that folder on failure.

To add one: a test in `extension EndToEnd` (one file per theme, `@MainActor`), `try await Instance.with { app in … }` (a shell
other than `/bin/cat`: `shell: .zsh` or `.agent("codex")`), commands through `app.send([...])`, `app.reply("…")`, `app.rows()`, and
`app.until("command") { reply in … }` for a fact nothing announces (it asks again with the app's own `delay` between asks and a
deadline). Read the command's real reply in its `AppController+Control*.swift` before asserting on it, and ask for a command that does
not exist instead of inventing one. A repository comes from `app.scratch.repository(…)`. Without `window:` a batch goes to `cmd`,
which acts on the window in front (the newest, once a second one is open); `app.send([...], window: 1)` writes `cmd1` and reads
`out1.txt`, the first window's own, and `window: 2` the second's.

Every instance gets one window size (`Instance.window`, 1600 x 900 points: 100 columns beside both side columns, and room to
split), asked with `size` once the window answers and checked with `geometry`, so no scenario depends on the display it runs on:
a size asked through the control folder holds where the screen is smaller than the window (AppKit would fit the window to the
screen: on a 1024-point display the terminal was 304 points wide, a split was refused and text wrapped at 35 columns).
The Studio's `studio-open`, `studio-save` and `studio-use` answer when they are done, beside the batch: read them with `app.late(…)`.
`a11y` lists AppKit's views only, so a SwiftUI button's spoken name is a known issue there ("What still needs a person").

Wave 8 (lane e2e-b) added: `Instance.with(auth: "allow", config: "<json>")` (what the owner's question answers, a `config.json`
before the first launch), `app.cli([...])` and `app.cliRun([...])` (the app's own command line asking the instance through its
socket, as an agent or a script would: `Client.swift`), `app.later(command, answer: prefix)` for a command that answers from a task
after its batch's `sync` marks (`project …`, `update`, `probe`, `studio-save`, `studio-open`, `studio-menu`, `history-copy`,
`studio-out`), `app.flags([...])`, `app.setting(key)`, `app.terminal(in:)`, `app.screen(showing:)` and `app.signal(SIGTERM)`.

A terminal of a test instance starts in the scratch `HOME` and its `~` is that folder: the app writes a `~` out with the core's
home (`HomeFolder`, `$HOME` first), so `rows` shows the scratch home's full path where `/tmp` and `/private/tmp` differ.
After `size` the window is laid out at once, and a key sent while nothing in the workspace window has the keys goes to the
selected terminal: a hosted runner's window gets no display cycle and never made the terminal its first responder.

What they may never touch: the person's data folder, `HOME`, agent home, clipboard, Trash, notifications, Touch ID, git repositories,
browser or editor, and the keyboard focus. The harness gives the app an environment of its own (no inherited variable): the data
folder is `Wormhole-swift-e2e` inside `/tmp/wh-e2e-<id>`, `HOME`, `TMPDIR` and `WORMHOLE_AGENT_HOME` are folders in it,
`WORMHOLE_AUTH=cancel` (`Instance.with(auth: "allow")` for a scenario that trusts a project: answered, never prompted), `WORMHOLE_NOTIFY=log`, `WORMHOLE_AGENT_PATH=` empty, git without the person's config. A test never passes a
real path to the app, never spawns a program itself (`Spawn.swift` is the one place `rules.py` allows a `Process`, marked
`// process:`), and never sets the app's isolation variables by hand.
