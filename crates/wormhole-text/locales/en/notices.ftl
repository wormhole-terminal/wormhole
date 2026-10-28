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

## Warnings, which stay on the chip until it is clicked

# The chip when the helper that keeps terminals running couldn't start: this terminal runs inside the app
# and ends when it quits. One line, so it is short; the reason is the next message.
notice-warned-host-fallback = This terminal ends when { -brand } quits

# The same, in full: what VoiceOver says when the chip appears, and the banner of a window whose chip is
# switched off.
notice-warned-host-fallback-detail = This terminal runs inside { -brand } and ends when it quits: the helper that keeps terminals running couldn't start.
