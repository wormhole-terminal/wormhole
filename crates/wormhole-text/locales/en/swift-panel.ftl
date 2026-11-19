# The Swift app's right panel (Notes, Changes with the diff, History, Files, Project and Usage), the panes' area and
# the terminal's title row and tab bar: the words of what the GPUI app draws that the other catalogs don't hold.
# A file's, folder's, branch's or commit's own name and a commit's own words are never here: they come from the disk
# or the repository.

## Git, as every tab says it

# What a tab says where its folder isn't in a repository.
git-not-repository = This terminal's folder isn't inside a git repository.
git-retry = Retry
git-copy-error = Copy error
git-unreadable = git can't be read.
git-not-a-repository = Not a git repository

## The branch picker

# The accessibility hint of the button that opens the list of branches.
picker-hint = Opens the list of branches
picker-search = Search branches
picker-none = Nothing matches.

# Under the picker's list while the repository's branches are being read.
picker-loading = Loading branches…

# Under the list of the Changes tab's picker: what comparing with a branch shows.
picker-compare-hint = Changes since this branch split off from it

# The picker's first choice in Changes: no branch, the working tree against HEAD (git's name for the current commit).
picker-head-compare = HEAD (uncommitted changes)

# The picker's first choice in History: the log of the branch that is checked out.
picker-head-history = HEAD (current branch)

## What a band or a row says it is, to VoiceOver

panel-spoken-folded = folded
panel-spoken-open = open

# The open-in-editor icon on a file row or a diff line, so a list of them is not a list of identical buttons (core's
# `spoken::open_file`). { $tooltip } is the icon's tooltip ("Open in Zed at line 12"), { $file } the file's name.
panel-spoken-open-file = { $tooltip }, { $file }
files-spoken-expanded = expanded
files-spoken-collapsed = collapsed

## Changes

changes-list-label = Changed files
changes-reading = Reading git status…
changes-clean = No changes. Working tree is clean.
changes-refresh = Refresh

# A red line over a list that stays. { $message } is git's first line.
changes-refresh-failed = Couldn't refresh: { $message }

# { $branch } is the branch the list was compared with.
changes-compare-missing = { $branch } no longer exists · showing vs HEAD

# The word before the branch the list is compared with ("vs main"), read out by VoiceOver.
changes-compare-versus = vs

changes-group-changes = Changes
changes-group-untracked = Untracked

# A section's head, read out by VoiceOver. { $group } is its name, { $count } how many files it holds.
changes-group-spoken = { $group }, { $count ->
    [0] no files
    [one] 1 file
   *[other] { $count } files
}

# The badge of a binary file in the list, short.
changes-badge-binary = bin

# The note over a long list that was folded. { $count } is a whole number, already formatted.
changes-folded = Folded because there are more than { $count }.

# Before the note above, when most files are in one folder. { $count } is a number, already formatted, { $folder } the
# folder's name.
changes-folded-crowded = { $count } of them are in { $folder }/. { $why }

# The same for untracked files, which git doesn't ignore.
changes-folded-crowded-untracked = { $count } of them are in { $folder }/, which isn't ignored. { $why }

# The first part of a renamed file's tooltip. { $kind } is "Renamed", { $path } its old path.
changes-renamed-from = { $kind } from { $path }

# The count of a binary file in a tooltip.
changes-counts-binary = binary

## What happened to a file

changes-kind-modified = Modified
changes-kind-added = Added
changes-kind-deleted = Deleted
changes-kind-renamed = Renamed
changes-kind-untracked = Untracked
changes-kind-conflicted = Conflicted

# A submodule's row, read out by VoiceOver. { $name } is the submodule, { $what } its state.
changes-spoken-submodule = Submodule { $name }, { $what }

# The button on a submodule's row. { $name } is the submodule's name.
submodule-enter = Open a terminal in { $name }

# The alert after the review was copied instead of pasted.
review-copied-instead = Copied the comments instead
review-copied-instead-ok = OK
review-copied-instead-detail = The program in this terminal would run each line of a paste as a command, so the comments are on the clipboard. Paste them into your agent's prompt.

## The diff

# Read out by VoiceOver. { $name } is the file's name.
diff-spoken = Diff of { $name }

diff-reading = Loading diff…
diff-unreadable = The diff can't be read.
diff-previous-file = Previous file
diff-next-file = Next file

# Where the open file stands among the changed ones. { $index } and { $count } are whole numbers.
diff-file-position = File { $index } of { $count }

diff-wrap-lines = Wrap long lines
diff-split = Split
diff-unified = Unified
diff-close = close
diff-close-spoken = Close diff
diff-close-tooltip = Close diff (esc)
diff-badge-new = New file

# { $path } is the old path.
diff-badge-renamed-from = Renamed from { $path }

diff-badge-binary = Binary
diff-badge-no-highlighting = No highlighting
diff-copy-oid = Copy object id
diff-binary-file = Binary file
diff-empty-file = Empty file
diff-no-content-changes = No content changes
diff-load-anyway = Load diff
diff-no-newline = no newline at end
diff-reveal-current = Reveal Current File in Finder

## Comments on the diff, for the agent

# The editor's title. { $place } is a file and line.
review-title = Comment on { $place }

review-placeholder = What should the agent change here?

# After the key cap that shows ⌘↩.
review-saves = saves

review-remove = Remove
review-add = Add Comment
review-save = Save
review-clear = Clear
review-paste = Paste into Terminal
review-paste-help = Paste the comments into the terminal's prompt, for you to send

# The bar's summary. { $count } is how many comments, { $files } over how many files.
review-summary = { $count ->
    [one] 1 comment
   *[other] { $count } comments
}

# The bar's summary when the comments are in more than one file.
review-summary-files = { $comments } on { $files } files

## Files

files-list-label = Files

# The menu line and the viewer's button that open the file in the chosen editor. { $editor } is the app's name.
files-open-in-editor = Open in { $editor }

## History

history-list-label = Commits
history-title = History

# The head of the list, read out by VoiceOver. { $count } is how many commits.
history-spoken-list = History, { $count ->
    [0] no commits
    [one] 1 commit
   *[other] { $count } commits
}

history-reading = Reading history…
history-no-files = No file changes.
history-loading-files = Loading files…
history-unreadable = The log can't be read.
history-details-unreadable = The details can't be read.

## Notes and next steps

notes-list-label = Notes
notes-title = Notes
notes-new = New note

# The band's VoiceOver label (the core's `section_band`: a zero is said). { $count } is how many notes are written.
notes-spoken = { $count ->
    [0] Notes, no notes
    [one] Notes, 1 note
   *[other] Notes, { $count } notes
}

# The editor's hint while it is empty.
notes-placeholder = Write a note or a prompt draft…  ⌘↩ sends the paragraph under the cursor (or the selection); ⌘⇧↩ also submits.

# The button under a note that is cut. { $count } is a number of lines, already formatted.
notes-show-all = Show all · { $count } lines

notes-stash-prompt = Stash prompt into a new note
notes-prompt-stashed = Prompt stashed in a new note
notes-task-done = done
notes-send = Send to terminal
notes-delete = Delete
steps-title = Next steps
steps-add = Add next step
steps-placeholder = Next step

# The band's VoiceOver label. { $count } is how many steps are open.
steps-spoken = { $count ->
    [0] Next steps, none open
    [one] Next steps, { $count } open
   *[other] Next steps, { $count } open
}

# The fold of finished steps. { $count } is a number, already formatted.
steps-done = { $count } done

## Project

project-folder = Project folder
project-none = This terminal's folder has no .wormhole/ project folder.
project-none-about = A committed .wormhole/ can hold project settings, a theme, actions, prompts, a terminal layout and shared notes. Every part is optional, and you choose which ones to use.
project-create = Create project folder
project-no-folder = This terminal's folder isn't known yet.

# { $folder } is the folder the project folder would be made in.
project-creates = Creates { $folder }/.wormhole/

# The notice chip after a call the core refused. { $reason } is the core's own words.
project-refused = Not allowed: { $reason }

# { $what } is what was not found, in the core's own words.
project-not-found = Not found: { $what }

project-folder-color = Folder colour
project-pick-colour = Pick a colour
project-color-none = None

# { $hex } is the colour, "#3366CC".
project-color-custom = Custom colour { $hex }
# The tooltip of the chosen custom colour. { $hex } is the colour, "#3366CC".
project-color-custom-help = Custom colour { $hex }, set in config.json

project-color-red = Red
project-color-orange = Orange
project-color-yellow = Yellow
project-color-green = Green
project-color-teal = Teal
project-color-blue = Blue
project-color-purple = Purple
project-color-pink = Pink
project-color-own = Marks this folder's band. Yours only: it isn't saved in the project.
project-actions = Actions
project-prompts = Prompts
project-prompt-paste = Paste into the terminal
project-prompt-note = Copy into a new note
project-layout = Terminal layout
project-open-layout = Open terminals

project-badge-trusted = trusted
project-badge-changed = changed
project-badge-untrusted = not trusted

# The button of the notice. { $count } is how many lines there are.
project-view-all = View all { $count }…

project-notice-changed-title = Files changed since you trusted this project
project-notice-untrusted-title = Trust this project?
project-notice-changed-commands = Review what it would run now and approve again.

# { $applies } says what trust applies ("its settings in config.json"), in English for now.
project-notice-changed-applies = Trusting it applies { $applies }. Review it and approve again.

project-notice-untrusted-commands = It wants to run these commands. Nothing runs until you approve.
# { $applies } says what trust applies, in English for now.
project-notice-untrusted-applies = Trusting it applies { $applies }. Nothing applies until you approve.

# The quiet line of a trusted project. { $applies } says what trust applies.
project-trusted-applies = Applies { $applies }.

project-trusted-commands = Actions, the terminal layout and project env may run.

## What a project folder can offer, and the file behind each

project-feature-settings = Settings
project-feature-theme = Theme
project-feature-actions = Actions
project-feature-prompts = Prompts
project-feature-layout = Terminal layout
project-feature-notes = Shared notes
project-feature-settings-detail = Setting overrides, environment variables and shell from config.json
project-feature-theme-detail = The project's color theme from theme.json
project-feature-actions-detail = Commands and key bindings from actions.json
project-feature-prompts-detail = Reusable prompts from the prompts folder
project-feature-layout-detail = Terminals opened (and commands started) with the project, from layout.json
project-feature-notes-detail = Notes and next steps synced with notes.md

## Usage

usage-plan-limits = Plan usage limits
usage-latest-response = Latest model response

# { $count } is a whole number of tokens.
usage-tokens = { $count ->
    [one] { $count } token
   *[other] { $count } tokens
}

usage-gemini-note = Latest report across Gemini terminals. Plan limits are not reported.
usage-empty-1 = Claude and Codex plan limits, and Gemini response tokens, show here as you use them in { -brand }.
usage-empty-2 = Enable or update their integrations in Settings → Coding agents. Plan limits need a supported subscription; API-key logins may not report them.

# A window that has reset since the numbers came.
usage-awaiting = Awaiting update

# A gauge's words. { $percent } is a whole number, { $left } how long until it resets ("34m").
usage-limit-resets = { $percent }% · resets { $left }

usage-updated-now = Updated just now

# { $age } is how long ago ("3m").
usage-updated-ago = Updated { $age } ago

# A gauge read out by VoiceOver. { $percent } is a whole number.
usage-spoken-percent = { $percent } percent

## Worktrees

worktree-title = New terminal in a new worktree

# { $name } is the repository's name.
worktree-about = A worktree of { $name } on a branch of its own, in a folder of its own, so work there touches nothing in the repository's folder.

worktree-this-repository = the repository
worktree-branch = Branch (new, or one the repository has)
worktree-cancels = cancels
worktree-create = Create

# Under the branch field. { $folder } is the folder that would be made.
worktree-folder = Folder: { $folder }

worktree-folder-pending = Folder: <branch>
worktree-remove-failed = Couldn't remove the worktree
worktree-refused-title = Can’t make a worktree here
worktree-create-failed = Couldn’t create the worktree

## The panes' area

pane-menu-split-right = Split right
pane-menu-split-down = Split down
pane-menu-zoom = Zoom pane
pane-menu-unzoom = Unzoom pane
pane-menu-leave-split = Move out of split
pane-close = Close pane
pane-menu = Pane menu
pane-divider = Pane divider
pane-divider-hint = Drag to resize · double-click to equalize

# A header read out by VoiceOver. { $position } of { $total } are whole numbers.
pane-spoken-position = pane { $position } of { $total }

pane-spoken-focused = focused
pane-drop-show-left = Show on the left
pane-drop-show-right = Show on the right
pane-drop-show-above = Show above
pane-drop-show-below = Show below

# What a drop would do. { $target } is the terminal's name.
pane-drop-place-left = Place left of { $target }
# { $target } is the terminal's name.
pane-drop-place-right = Place right of { $target }
# { $target } is the terminal's name.
pane-drop-place-above = Place above { $target }
# { $target } is the terminal's name.
pane-drop-place-below = Place below { $target }

## The tab bar, the group switcher and the profile chip

tab-close = Close terminal
tab-new = New terminal

# A tab, read out by VoiceOver. { $name } is the terminal's name.
tab-spoken = { $name }, tab
# { $name } is the terminal's name.
tab-spoken-selected = { $name }, tab, selected

# A group's button. { $group } is its name.
tab-switcher-spoken = Group { $group }, switch group
# { $group } is the group's name.
tab-switcher-spoken-alert = Group { $group }, another group needs you

tab-switcher-search = Go to group or terminal…
tab-switcher-no-matches = No matches
tab-switcher-no-terminals = No terminals
tab-switcher-new-group = New group
# The + on a group's card. { $group } is its name.
tab-switcher-new-in-group = New terminal in { $group }
# The switcher's footer. { $previous } and { $next } are the two keys.
tab-switcher-keys-hint = { $previous } / { $next } previous, next group
# The head over the shells in the menu of the +.
tab-new-with = New terminal with
tab-switcher-show-in-sidebar = Show all groups in the sidebar
tab-switcher-ungrouped = Ungrouped

# Under a group's name. { $count } is how many terminals it holds.
tab-switcher-group = { $count ->
    [one] group · 1 terminal
   *[other] group · { $count } terminals
}

# A group's card, read out by VoiceOver, between commas.
tab-switcher-card-terminals = { $count ->
    [one] 1 terminal
   *[other] { $count } terminals
}

tab-switcher-card-current = current
tab-switcher-card-alert = needs you

# { $name } is the profile's name.
profile-chip-spoken = Profile { $name }, switch profile

profile-menu-switch-to = Switch { -brand } to
profile-menu-open-in = Open a window in…
profile-menu-move-to = Move this window to…
profile-menu-new = New profile…
profile-menu-manage = Manage profiles…
profile-menu-settings = Settings…

## The title row

title-row-path-insert = Insert into terminal

# The button over the folder, read out by VoiceOver. { $shown } is the folder as the row shows it.
title-row-path-spoken-open = Open folder { $shown } in Finder

title-row-detached = detached

# { $name } is the branch's name.
title-row-branch-spoken = branch { $name }

# { $phrase } says what the submodule is.
title-row-submodule = A submodule: { $phrase }

title-row-hidden-from-agents = Hidden from agents
title-row-open-in-header = Open folder in
title-row-open-in-another = Open in another app

# { $app } is the app's name.
title-row-open-in-app = Open folder in { $app }

# { $missing } is the app that was chosen and isn't installed, { $app } the one that opens instead.
title-row-open-in-missing = { $missing } isn't installed. Open folder in { $app }

title-row-split-right = Split right (⌘D)
title-row-split-down = Split down (⇧⌘D)

# A branch's state against its upstream, in the branch status mark's tooltip.
title-row-uncommitted = Uncommitted changes

# { $count } is a whole number of commits, { $upstream } the upstream's name with a space before it, or nothing.
title-row-ahead = { $count ->
    [one] 1 commit ahead of{ $upstream }
   *[other] { $count } commits ahead of{ $upstream }
}

# The same behind it. { $count } is a whole number of commits, { $upstream } as above.
title-row-behind = { $count ->
    [one] 1 commit behind{ $upstream }
   *[other] { $count } commits behind{ $upstream }
}

# The button of a listening port. { $port } is its number.
title-row-port-open = Open localhost:{ $port } in the browser

# { $count } is how many more ports listen, { $ports } their numbers, comma separated.
title-row-ports-more = { $count } more listening: { $ports }

# The agent meter's words. { $percent } is a whole number.
title-row-context-used = { $percent }% of the context window used

diff-badge-out-of-date = Out of date
diff-badge-gone = No longer changed
project-trust-password = Trust (approve with password)

# A submodule's diff card: the heading over the commits the pointer moved forward over.
diff-submodule-new-commits = New commits

# The heading over the commits the pointer no longer includes (it went back, or to another line of history).
diff-submodule-left-behind = Commits left behind

# After the commits listed. { $count } is how many more there are, already written with its separators.
diff-submodule-more = and { $count } more

# The file's path in the title row while the viewer shows it, for VoiceOver. { $path } is the path below the folder.
files-spoken-reveal-file = Reveal { $path } in Finder
