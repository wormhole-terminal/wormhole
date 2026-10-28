# The Files tab: the folder a terminal is in as a tree, its path menu and the menu of a row. A file's or
# folder's own name is never here: it comes from the disk.

## What stands where a folder has nothing to list

files-reading = Reading…
files-empty = Empty folder
files-no-folder = This terminal hasn't said which folder it is in yet.
files-folder-gone = This folder is gone.
files-folder-denied = Wormhole isn't allowed to read this folder.
files-folder-unreadable = This folder can't be read.

# The last line of a folder that has more entries than the tab lists. { $count } is a whole number,
# already formatted.
files-more = { $count } more

## The path menu at the top of the tab

# A line of the path menu: pins the folder the tab shows. { $name } is the folder's name.
files-menu-pin = Pin “{ $name }”

# The same line when the folder is pinned already.
files-menu-unpin = Unpin “{ $name }”

files-menu-choose-folder = Choose Folder…
files-menu-follow-terminal = Follow Terminal

# The system's folder panel: its confirm button.
files-choose-prompt = Choose

## The menu of a row

files-row-open-in = Open in
files-row-reveal = Reveal in Finder
files-row-copy-path = Copy Path
files-row-pin-folder = Pin Folder
files-row-unpin-folder = Unpin Folder

## Read out by VoiceOver

# The button that opens the path menu. { $name } is the folder's name.
files-spoken-path-button = { $name }, folder, opens the path menu

# A row of a folder that is open. { $name } is the folder's name.
files-spoken-folder-open = { $name }, folder, expanded

# A row of a folder that is closed. { $name } is the folder's name.
files-spoken-folder-closed = { $name }, folder, collapsed

# A symbolic link. { $name } is its name.
files-spoken-link = { $name }, link

# After the path is copied.
files-spoken-path-copied = Path copied
