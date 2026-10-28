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

# The same when the reader stopped before the end of a very large folder, so there are at least that many.
# { $count } is a whole number, already formatted.
files-more-at-least = { $count }+ more

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

# Turns the row's name into a field, with the name selected.
files-row-rename = Rename…

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

# The name field that stands in a row's place while it is renamed.
files-spoken-rename-field = New name, text field

# After a rename. { $name } is the new name.
files-spoken-renamed = Renamed to { $name }

## The viewer: a file shown in place of the terminal

# The close button's tooltip and VoiceOver label.
files-viewer-close = Close file (esc)

# The viewer's card, for VoiceOver. { $name } is the file's name.
files-spoken-viewer = { $name }, file viewer

# The line under the text of a file that is larger than what is read. { $shown } and { $size } are sizes,
# already written ("2 MB").
files-viewer-truncated = Showing the first { $shown } of { $size }

files-viewer-empty = This file is empty.

# The panel for a file that is not text. { $size } is its size, already written ("48.2 KB").
files-viewer-binary = Binary file, { $size }

# The button on that panel, as Open in… is elsewhere.
files-viewer-open-in = Open in…

files-viewer-image-failed = This picture can't be shown.
files-viewer-image-too-large = This picture is too large to show.
files-viewer-gone = This file was deleted.
files-viewer-outside = This is a link that leads out of the folder, so it isn't shown.
files-viewer-not-file = This isn't a regular file.
files-viewer-denied = Wormhole isn't allowed to read this file.
files-viewer-unreadable = This file can't be read.
