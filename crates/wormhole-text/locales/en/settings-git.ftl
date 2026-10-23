# Settings window, git page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-git = Changes & diffs
    .intro = The Changes panel and the diff view.

settings-git-changes-view = File view
    .description = A flat list, or folders as a tree
    .keywords = folders files list tree

## Group untracked files separately

settings-git-group-untracked = Group untracked files separately
    .description = New files get their own section under the changes
    .keywords = untracked new files

settings-git-diff-layout = Default layout
    .description = Split keeps old and new side by side, even for a new or deleted file. The diff's own toolbar switches one diff
    .keywords = split unified side by side

settings-git-diff-wrap = Wrap long lines
    .description = Otherwise the diff scrolls sideways
    .keywords = wrap

settings-git-tab-width = Tab width
    .description = Spaces per tab in diffs
    .keywords = indent tabs

settings-git-size-units = File sizes
    .description = Decimal like Finder (12.4 MB), or binary like git (11.8 MiB)
    .keywords = size units bytes kilobytes megabytes kb mb kib mib finder binary decimal

## settings_window/sections/git.rs (labels, hints and notes drawn by this page's code)

settings-git-list = List
settings-git-tree = Tree
settings-git-split = Split
settings-git-unified = Unified
settings-git-mb = MB
settings-git-mib = MiB

## Sentences with a hole in them

# The tab width stepper's value. { $count } is a whole number of spaces.
settings-git-tab-width-value = { $count ->
    [one] { $count } space
   *[other] { $count } spaces
}
