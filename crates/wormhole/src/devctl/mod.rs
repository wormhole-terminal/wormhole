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
//!   block <n> <select|copy-output|copy-command|rerun>   the Edit menu's action on the n-th block as
//!                   `blocks` lists them (`key cmd-up` / `key cmd-down` jump between the prompts)
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
//!   rename <n> <name>   name the n-th terminal as a project's layout would (as `state` numbers them)
//!   group-new <name> <row>…   a new group of the terminals in those rows (as `state` numbers them);
//!                   prints its id
//!   sidebar-fold <group>   fold or open the group with this name
//!   group <name> <edit|pin|ungroup|close|move|color:<name|none>>   what the group's popover and menu
//!                   do: open the popover, pin or unpin, take apart, close (asks if an agent works in
//!                   it), move to a window of its own, or colour it (grey, blue, red, yellow, green,
//!                   pink, purple, cyan or orange)
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
//!   right-click <x> <y>   a right-click there (a sidebar row's or group's context menu)
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
//!                   apps found, whether the path's hover card is open, and the extras (the branch's
//!                   status, the agent's model and context, the ports, the pull request: what each
//!                   says, or that the row gave it up). A test instance has no `gh` unless
//!                   `WORMHOLE_GH_BIN` names a stand-in (a pull request is asked only in a
//!                   trusted project); a status line is `Wormhole statusline` run with
//!                   `WORMHOLE_SOCKET` and `WORMHOLE_TAB_ID` set to the instance's and the tab's
//!   path-card <open|close>   open or close the hover card under the title row's path at once, as
//!                   resting the pointer on the path does after a moment (`hover <x> <y>` does too)
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
mod blocks;
mod control;
mod flow;
mod input;
mod menus;
mod panels;
mod sessions;
mod settings;
mod shots;
mod sidebar;
mod studio;
mod title;
mod welcome;

use std::collections::{HashSet, VecDeque};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use gpui_kit::*;

use crate::workspace::Workspace;

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

/// The command called `name`, if there is one.
fn command(name: &str) -> Option<Command> {
    Some(match name {
        "shot" => shots::shot,
        "screen" => shots::screen,
        "blocks" => blocks::blocks,
        "block" => blocks::block,
        "images" => blocks::images,
        "state" => shots::state,
        "studio-shot" => shots::studio_shot,
        "settings-shot" => shots::settings_shot,
        "frames" => shots::frames,
        "paints" => shots::paints,
        "type" => input::type_text,
        "paste" => input::paste,
        "drag" => |ctx, arg| input::file_drag(ctx, arg, input::FileDrag::Enter),
        "drop" => |ctx, arg| input::file_drag(ctx, arg, input::FileDrag::Drop),
        "drag-end" => |ctx, arg| input::file_drag(ctx, arg, input::FileDrag::End),
        "hover" => input::hover,
        "scroll" => input::scroll,
        "mouse-down" => |ctx, arg| input::mouse(ctx, arg, input::Mouse::Down),
        "mouse-drag" => |ctx, arg| input::mouse(ctx, arg, input::Mouse::Drag),
        "mouse-up" => |ctx, arg| input::mouse(ctx, arg, input::Mouse::Up),
        "click" => input::click,
        "double-click" => input::double_click,
        "text" => input::text,
        "key" => input::key,
        "menus" => menus::menus,
        "palette" => menus::palette,
        "sheet" => menus::sheet,
        "action" => menus::action,
        "available" => menus::available,
        "about" => |ctx, arg| menus::about(ctx, "about", arg),
        "intro" => |ctx, arg| crate::intro::devctl::play(ctx.window, arg, ctx.cx),
        "intro-frames" => |ctx, arg| crate::intro::devctl::frames(arg, ctx.dir, ctx.cx),
        "licenses" => |ctx, arg| menus::about(ctx, "licenses", arg),
        "about-shot" => |ctx, arg| menus::about(ctx, "about-shot", arg),
        "licenses-shot" => |ctx, arg| menus::about(ctx, "licenses-shot", arg),
        "licenses-search" => |ctx, arg| menus::about(ctx, "licenses-search", arg),
        "licenses-select" => |ctx, arg| menus::about(ctx, "licenses-select", arg),
        #[cfg(any(feature = "inspector", debug_assertions))]
        "inspect" => input::inspect,
        #[cfg(any(feature = "inspector", debug_assertions))]
        "bounds" => shots::bounds,
        "new" => sessions::new,
        "new-shell" => sessions::new_shell,
        "shells" => sessions::shells,
        "activate" => sessions::activate,
        "rename" => sessions::rename,
        "menu-new-window" => sessions::menu_new_window,
        "menu-updates" => sessions::menu_updates,
        "new-window" => sessions::new_window,
        "close-window" => sessions::close_window,
        "rows" => sessions::rows,
        "split-join" => sessions::split_join,
        "split-leave" => sessions::split_leave,
        "row-drop" => sessions::row_drop,
        "pane-split" => sessions::pane_split,
        "pane-close" => sessions::pane_close,
        "pane-zoom" => sessions::pane_zoom,
        "pane-focus" => sessions::pane_focus,
        "layout" => sessions::layout,
        "panes" => sessions::panes,
        "note" => sessions::note,
        "task" => sessions::task,
        "quit" => sessions::quit,
        "profile" => sessions::profile,
        "profile-new" => sessions::profile_new,
        "profile-open" => sessions::profile_open,
        "profile-move" => sessions::profile_move,
        "profile-delete" => sessions::profile_delete,
        "profiles" => sessions::profiles,
        "group-new" => sidebar::new_group,
        "sidebar-fold" => sidebar::fold,
        "group" => sidebar::group,
        "to-group" => sidebar::to_group,
        "rename-edit" => sidebar::rename_edit,
        "pin" => sidebar::pin,
        "folder-color" => sidebar::folder_color,
        "hold-cmd" => |ctx, _| sidebar::hold_cmd(ctx, true),
        "release-cmd" => |ctx, _| sidebar::hold_cmd(ctx, false),
        "right-click" => sidebar::right_click,
        "sidebar-scroll" => sidebar::scroll,
        "resize" => sessions::resize,
        "onboarding" => sessions::onboarding,
        "welcome" => welcome::welcome,
        "welcome-dock" => welcome::dock,
        "welcome-probe" => welcome::probe,
        "title" => title::title,
        "path-card" => title::path_card,
        "toggle-sidebar" => sessions::toggle_sidebar,
        "toggle-panel" => sessions::toggle_panel,
        "stash" => sessions::stash,
        "send" => |ctx, arg| sessions::send(ctx, arg, false),
        "send-submit" => |ctx, arg| sessions::send(ctx, arg, true),
        "hook" => sessions::hook,
        "control" => control::control,
        "changes" => panels::changes,
        "diff" => panels::diff,
        "split" => panels::split,
        "diff-scroll-x" => panels::diff_scroll_x,
        "history" => panels::history,
        "picker" => panels::picker,
        "commit" => panels::commit,
        "commit-file" => panels::commit_file,
        "compare" => panels::compare,
        "project" => panels::project,
        "trust-window" => panels::trust_window,
        "trust-shot" => panels::trust_shot,
        "panel-tab" => panels::panel_tab,
        "project-init" => panels::project_init,
        "project-trust" => panels::project_trust,
        "comment" => |ctx, arg| {
            let said = ctx.workspace.devctl_comment(arg, ctx.window, ctx.cx).map_err(|e| anyhow!(e))?;
            Ok(said)
        },
        "note-seed" => panels::note_seed,
        "note-add" => panels::note_add,
        "note-toggle" => panels::note_toggle,
        "task-seed" => panels::task_seed,
        "task-done" => panels::task_done,
        "panel-scroll" => panels::panel_scroll,
        "set" => settings::set,
        "theme" => settings::theme,
        "settings" => settings::open,
        "settings-section" => settings::section,
        "update-installed" => settings::update_installed,
        "settings-project" => settings::project,
        "settings-project-menu" => settings::project_menu,
        "settings-search" => settings::search,
        "settings-resize" => settings::resize,
        "settings-scroll" => settings::scroll,
        "profile-sheet" => settings::profile_sheet,
        "settings-record" => settings::record,
        "settings-reveal" => settings::reveal,
        "settings-preview" => settings::preview,
        "settings-select" => settings::select,
        "settings-reset-confirm" => settings::reset_confirm,
        "settings-cli-tool" => settings::cli_tool,
        "font-picker" => settings::font_picker,
        "a11y" => a11y::a11y,
        "keyboard-navigation" => a11y::keyboard_navigation,
        "window-front" => a11y::window_front,
        "front" => sessions::front,
        "reduce-motion" => settings::reduce_motion,
        "scroller" => settings::scroller,
        "appearance" => settings::appearance,
        "reduce-transparency" => settings::reduce_transparency,
        "increase-contrast" => settings::increase_contrast,
        "studio" => studio::toggle,
        "studio-select" => |ctx, arg| studio::with(ctx, "studio-select", arg),
        "studio-scroll" => |ctx, arg| studio::with(ctx, "studio-scroll", arg),
        "studio-save" => |ctx, arg| studio::with(ctx, "studio-save", arg),
        "studio-set" => |ctx, arg| studio::with(ctx, "studio-set", arg),
        "studio-type" => |ctx, arg| studio::with(ctx, "studio-type", arg),
        "studio-filter" => |ctx, arg| studio::with(ctx, "studio-filter", arg),
        "studio-area" => |ctx, arg| studio::with(ctx, "studio-area", arg),
        "studio-resize" => |ctx, arg| studio::with(ctx, "studio-resize", arg),
        "studio-menu" => |ctx, arg| studio::with(ctx, "studio-menu", arg),
        "studio-frames" => studio::frames,
        "menubar" => ambient::menubar,
        "dock" => ambient::dock,
        "menubar-icons" => ambient::menubar_icons,
        "notify-sim" => ambient::notify_sim,
        "notify-log" => ambient::notify_log,
        "notify-permission" => ambient::notify_permission,
        "quick-panel" => ambient::quick_panel,
        "quick-panel-shot" => ambient::quick_panel_shot,
        "quick-panel-key" => ambient::quick_panel_key,
        "quick-panel-text" => ambient::quick_panel_text,
        "jump" => ambient::jump,
        "wait-idle" => flow::wait_idle,
        "sync" => flow::sync,
        _ => return None,
    })
}

pub fn poll(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let Some(dir) = std::env::var_os("WORMHOLE_DEVCTL").map(PathBuf::from) else {
        return;
    };
    poll_dir(dir, workspace, window, cx);
}

/// [`poll`] with the control directory given (the view tests have no environment to set).
pub(crate) fn poll_dir(
    dir: PathBuf,
    workspace: &mut Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    // A batch that waits (`wait-idle`) finishes before the next one is read, or the two would
    // interleave.
    if cx.try_global::<Waiting>().is_some_and(|waiting| waiting.0.contains(&cx.entity_id())) {
        return;
    }
    let number = workspace.window_number(cx);
    let suffix = if number == 1 { String::new() } else { number.to_string() };
    let out_path = dir.join(format!("out{suffix}.txt"));
    let Some(script) = take_script(&dir.join(format!("cmd{suffix}"))) else {
        return;
    };
    let batch = Batch {
        lines: script.lines().filter(|l| !l.trim().is_empty()).map(str::to_string).collect(),
        out: String::new(),
        sync: Vec::new(),
        dir,
        out_path,
    };
    run(batch, workspace, window, cx);
}

/// One command file being worked through.
struct Batch {
    /// The lines still to run.
    lines: VecDeque<String>,
    /// What the commands printed so far.
    out: String,
    sync: Vec<String>,
    dir: PathBuf,
    out_path: PathBuf,
}

/// The workspaces (by entity) whose batch is waiting for a command to resolve.
#[derive(Default)]
struct Waiting(HashSet<EntityId>);

impl Global for Waiting {}

/// Run `batch` until it ends, or until a command makes the rest wait for it.
fn run(mut batch: Batch, workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    while let Some(line) = batch.lines.pop_front() {
        let (name, arg) = split_command(&line);
        let mut ctx = Ctx {
            workspace: &mut *workspace,
            window: &mut *window,
            cx: &mut *cx,
            dir: &batch.dir,
            out: &batch.out_path,
            wait: None,
            sync: Vec::new(),
        };
        let result = command(name).map(|run| run(&mut ctx, &arg));
        let (wait, mut sync) = (ctx.wait.take(), std::mem::take(&mut ctx.sync));
        batch.sync.append(&mut sync);
        match result {
            Some(Ok(text)) => batch.out.push_str(&text),
            Some(Err(err)) => {
                writeln!(batch.out, "{name} failed: {err:#}").ok();
            }
            None => {
                writeln!(batch.out, "unknown command: {name}").ok();
            }
        }
        if let Some(wait) = wait {
            resume_after(wait, batch, window, cx);
            return;
        }
    }
    finish(batch, cx);
}

/// Go on with `batch` once `wait` resolves, adding what it printed.
fn resume_after(wait: Task<String>, mut batch: Batch, window: &mut Window, cx: &mut Context<Workspace>) {
    let id = cx.entity_id();
    cx.default_global::<Waiting>().0.insert(id);
    cx.spawn_in(window, async move |this, cx| {
        batch.out.push_str(&wait.await);
        let resumed = this.update_in(cx, |workspace, window, cx| {
            cx.default_global::<Waiting>().0.remove(&id);
            run(batch, workspace, window, cx);
        });
        if resumed.is_err() {
            tracing::debug!("devctl: the window closed while its batch waited");
        }
    })
    .detach();
}

fn finish(batch: Batch, cx: &mut Context<Workspace>) {
    let _ = std::fs::write(&batch.out_path, &batch.out);
    if !batch.sync.is_empty() {
        // Deferred, so the token comes after what commands of this batch append once the update
        // is over (`settings-shot`): a script that waits for it finds their results too.
        let (path, sync) = (batch.out_path, batch.sync);
        cx.defer(move |_| append(&path, &sync_lines(&sync)));
    }
    cx.notify();
}

/// The lines `sync` writes for these tokens.
fn sync_lines(tokens: &[String]) -> String {
    tokens
        .iter()
        .map(|token| if token.is_empty() { "sync\n".into() } else { format!("sync {token}\n") })
        .collect()
}

/// Add `text` to the results file at `path`.
fn append(path: &Path, text: &str) {
    use std::io::Write as _;
    let appended = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(text.as_bytes()));
    if let Err(err) = appended {
        tracing::warn!("devctl: could not append to {}: {err}", path.display());
    }
}

/// Read the command file and remove it. It is renamed first, so a line a writer appends
/// while this runs lands in a new file for the next tick; read-then-delete lost it.
fn take_script(path: &Path) -> Option<String> {
    let taken = path.with_extension("run");
    std::fs::rename(path, &taken).ok()?;
    let script = std::fs::read_to_string(&taken);
    let _ = std::fs::remove_file(&taken);
    script.ok()
}

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
    use super::{coordinates, file_name, nth, split_command, sync_lines, take_script};

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
    fn sync_writes_one_line_per_token_in_order() {
        assert_eq!(sync_lines(&["a1".into(), String::new(), "b 2".into()]), "sync a1\nsync\nsync b 2\n");
        assert_eq!(sync_lines(&[]), "");
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

    #[test]
    fn taking_the_script_leaves_room_for_the_next_one() {
        let dir = std::env::temp_dir().join(format!("wormhole-devctl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let cmd = dir.join("cmd");
        assert_eq!(take_script(&cmd), None, "no file, no script");
        std::fs::write(&cmd, "state\nscreen\n").expect("write");
        assert_eq!(take_script(&cmd).as_deref(), Some("state\nscreen\n"));
        // The file is gone, and so is the one it was renamed to: a writer's next line starts
        // a new file.
        assert!(!cmd.exists() && !dir.join("cmd.run").exists());
        std::fs::write(&cmd, "state\n").expect("write again");
        assert_eq!(take_script(&cmd).as_deref(), Some("state\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
