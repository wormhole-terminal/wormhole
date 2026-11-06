# Settings window, Import page: bringing the font, colours, shell and more over from another terminal, and what
# doesn't carry over. Also the words of the first-run step that does the same in one click.

settings-section-import = Import
    .intro = Bring your font, colours, shell and more over from the terminal you used before. Nothing changes until you choose what to import, and the other terminal's own files are never touched.

## Found on this Mac

settings-import-group-found = Found on this Mac

settings-import-found = Terminals found
    .keywords = import migrate switch move settings config configuration ghostty iterm iterm2 terminal.app alacritty kitty wezterm warp zed vscode vs code font theme colours colors shell

settings-import-scan-again = Scan again
settings-import-scanning = Looking for terminals…
settings-import-none = No other terminal's settings found
settings-import-none-detail = { -brand } looked for Ghostty, iTerm2, Terminal.app, Alacritty, kitty, WezTerm, Warp, Zed and VS Code in the places they keep their settings, and found none.

settings-import-files-read = Files read

# { $count } is a whole number.
settings-import-count = { $count ->
    [one] { $count } setting
   *[other] { $count } settings
}

# { $terminal } is a terminal's name, { $count } how many settings it has ("13 settings"). What a screen reader
# says for the card's header, which opens and closes the card.
settings-import-card-label = { $terminal }, { $count }

settings-import-take = Brings over
settings-import-leave = Doesn't carry over
settings-import-nothing-to-take = Nothing in these settings can be brought over.
settings-import-runs-everywhere = { -brand } would run this in every new terminal, so it isn't ticked until you tick it.

# { $selected } of { $total } rows are ticked.
settings-import-selected = { $selected } of { $total } selected
settings-import-apply = Import selected

# { $brought } is how many settings were brought over and { $left } how many didn't carry over: both whole numbers.
settings-import-summary = { $brought ->
    [one] { $brought } setting brought over, { $left } didn't
   *[other] { $brought } settings brought over, { $left } didn't
}

# { $error } is what went wrong when saving the colours.
settings-import-failed = Couldn't import: { $error }

## The terminals

settings-import-terminal-ghostty = Ghostty
settings-import-terminal-iterm2 = iTerm2
settings-import-terminal-terminal-app = Terminal.app
settings-import-terminal-alacritty = Alacritty
settings-import-terminal-kitty = kitty
settings-import-terminal-wezterm = WezTerm
settings-import-terminal-warp = Warp
settings-import-terminal-zed = Zed
settings-import-terminal-vscode = VS Code

## What a setting is called

settings-import-setting-font-family = Font
settings-import-setting-font-size = Font size
settings-import-setting-line-height = Line height
settings-import-setting-ligatures = Ligatures
settings-import-setting-theme = Colours
settings-import-setting-theme-dark = Colours, dark mode
settings-import-setting-theme-light = Colours, light mode
settings-import-setting-opacity = Opacity
settings-import-setting-blur = Background blur
settings-import-setting-option-as-meta = Option as Meta
settings-import-setting-scrollback = Scrollback lines
settings-import-setting-bell-sound = Bell sound
settings-import-setting-bell-visual = Visual bell
settings-import-setting-default-shell = Shell
settings-import-setting-shell-profile = Shell command

## What it would be set to

settings-import-value-on = On
settings-import-value-off = Off
settings-import-value-meta-left = Left Option key
settings-import-value-meta-right = Right Option key
settings-import-value-meta-both = Both Option keys

## Why a setting doesn't carry over

settings-import-reason-cursor-shape = { -brand } has no setting for the cursor's shape yet.
settings-import-reason-cursor-blink = { -brand } has no setting for a blinking cursor yet.
settings-import-reason-padding = { -brand } has no setting for the space around the text yet.
settings-import-reason-copy-on-select = { -brand } has no copy-on-select setting yet.
settings-import-reason-confirm-close = { -brand } has no setting for confirming before a terminal closes.
settings-import-reason-working-directory = { -brand } has no setting for the folder a new terminal starts in.
settings-import-reason-keybinding = { -brand } has no file for your own key bindings yet.
settings-import-reason-environment = Environment variables are set per profile in Settings, not imported.
settings-import-reason-font-variant = { -brand } has no separate bold or italic fonts, or a second font for other text.
settings-import-reason-cell-width = { -brand } has no setting for letter spacing or cell width.
settings-import-reason-other = { -brand } has no such setting.
settings-import-reason-unsupported-value = { -brand } can't show this value.
settings-import-reason-dynamic-lua = It's computed in Lua, which { -brand } never runs.
settings-import-reason-executes = It runs a program, which an import never does.
settings-import-reason-unreadable = The file couldn't be read: it's missing, too big or not valid.
settings-import-reason-include-cycle = A file that includes itself, so it was read once.
settings-import-reason-include-limit = Files include each other too deep, or there are too many to read.
settings-import-reason-old-format = A settings file in a format that isn't read any more.
settings-import-reason-outside-home = It's outside your home folder, or a link leads out of it, so it wasn't read.
settings-import-reason-partial-palette = It sets only some colours; a theme needs at least a background and a foreground.

# { $font } is the font's name as the other terminal wrote it.
settings-import-reason-font-not-installed = The font { $font } isn't installed on this Mac.

# { $theme } is the name of the theme the other terminal ships inside its app.
settings-import-reason-built-in-theme = { $theme } is built into the other terminal, so its colours aren't in any file.

## First run

# { $terminal } is the name of the terminal whose settings were found.
settings-import-use-mine = Use my { $terminal } settings

# { $brought } and { $left } are whole numbers: the settings brought over and those that didn't carry over.
settings-import-onboarding-summary = { $brought ->
    [one] { $brought } setting brought over, { $left } didn't: Settings → Import lists them
   *[other] { $brought } settings brought over, { $left } didn't: Settings → Import lists them
}

# { $brought } is a whole number.
settings-import-onboarding-all = { $brought ->
    [one] { $brought } setting brought over
   *[other] { $brought } settings brought over
}

## First run, in the terminal welcome (`Wormhole welcome`)

# { $terminal } is the name of the terminal whose settings were found.
settings-import-welcome-found = Your { $terminal } settings were found.

# Shown in a tour that isn't connected to { -brand }, where nothing can be changed.
settings-import-welcome-tour = Settings → Import brings them over.
