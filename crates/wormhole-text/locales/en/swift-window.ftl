# The Swift app's window: the terminal list's menus and popover, the command palette and the shortcut sheet,
# the status bar, the window's accessibility words and the few labels the Settings window writes itself.
# What the settings schema and the palette's rows say comes from the core, in these catalogs already.

## The terminal list: its rows and groups

# VoiceOver's name for the list.
sidebar-list = Terminals

# The line in a group with no terminals.
sidebar-group-empty = No terminals

# VoiceOver's hint on a group's head.
sidebar-group-head-hint = Folds or opens the group

# The tooltip of the chain on a row whose split has terminals in other folders.
sidebar-mark-split-folders = In a split with terminals in other folders

# The close button on a row: its tooltip and its VoiceOver name; the same words are the menu item and the action.
sidebar-close-terminal = Close terminal

# VoiceOver's name for the field a row's or a group's name is edited in.
sidebar-rename-field = Name

# The VoiceOver action on a group's head that opens its popover.
sidebar-group-edit-action = Edit group

# A row's tooltips: the mark of a terminal hidden from agents, in two lines.
sidebar-hidden-from-agents = Hidden from agents
sidebar-hidden-from-agents-detail = Programs that control Wormhole can't see it

# The worktree mark's tooltip. { $name } is the worktree's name, { $repo } the repository's folder name.
sidebar-worktree-of = { $name } · worktree of { $repo }

## The note under the list while no coding agent is installed

tip-no-agents-title = No coding agents yet
tip-no-agents-body = Rows show the running command instead. Status glyphs still mark a busy terminal.
tip-no-agents-settings = Coding agents settings
tip-close = Close tip
tip-closed = Tip closed
tip-never = Don't show again
tip-keep = Keep the tip for next time

# The VoiceOver action on a row (the menu item has an ellipsis).
sidebar-rename-action = Rename

sidebar-pin = Pin
sidebar-unpin = Unpin

## A group's popover

sidebar-popover-group-name = Group name
sidebar-popover-colour-other = Other colour

## A terminal's menu

sidebar-menu-rename = Rename…
sidebar-menu-add-to-new-group = Add terminal to new group
sidebar-menu-add-to-group = Add terminal to group
sidebar-menu-remove-from-group = Remove from group
sidebar-menu-split-right = Split right
sidebar-menu-remove-from-split = Remove from split
sidebar-menu-pin-top = Pin to top
sidebar-menu-forget-text = Don't keep this terminal's text
sidebar-menu-hide-from-agents = Hide from Agents
sidebar-menu-show-beside = Show beside…

## A group's menu

sidebar-menu-group-colour-other = Other colour…
sidebar-menu-group-colour-none = No colour
sidebar-menu-group-new-terminal = New terminal in group
sidebar-menu-group-ungroup = Ungroup
sidebar-menu-group-close = Close group
sidebar-menu-group-pin = Pin group
sidebar-menu-group-unpin = Unpin group
sidebar-menu-group-move-to-window = Move group to new window
sidebar-menu-group-set-folder = Set folder…
sidebar-menu-group-clear-folder = Clear folder

## The command palette

palette-title = Command palette
palette-scope-label = Scope

# The groups of results, and the scopes that show one of them.
palette-group-settings = Settings
palette-group-actions = Actions
palette-group-terminals = Terminals
palette-group-folders = Folders
palette-group-commits = Commits
palette-scope-all = All

# What the empty search field says, per scope.
palette-placeholder-all = Jump to a terminal, folder, commit or setting…
palette-placeholder-terminals = Terminals in every window…
palette-placeholder-folders = Folders with a terminal, and recent ones…
palette-placeholder-commits = Commits on this branch…
palette-placeholder-settings = Find a setting…
palette-placeholder-actions = Run an action…

# The headings of the Actions scope.
palette-section-terminal = Terminal
palette-section-view = View
palette-section-app = App
palette-section-profiles = Profiles

# A note in the list when there is nothing to show.
palette-empty-nothing-yet = Nothing here yet
palette-empty-nothing-matches = Nothing matches
palette-empty-type-setting = Type to find a setting

# The Terminals scope groups terminals by what they are doing.
palette-state-needs-input = Needs input
palette-state-working = Working
palette-state-finished = Finished
palette-state-idle = Idle
palette-state-failed = Failed

# The heading over a trusted project's own actions. { $project } is the project's name.
palette-project-named = Project · { $project }
palette-project = Project

# The row that opens a terminal with another shell. { $shell } is the shell's name.
palette-new-terminal-with = New terminal: { $shell }

# The rows for another profile. { $profile } is the profile's name.
palette-profile-switch = Switch to { $profile }
palette-profile-switch-detail = every window switches
# { $profile } is the profile's name.
palette-profile-open-window = Open window in { $profile }
# { $profile } is the profile's name.
palette-profile-move-window = Move this window to { $profile }

# The line after an action's name.
palette-detail-new-terminal-in = in { $folder }
palette-detail-open-folder = start a terminal in another folder
# { $count } is a whole number of files.
palette-detail-files-changed = { $count ->
    [one] { $count } file changed
   *[other] { $count } files changed
}

# The line under a setting. { $section } is its page.
palette-detail-setting = Settings › { $section }

# The line under a folder, after its path. { $count } is a whole number of terminals.
palette-detail-terminals = { $count ->
    [one] { $count } terminal
   *[other] { $count } terminals
}

# The line under a terminal in the Terminals scope. { $window } is a whole number.
palette-terminal-window = Window { $window }

# What a failed terminal's command exited with.
palette-terminal-exit = exit { $code }

# The line under a terminal that waits for an answer. { $who } is the agent, { $message } what it asked.
palette-terminal-asking = { $who } is asking: { $message }

## The palette's footer: each key and what it does

palette-hint-switch = switch
palette-hint-close-terminal = close terminal
palette-hint-run = run
palette-hint-back = back to all
palette-hint-move = move
palette-hint-open-in-settings = open in Settings
palette-hint-copy-setting-key = copy setting key
palette-hint-open = open
palette-hint-open-in-window = open in new window
palette-hint-scope = scope

# The count at the footer's right once something is typed. { $count } is a whole number.
palette-results = { $count ->
    [one] { $count } result
   *[other] { $count } results
}

# With all searched and nothing typed: "Type > for actions", the > being a key cap.
palette-actions-hint-type = Type
palette-actions-hint-for-actions = for actions

## The keyboard shortcut sheet

palette-sheet-title = Keyboard shortcuts
palette-sheet-filter = Filter shortcuts
palette-sheet-none = No shortcut matches

# The line over a trusted project's own shortcuts. { $project } is its name.
palette-sheet-project = This project · { $project }

# The footer while filtering. { $shown } and { $total } are whole numbers, { $key } is a sample key.
palette-sheet-filtering = { $shown } of { $total } · matches labels and keys (“{ $key }” finds Close terminal)

# The footer: the global hotkey, with its key cap between the two parts.
palette-sheet-hotkey = Show or hide { -brand } from any app
palette-sheet-hotkey-where = set in Settings › Hotkey
palette-sheet-hotkey-unset = A key that shows or hides { -brand } from any app is set in Settings › Hotkey
palette-sheet-also = Also in Help › Keyboard Shortcuts and the palette

# What VoiceOver says for a shortcut: its name, then its keys.
palette-sheet-spoken = { $label }, { $keys }

## The window

window-new-terminal = New terminal
window-new-terminal-help = New terminal (⌘T)
window-terminal = Terminal
window-panel = Right panel
window-status-bar = Status bar

# VoiceOver's value for a column's edge. { $points } is a whole number.
window-column-width = { $points ->
    [one] { $points } point
   *[other] { $points } points
}

# The readout beside the pointer when a column stops at its smallest or largest width.
window-resize-min = { $width } · min
# { $width } is a whole number of points.
window-resize-max = { $width } · max
window-resize-sidebar = Resize sidebar
window-resize-panel = Resize panel

# What letting go would do to a column dragged past its smallest width.
window-resize-hide = Hide
window-resize-close = Close

# A panel tab's place for VoiceOver. { $position } and { $total } are whole numbers.
window-tab-position = { $position } of { $total }

## Where the keys are now, as VoiceOver says it after F6

# { $item } is the button the keys are on, { $position } and { $total } whole numbers.
window-focus-title-row = Title row, { $item }, button { $position } of { $total }
window-focus-hide-sidebar = Hide sidebar
window-focus-show-sidebar = Show sidebar

# { $title } is the selected terminal's name.
window-focus-terminal = Terminal, { $title }
window-focus-terminal-none = none

# { $tab } is the name of the panel tab that is shown.
window-focus-panel-tabs = Right panel tabs, { $tab }

# { $position } and { $total } are whole numbers.
window-focus-status-bar = Status bar, item { $position } of { $total }

## The status bar

# A terminal's state in words, in the bar and in the title row.
window-state-working = working
window-state-needs-input = needs input
window-state-needs-you = needs you
window-state-finished = finished
window-state-done = done
window-state-failed = failed
window-state-exited = exited

# The counts. { $count } is a whole number of terminals.
window-status-working = { $count } working

# The same for the terminals that wait for an answer.
window-status-need-input = { $count ->
    [one] { $count } needs input
   *[other] { $count } need input
}
# The same for finished and failed terminals.
window-status-done = { $count } done
# { $count } is a whole number of terminals.
window-status-failed = { $count } failed

# What VoiceOver says for the selected terminal's size. { $size } is "80x24".
window-status-size-spoken = { $title }, terminal size { $size }

## The Settings window: what it writes itself

settings-row-not-set = Not set
settings-row-none = None
settings-row-changed = Changed
settings-row-recorder-prompt = Press the shortcut…
settings-row-recorder-none = Not set
settings-panel-not-built = Not in this build yet
settings-nav-profile-default = Default

# VoiceOver's name for a preview. { $scene } is the page's title.
settings-window-preview-label = Preview of { $scene } with your current settings

# Over the Changed page (the arrow is the Reset button of each row).
settings-changed-intro-arrow = Every setting that differs from its default, each put back by its arrow.

settings-menu-bar-off = The menu bar item is off
title-row-move-up = Move up
title-row-move-down = Move down

# The profile pages are not available when there is no core behind the window.
settings-profile-unavailable = Profiles are not available in this build.
settings-import-theme-unavailable = Themes can't be imported in this build.

# The names of the densities in the Terminals list's preview.
settings-window-preview-density-regular = Regular
settings-window-preview-density-compact = Compact
settings-window-preview-density-dense = Dense
