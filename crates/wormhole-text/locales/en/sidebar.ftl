# The terminal list's groups: what a group's folder asks and says.

## The question the + on a group with no folder asks, as a system alert

# { $group } is the group's name and { $folder } the name of the folder offered, without the path.
sidebar-folder-ask-title = Use “{ $folder }” for “{ $group }”?

# { $path } is the folder's whole path, as the list shows it (`~/Projects/app`).
sidebar-folder-ask-detail =
    { $path }

    New terminals in the group open there.

# The same, when some of the group's terminals are in other folders. { $path } is the folder's whole path
# and { $count } a whole number of terminals (a split's panes each count).
sidebar-folder-ask-detail-leaving = { $count ->
    [one]
        { $path }

        New terminals in the group open there. { $count } terminal is in another folder and would leave the group if the box is ticked.
   *[other]
        { $path }

        New terminals in the group open there. { $count } terminals are in other folders and would leave the group if the box is ticked.
}

# The button Return presses.
sidebar-folder-ask-use = Use Folder

# The button Esc presses: the group isn't asked again.
sidebar-folder-ask-decline = Not Now

sidebar-folder-ask-only = Only keep terminals in this folder

## A group's menu and popover

# A checked item while the group keeps only the terminals in its folder: one that moves to another folder
# leaves the group, and none from outside can be put in.
sidebar-group-only-in-folder = Only terminals in this folder
