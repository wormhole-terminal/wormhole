# The app's questions, as system alerts.

# The button that backs out of a question and does nothing.
dialog-cancel = Cancel

## The question before quitting

# What it says when nothing is kept: the saved terminals come back, what ran in them does not.
dialog-quit-ends = Your terminals come back next time, but what runs in them ends.

# With the box ticked, when every terminal that runs something is kept (it runs in a background helper).
dialog-quit-keeps-all = Your terminals come back next time and what runs in them keeps going. Untick the box to end them.

# With the box, when some terminals are kept and some run inside the app and so end whichever way the box is.
# { $kept } and { $ended } are whole numbers of terminals.
dialog-quit-keeps-some =
    Your terminals come back next time. { $kept ->
        [one] 1 terminal keeps
       *[other] { $kept } terminals keep
    } running; { $ended ->
        [one] 1 terminal ends because it runs
       *[other] { $ended } terminals end because they run
    } inside { -brand }. Untick the box to end the others too.

# Added for a profile that is not shown but has terminals running something. { $count } is a whole number.
dialog-quit-other-profile = { $count ->
    [one] 1 terminal in { $profile } is running an agent or a command.
   *[other] { $count } terminals in { $profile } are running an agent or a command.
}
