# What a terminal itself says on top of its text.

# The chip in the middle of a terminal while its window is being resized and after its text was zoomed:
# the grid's size. { $columns } and { $rows } are numbers of cells; the sign between them is a
# multiplication sign.
terminal-size-readout = { $columns } × { $rows }

# The context menu on a terminal's text (right-click). Its titles follow the Edit menu's where
# they are the same; the key each shows beside it is the action's own.

# Context menu: puts the selected text on the clipboard.
terminal-menu-copy = Copy

# Context menu: types the clipboard's text into the terminal.
terminal-menu-paste = Paste

# Context menu: selects everything the terminal holds.
terminal-menu-select-all = Select All

# Context menu: opens the find bar over the terminal. The ellipsis says a field follows.
terminal-menu-find = Find…

# Context menu, over a link in the text: opens it in the browser or the app that handles it.
terminal-menu-open-link = Open Link

# Context menu, over a link in the text: puts its address on the clipboard.
terminal-menu-copy-link = Copy Link

# Context menu, over a finished command (needs shell integration): puts the command's text on the clipboard.
terminal-menu-copy-command = Copy Command

# Context menu, over a finished command: puts what the command printed on the clipboard.
terminal-menu-copy-output = Copy Output

# Context menu, over a finished command: types the command again at the prompt, for the person to run.
terminal-menu-put-command-at-prompt = Put Command at Prompt

# Context menu: shows a new terminal to the right of this one, in a split.
terminal-menu-split-right = Split Right

# Context menu: shows a new terminal below this one, in a split.
terminal-menu-split-down = Split Down

# Context menu: erases the terminal's history and has the shell draw a clean prompt.
terminal-menu-clear = Clear Buffer
