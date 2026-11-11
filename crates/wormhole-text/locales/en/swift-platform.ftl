# The Swift app's macOS edge: the status item and Dock menus, the hotkey quick panel, the menu bar's dynamic parts,
# notification buttons, the shortcut recorder and the explanations of who owns a key. The menu bar's own labels come
# from the core's keymap, the folder picker's words are folder-picker.ftl, and the context menus use the sidebar,
# history and terminal messages; this file holds only what has none.

## The status item's tooltip

# The tooltip of the menu bar icon; { $summary } is one of the lines below, or the counts joined with " · ".
platform-status-tooltip = { -brand } — { $summary }

platform-status-no-terminals = no terminals open

# { $count } is how many terminals are open, none of them needing anything.
platform-status-all-quiet = { $count -> [one] { $count } terminal *[other] { $count } terminals }, all quiet

# A count among the tooltip's parts: "2 need input · 1 failed · 2 working · 2 finished".
platform-status-finished = { $count } finished

## A terminal's second line in the menus and the quick panel

# After the command's name: how long it took; { $duration } is like "4m 12s".
platform-detail-after = after { $duration }

# A terminal that needs input and told nothing but the bell.
platform-detail-bell = Bell

platform-detail-exited = Exited

## The status item's menu and the Dock menu

platform-menu-new-terminal = New Terminal
platform-menu-new-window = New Window
platform-menu-open-folder = Open Folder…

# The submenu of terminals that are idle or ended.
platform-status-other = Other Terminals

# The last line of a long list; { $count } is how many more terminals there are.
platform-status-more = { $count } more…

platform-status-none-open = No terminals open
platform-status-projects = Projects
platform-status-show = Show { -brand }
platform-status-jump = Jump to Next Waiting
platform-status-updates = Check for Updates…
platform-status-quit = Quit { -brand }

# A terminal without a name.
platform-status-untitled = Untitled

# The header of a project's actions; { $name } is the project's folder name.
platform-status-actions = Actions · { $name }

# The same when two projects share a folder name; { $root } is the project's path.
platform-status-actions-at = Actions · { $name } — { $root }

## The hotkey quick panel

# The rows with a new terminal or a project's action; { $title } is the action's name.
platform-quick-run = Run { $title }

# The second line of a project's action; { $project } is the project's name.
platform-quick-action-detail = action · { $project }

platform-quick-nothing-waiting = Nothing needs input

# The section of terminals that finished lately.
platform-quick-recent = Recent

# The section of things to do: new terminal, open a folder, project actions.
platform-quick-do = Do

platform-quick-terminals = Terminals
platform-quick-nothing-matches = Nothing matches

# What a terminal that waits for an answer shows when it has no message of its own.
platform-quick-waiting = Waiting for your answer

# The hints under the list, each a key and what it does.
platform-quick-hint-select = select
platform-quick-hint-answer = answer here
platform-quick-hint-close = close
platform-quick-hint-send = send reply
platform-quick-hint-open-terminal = open terminal
platform-quick-hint-back = back

## Notifications

platform-notice-open = Open
platform-notice-reply = Reply…
platform-notice-show-changes = Show Changes

# The reply button of a notification and the field the reply is typed in.
platform-send = Send
platform-reply = Reply

## The folder picker

# VoiceOver's name for the path above the list.
platform-picker-crumbs = Where you are

# VoiceOver's word for a row that can be opened.
platform-picker-has-folders = has folders

## The menu bar's parts that follow the window

platform-menu-open-recent = Open Recent
platform-menu-no-recent = No Recent Folders
platform-menu-project-actions = Project Actions
platform-menu-no-project-actions = No Project Actions
platform-menu-new-terminal-with = New Terminal With
platform-menu-no-shells = No Shells
platform-menu-open-window-in = Open Window in
platform-menu-move-window-to = Move Window to
platform-menu-profile = Profile

# The close item while a split is shown: it closes the pane that has the focus.
platform-menu-close-pane = Close Pane

platform-menu-services = Services

## Context menus

# The first item of a file's menu in the Changes list.
platform-menu-open = Open

# VoiceOver's name for the row of colour dots in a menu.
platform-swatch-label = Colour

## Shortcuts: who has a key already

# { $key } is the key, { $name } the action that has it.
platform-key-is = { $key } is { $name }

# { $what } is what macOS does with the key (the keymap's own words).
platform-key-system = macOS uses it for { $what }

# { $what } is one of the three lines below.
platform-key-shell = the shell needs it ({ $what })
platform-key-shell-control = a control character
platform-key-shell-meta = Meta, word jumps and accents
platform-key-shell-typing = typing

# What the shortcut field says when the key is taken; { $key } is the key, { $what } what macOS does with it.
platform-recorder-system = macOS uses { $key } for { $what }
platform-recorder-shell = The shell uses { $key }

platform-recorder-add-modifier = Add ⌘, ⌃ or ⌥ to the key
platform-recorder-type = Type shortcut
platform-recorder-recording = Recording
platform-recorder-help = Press to record a new shortcut. Escape cancels, Delete clears.

# The global hotkey could not be registered; { $status } is the system's error number.
platform-hotkey-taken = another app owns this combination (error { $status })

## Settings values and the default terminal

platform-value-on = On
platform-value-off = Off

# The reason Settings gives when Wormhole cannot become the default terminal because it was not started from its app.
platform-default-terminal-not-bundled = not running from Wormhole.app
