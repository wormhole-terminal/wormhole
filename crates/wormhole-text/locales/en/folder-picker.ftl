# The folder picker at the prompt (⌘J): a list of the folders below the terminal's, with a search field,
# that puts the picked folder's path at the cursor.

# What VoiceOver calls the picker (a dialog).
folder-picker-name = Insert folder path

# The search field's placeholder and VoiceOver label. What is typed is a path: `src/` lists that folder's
# folders, `../` the one above, `~/` the home folder.
folder-picker-search = Search folders

# The one row of a list that has no folders, or none that the typed text narrows it to.
folder-picker-empty = No folders

# The one row when the folder being listed can't be read: it isn't there, isn't a folder, or macOS refuses.
folder-picker-unreadable = Can't read this folder

# The last row of a list that was cut short because the folder has very many folders (or, once something is
# typed, very many that match). { $count } is a number, written with its thousands separator.
folder-picker-cut-folders = Only the first { $count } folders are listed

# The last row of a list that was cut short because the folder has very many entries, folders or not, and
# only the first of them were searched. { $count } is a number, written with its thousands separator.
folder-picker-cut-scanned = Only the first { $count } entries were searched, so some folders may be missing
