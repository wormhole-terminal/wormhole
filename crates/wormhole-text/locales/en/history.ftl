# The History tab: what an opened commit shows, its buttons and menus, and how it is read out. A commit's
# own words (its message, names, branches) are never here: they come from the repository.

## What the details of an opened commit are called

history-author = Author
history-committer = Committer
history-co-author = Co-author
history-co-authors = Co-authors
history-commit = Commit
history-parent = Parent
history-parents = Parents
history-changes = Changes

# { $count } is a whole number of files a commit changed.
history-files = { $count ->
    [one] { $count } file
   *[other] { $count } files
}

history-reading-details = Reading details…

# Under a message that was cut because it is very long, or that is shown cut.
history-message-continues = The message goes on: copy it to read all of it.

# After the branches and tags shown, how many more point at the commit. { $count } is a whole number.
history-more-refs = +{ $count }

# After the parents shown, how many more a merge has. { $count } is a whole number.
history-more-parents = +{ $count }

# After the short hash in the list: this commit has more than one parent.
history-merge = merge

## When

history-just-now = just now

# How long ago, in a short form: { $age } is "3d" or "12m". The space is a no-break space, so "ago" never
# stands alone on a line.
history-ago = { $age } ago

# A date and how long ago it was. { $date } is "2026-10-20 15:45 +02:00" and { $ago } the line above or
# "just now".
history-date-ago = { $date } · { $ago }

## Buttons, tooltips and menus

history-show-more = Show more
history-show-less = Show less
history-copy-message = Copy message
history-copy-hash = Copy hash
history-more-copy = More ways to copy

history-menu-copy-hash = Copy Hash
history-menu-copy-short-hash = Copy Short Hash
history-menu-copy-message = Copy Message
history-menu-copy-subject = Copy Subject
history-menu-copy-author = Copy Author

# In a commit's menu: open the commit it follows. { $short } is its short hash.
history-menu-open-parent = Open Parent { $short }

## Read out by VoiceOver

# The details block of the opened commit. { $short } is the short hash.
history-spoken-details = Details of { $short }

# { $author } is a name.
history-spoken-by = by { $author }

# { $count } is a whole number of parents, more than one.
history-spoken-parents = { $count } parents

# A parent's link. { $short } is the short hash.
history-open-parent = Open parent { $short }
