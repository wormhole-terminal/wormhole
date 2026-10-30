# The notice chip: the short messages that drop onto the top of a terminal ("Copied 3 lines"). What was
# copied is never in them, only what it was and how much. VoiceOver says the same words, except where a
# message has a longer one beside it.

## What was copied

# Text from a terminal's screen, or a program's write to the clipboard, of several lines. { $count } is a
# whole number.
notice-copied-lines = { $count ->
    [one] Copied { $count } line
   *[other] Copied { $count } lines
}

# The same, of a single line. { $count } is a whole number of characters.
notice-copied-characters = { $count ->
    [one] Copied { $count } character
   *[other] Copied { $count } characters
}

# A command block's output. { $count } is a whole number of lines.
notice-copied-output = { $count ->
    [one] Copied output, { $count } line
   *[other] Copied output, { $count } lines
}

# The same two, when a program in a terminal the person isn't typing in wrote to the clipboard: the chip is on
# that terminal, so it is a program of it. { $count } is a whole number of lines.
notice-copied-program-lines = { $count ->
    [one] A program copied { $count } line
   *[other] A program copied { $count } lines
}

# { $count } is a whole number of characters.
notice-copied-program-characters = { $count ->
    [one] A program copied { $count } character
   *[other] A program copied { $count } characters
}

# The same, when the program is known: { $program } is its name as the sidebar shows it, e.g. ssh. { $count }
# is a whole number of lines.
notice-copied-named-lines = { $count ->
    [one] { $program } copied { $count } line
   *[other] { $program } copied { $count } lines
}

# { $program } is the program's name, e.g. ssh. { $count } is a whole number of characters.
notice-copied-named-characters = { $count ->
    [one] { $program } copied { $count } character
   *[other] { $program } copied { $count } characters
}

notice-copied-command = Copied command

notice-copied-path = Copied path

notice-copied-setting-key = Copied setting key

notice-copied-error = Copied error

notice-copied-object-id = Copied object id

## What a program that controls { -brand } is doing to the terminal

# A coding agent types, or has a command run, in the terminal the chip is on. { $name } is the agent's name,
# e.g. Claude Code.
notice-driven-typing = { $name } is typing

# A coding agent reads what the terminal shows. { $name } is the agent's name, e.g. Claude Code.
notice-driven-reading = { $name } is reading

notice-driven-scripts-typing = A script is typing

notice-driven-scripts-reading = A script is reading

# A commit in the History tab: its full hash, the short one, its whole message, its subject line, its author.
notice-copied-commit-hash = Copied commit hash

notice-copied-commit-short-hash = Copied short hash

notice-copied-commit-message = Copied commit message

# The message was longer than Wormhole reads, so what was copied is its start.
notice-copied-commit-message-cut = Copied the start of the message, which is cut

notice-copied-commit-subject = Copied subject

notice-copied-commit-author = Copied author

## What went wrong

# A copy that needed the commit's details, which git could not read.
notice-failed-copy-commit = Couldn't copy: git could not read the commit

# A path that has a control character in its name (a newline would run what follows it when pasted into
# a terminal), or that is not text.
notice-failed-copy-path = Not copied: the path has characters that would act when pasted

# A rename in the Files tab that was refused: the name can't be one (empty, a slash, a control
# character, too long).
notice-failed-rename-name = Not renamed: that isn't a usable name

# The same, because something is called that already.
notice-failed-rename-exists = Not renamed: something with that name exists already

notice-failed-rename-denied = Not renamed: Wormhole isn't allowed to change it

# The file or folder, or the folder it is in, is gone, or the path leads through a link.
notice-failed-rename-gone = Not renamed: it isn't there any more

notice-failed-rename-failed = Couldn't rename it

# A finished command that Put at Prompt refused to type, because of what it would do there. The four that
# follow say why, each after the same start; they are short, as the chip cuts a long line.
notice-failed-put-concealed = Not put at the prompt: part of it is concealed

notice-failed-put-same-color = Not put at the prompt: part of it is in its background's color

notice-failed-put-invisible = Not put at the prompt: it has characters you can't see

# The shell doesn't take a paste as one piece, so a command of several lines would run line by line.
notice-failed-put-lines = Not put at the prompt: each line would run in this shell

## Warnings, which stay on the chip until it is clicked

# The chip when the helper that keeps terminals running couldn't start: this terminal runs inside the app
# and ends when it quits. One line, so it is short; the reason is the next message.
notice-warned-host-fallback = This terminal ends when { -brand } quits

# The same, in full: what VoiceOver says when the chip appears, and the banner of a window whose chip is
# switched off.
notice-warned-host-fallback-detail = This terminal runs inside { -brand } and ends when it quits: the helper that keeps terminals running couldn't start.

# The chip when an update is installed and waits for a restart; a click on it restarts. { $version } is the
# new version's number, e.g. 1.6.0.
notice-warned-update-ready = { -brand } { $version } is ready — Restart

## The buttons of a warning that stays

# The × on the chip, which closes the warning for good. Its tooltip, and what VoiceOver calls it.
notice-close = Close

# The ⓘ on the chip, which opens the explanation of the warning. Its tooltip, and what VoiceOver calls it.
notice-info = More about this warning

## What the explanation says when a terminal's helper could not start

# The heading of the explanation, which opens from the ⓘ or a click on the chip.
notice-info-host-heading = This terminal can't outlive { -brand }

# What happened and what it means. The helper is the small program that keeps a terminal running when
# { -brand } quits (the session host); the main text avoids that word.
notice-info-host-what = { -brand } normally keeps each terminal running in a small helper, so your work carries on after you quit { -brand } and is waiting when you open it again. The helper for this terminal couldn't start, so the terminal runs inside { -brand } instead and closes when { -brand } quits.

# That the problem is this terminal's alone.
notice-info-host-others = Other terminals that already run in a helper are not affected.

# What to do. A new terminal does try to start a helper again; the warning is said once each time
# { -brand } starts.
notice-info-host-try = Finish what is running here before you quit. Each new terminal tries to start its helper again, and { -brand } says this once each time it starts.

# Above the line with the error's own words, which can be selected and copied.
notice-info-host-reason = What went wrong, if you want to report it:

# The button that shows the folder with the log files in Finder.
notice-info-logs = Open Logs Folder

# The button that copies the error's words, and what it says for a moment after.
notice-info-copy = Copy reason

notice-info-copied = Copied
