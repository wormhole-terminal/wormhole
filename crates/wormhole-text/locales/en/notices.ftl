# The notice chip: the short messages that drop onto the top of a terminal ("Copied 3 lines"). What was
# copied is never in them, only what it was and how much. VoiceOver says the same words.

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
