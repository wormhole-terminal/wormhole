# Words of the Swift app's Theme Studio, font picker, terminal view and key caps that the GPUI app has no line for.
# The ids follow the areas of the other catalogs (`studio-`, `terminal-`, `notice-`, `settings-fonts-`).

## Theme Studio: the theme menu, the token editor and the token list

studio-save-more = More ways to save
studio-name-field = Theme name
studio-list-label = Tokens
studio-filter-clear = Clear the filter
studio-editor-label = Token editor

# The rule line of a token that follows the terminal's palette (the GPUI app names the theme: studio-rule-palette).
studio-rule-palette-short = From the terminal's palette. Typing a value overrides it.

# The question before a theme file goes to the Trash; the first line is settings-appearance-trash-message.
studio-trash-detail = A theme in use falls back to the default theme, and unsaved edits are dropped.

# What VoiceOver says of a group of tokens in the list: whether it is open.
studio-band-expanded = expanded
studio-band-collapsed = collapsed

# VoiceOver's hint on a group of tokens. { $count } is how many fall short of a contrast minimum.
studio-band-low-hint = { $count ->
    [one] 1 falls short of a contrast minimum
   *[other] { $count } fall short of a contrast minimum
}

# VoiceOver's hint on one token of the list.
studio-row-low-hint = falls short of a contrast minimum

## Theme Studio: contrast

studio-contrast-passes = passes
studio-contrast-falls-short = falls short

# What VoiceOver says of a contrast check's value. { $ratio } is the contrast, { $min } the minimum.
studio-contrast-ratio-spoken = { $ratio } to 1, minimum { $min }

# The suggested fix of a contrast problem. { $ratio } is the minimum ratio, e.g. 4.5.
studio-contrast-fix-lighten = Lighten to { $ratio }:1

# The same, for a text that is lighter than its background. { $ratio } is the minimum ratio, e.g. 4.5.
studio-contrast-fix-darken = Darken to { $ratio }:1

# { $current } is the value now (cut when long), { $ratio } the minimum ratio.
studio-contrast-fix-readable = Use readable({ $current }, …, { $ratio })

## Theme Studio: theme files

studio-folder-too-large = A theme file is at most 256 KiB.

# { $name } is the file's name.
studio-folder-link = { $name } is a link, which is never followed.

## Theme Studio: the gallery

studio-gallery-label = Gallery
studio-gallery-areas = Show the part of the app

# VoiceOver's value of a specimen that draws the selected token.
studio-gallery-uses-selected = uses the selected token
studio-gallery-show-palette = Show the palette
studio-gallery-show-sheet = Show the shortcut sheet
studio-gallery-show-quick-panel = Show the quick panel
studio-gallery-turn-count = Turn the counts over
studio-gallery-play-status = Play a status change

# Under a control of the gallery that macOS draws, not the theme.
studio-gallery-system-tooltip = macOS draws tooltips; a theme does not change them.
studio-gallery-tooltip-here = Rest the pointer here
studio-gallery-tooltip-sample = A tooltip explains a control
studio-gallery-system-menu = macOS draws menus; a theme does not change them.
studio-gallery-show-menu = Open a menu
studio-gallery-system-dialog = macOS draws questions as its own alerts; a theme does not change them.
studio-gallery-show-dialog = Show a question
studio-gallery-system-scrollbar = macOS draws the scroll bars; a theme does not change them.
studio-gallery-system-about = About Wormhole is the system’s own panel; a theme does not change it.
studio-gallery-show-about = Show About Wormhole

## The font picker

# The groups of the list and the tags beside a family.
settings-fonts-source-custom = Custom
settings-fonts-source-installed = Installed
settings-fonts-recent = Recent
settings-fonts-monospace-tag = monospace

# { $query } is what was typed in the search field.
settings-fonts-no-matches = No font has these words in its name: “{ $query }”
settings-fonts-show-all-hint = Show all fonts to include proportional ones

# VoiceOver's label of the list.
settings-fonts-picker = Font list

# The preview under the list and what it says of the font.
settings-fonts-preview = Preview
settings-fonts-coverage = Glyphs this font draws itself
settings-fonts-falls-back = falls back to another font
settings-fonts-drawn = drawn by this font
settings-fonts-box-drawing = Box drawing
settings-fonts-powerline = Powerline
settings-fonts-nerd-icons = Nerd icons
settings-fonts-wide = Wide characters
settings-fonts-bold = Bold
settings-fonts-italic = Italic
settings-fonts-no-bold = No bold face: bold text will be drawn heavier by the system
settings-fonts-no-italic = No italic face: italic text will be slanted by the system

## The terminal view

terminal-accessibility-label = Terminal
terminal-accessibility-scrollback = Scrollback

# Context menu, over a finished command: selects what the command printed.
terminal-menu-select-output = Select Output

# The chip on a finished command that failed. { $code } is its exit code.
terminal-block-exit = exit { $code }

## What VoiceOver says of a command's block
spoken-block-prompt = prompt
spoken-block-running = running
spoken-block-finished = finished
spoken-block-empty = empty prompt

# { $code } is the command's exit code.
spoken-block-failed = failed with exit code { $code }

## The find bar
terminal-find-placeholder = Find
terminal-find-invalid = Not a valid pattern
terminal-find-no-results = No results

# { $at } is the number of the match in view, { $total } how many there are.
terminal-find-count = { $at } of { $total }

# The tooltips carry the keys the actions have.
terminal-find-previous = Previous match (⇧⌘G)
terminal-find-next = Next match (⌘G)
terminal-find-close = Close find (esc)
terminal-find-match-case = Match case
terminal-find-whole-word = Whole word
terminal-find-regex = Regular expression

## The minimap
terminal-minimap = Minimap

# The slider's value for VoiceOver. { $line } is the first line in view, { $total } how many lines there are.
terminal-minimap-value = line { $line } of { $total }

## Saving the output of a command
# The name of the file when the terminal has no title of its own.
terminal-export-default-name = Terminal

## A program asks to read the clipboard
terminal-clipboard-once = Allow once
terminal-clipboard-once-help = Give it the clipboard's text this time
terminal-clipboard-always = Always
terminal-clipboard-always-long = Always for this terminal
terminal-clipboard-always-help = Give it the clipboard whenever it asks, until this terminal closes
terminal-clipboard-deny = Deny
terminal-clipboard-deny-help = Give it nothing

## A terminal whose program has ended
terminal-exited-lost = This terminal ended unexpectedly

# { $shell } is the program's name, e.g. zsh.
terminal-exited = { $shell } exited

# { $shell } is the program's name, { $code } its exit code.
terminal-exited-code = { $shell } exited with code { $code }

terminal-exited-restart = Restart
terminal-exited-close = Close terminal
terminal-host-notice-dismiss = Dismiss

## Key caps, as VoiceOver says them (it reads the glyphs badly)
keycap-control = Control
keycap-option = Option
keycap-shift = Shift
keycap-command = Command
keycap-up-arrow = Up arrow
keycap-down-arrow = Down arrow
keycap-left-arrow = Left arrow
keycap-right-arrow = Right arrow
keycap-delete = Delete
keycap-return = Return
keycap-escape = Escape
keycap-tab = Tab
studio-changed-dot = Changed
