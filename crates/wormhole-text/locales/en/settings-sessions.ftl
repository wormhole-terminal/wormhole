# Settings window, sessions page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-sessions = Terminals & sidebar
    .intro = The Terminals list on the left: how it's grouped and what each row shows.

settings-sessions-sidebar-preview = Live preview
    .keywords = preview sidebar

## Terminals in

settings-sessions-group-terminals-in = Terminals in

settings-sessions-terminals-in = Show terminals in
    .description = Sidebar: the list on the left; Tabs: a tab bar instead, with the groups as sets of tabs
    .keywords = tabs tab bar sidebar list layout

## Terminals list

settings-sessions-group-terminals-list = Terminals list

settings-sessions-density = Density
    .description = How much room each terminal takes, in the sidebar and the tab bar: Regular rows are 48 pt with a second line, Compact 32, Dense 26
    .keywords = density compact dense regular comfortable rows size

settings-sessions-cards = Show groups as cards
    .description = Off: groups are a head and their rows, with no card around them
    .keywords = cards groups card background border

settings-sessions-fade-group-colors = Fade group colours
    .description = On: a group's colour fades out across its card; off, it fills the card evenly
    .keywords = fade gradient group colour color card wash

settings-sessions-show-group-folders = Show group folders
    .description = The folder a group's new terminals start in, under its name
    .keywords = group folder path directory name head show hide

settings-sessions-sidebar-numbers = ⌘1–⌘9 numbers
    .description = Stay with the terminal: dealt down the list, then kept (a new terminal gets the next one); Follow the list: the first nine rows you can see
    .keywords = command shortcuts numbers stay follow list stable classic

settings-sessions-cmd-hints = Show ⌘1–⌘9 while ⌘ is held
    .description = Keycaps on the first nine terminals after 250 ms; the shortcuts work either way
    .keywords = command keycaps shortcuts numbers hints

## Row details

settings-sessions-group-row-details = Row details

settings-sessions-sidebar-changes = Changed files
    .description = ±N uncommitted files in the terminal's row
    .keywords = git changes count

settings-sessions-sidebar-changes-scope = Count
    .description = The files this terminal changed, or everything changed in its repository, shown once, on the topmost row of that repository
    .keywords = git changes count repository terminal own

settings-sessions-sidebar-changes-plain = Plain terminals
    .description = Which programs make a terminal without an agent count the files changed while they run. Commands that finish: a build, a test run, a formatter. Any program: an editor or a dev server too, which then take edits made outside the app
    .keywords = shell command editor dev server vim ssh

settings-sessions-sidebar-changes-shared = Files changed at once
    .description = When two terminals worked at the same time, whose file it is can't be told: both count it, or neither does
    .keywords = both shared overlap ambiguous agents

settings-sessions-sidebar-changes-remember = Remember after restart
    .description = Keep which files each terminal changed when the app quits, for the terminals that come back
    .keywords = persist restart restore

settings-sessions-sidebar-dot = Status dot
    .description = Working, needs input, finished or idle, as the state round every terminal's status tile
    .keywords = status glyph agent

settings-sessions-sidebar-notes = Notes and tasks
    .description = Note icon with the number of notes and open tasks
    .keywords = notes tasks next steps count

## settings_window/sections/sessions.rs (labels, hints and notes drawn by this page's code)

settings-sessions-sidebar = Sidebar
settings-sessions-tabs = Tabs
settings-sessions-stay-with-the-terminal = Stay with the terminal
settings-sessions-follow-the-list = Follow the list
settings-sessions-changes-this-terminal = This terminal
settings-sessions-changes-repository = Repository
settings-sessions-changes-plain-finishing = Commands that finish
settings-sessions-changes-plain-any = Any program
settings-sessions-changes-plain-none = None
settings-sessions-changes-shared-each = Both count it
settings-sessions-changes-shared-none = Neither

## settings_window/sections/preview.rs (labels, hints and notes drawn by this page's code)

settings-sessions-preview-caption-stacked = Live preview of the Terminals list. Hovering a density previews it under the current one
settings-sessions-preview-caption-side-by-side = Live preview of the Terminals list. Hovering a density previews it next to the current one
settings-sessions-terminals = Terminals

## Sentences with a hole in them

# Over the live preview: the Terminals list as it is now. { $density } is the density's name.
settings-sessions-preview-now = Now · { $density }

# Over the live preview: the list as the density under the pointer would draw it.
settings-sessions-preview-hovering = Previewed while hovering { $density }
