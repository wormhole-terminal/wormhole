# Settings window, appearance page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-appearance = Appearance
    .intro = What every window looks like. Changes apply as you make them.

settings-appearance-preview = Live preview
    .keywords = preview wallpaper window

## Theme

settings-appearance-group-theme = Theme

settings-appearance-theme-mode = Mode
    .description = Match system follows macOS: your dark pick in Dark Mode, your light pick in Light Mode
    .keywords = dark light system appearance match night day

settings-appearance-theme = Theme
    .keywords = colour color scheme palette dark light studio import ghostty iterm2 alacritty

## Accent

settings-appearance-group-accent = Accent

settings-appearance-primary-color = Use the theme's primary colour
    .description = Off: a colour of your choice for buttons, focus rings, selection and links, in every theme
    .keywords = accent highlight color colour swatch custom own

settings-appearance-branch-color = Branch colour
    .description = The git branch on folders, in the title row and in the status bar. Any colour is kept readable
    .keywords = git branch chip accent color colour magenta purple primary swatch custom own

# The branch colour's three choices: the theme's own, the primary colour, a colour of your own.
settings-appearance-branch-theme = Theme
settings-appearance-branch-primary = Primary
settings-appearance-branch-custom = Custom

## Window

settings-appearance-group-window = Window

settings-appearance-opacity = Background opacity
    .description = 30–100 %. Below 100 % the desktop shows through
    .keywords = transparency translucent see through

settings-appearance-blur = Blur what's behind the window
    .description = Only visible below 100 % opacity
    .keywords = vibrancy frosted

settings-appearance-liquid-glass = Liquid Glass
    .description = Glass under the sidebar's and the panel's cards. Off: every surface painted.
    .keywords = glass translucent material apple tahoe cards beta


settings-appearance-translucent = Also translucent
    .description = Use the opacity and blur above for these windows too
    .keywords = settings theme studio window

settings-appearance-title-border = Line under the title row
    .description = Between the bar and the terminal: always, only when output is scrolled under the bar, or never
    .keywords = border hairline rule separator title bar toolbar

settings-appearance-column-dividers = Column dividers
    .description = The lines between the sidebar, the terminal and the right panel. Off hides them in every theme; the edges still resize. Increase Contrast keeps them
    .keywords = border hairline rule separator edge columns divider

settings-appearance-status-bar = Show status bar
    .description = Branch, agents and terminal size along the bottom of every window
    .keywords = bottom bar footer

settings-appearance-icon-animations = Animate icons
    .description = Buttons show what a click did with their icon, such as the + turning, Copy turning into a check, or the sidebar button's strip draining as the sidebar slides away. Reduce Motion turns this off too
    .keywords = motion animation toolbar buttons icons sidebar panel toggle

## Title row

settings-appearance-group-title-row = Title row

settings-appearance-title-row = Parts and order
    .description = What the title row shows, and where. Drag a part to move it: the ones above the flexible space sit at the row's left, the others at its right. Right-click the title row to show or hide them there. The pull request is asked of the GitHub CLI (gh), only in a trusted project; hidden, it isn't asked for, and the folder's band in the sidebar has no mark for it either
    .keywords = toolbar customize arrange order reorder rearrange move hide show folder path branch git dirty uncommitted ahead behind status working claude model context window ports localhost server pull request pr github gh checks open in editor split buttons

## File icons

settings-appearance-group-file-icons = File icons

settings-appearance-icon-theme = File icons
    .description = In the Changes and History panels. Zed icon themes work too
    .keywords = icon theme

settings-appearance-icon-themes-folder = Icon themes folder
    .description = Put a Zed icon theme folder here; it's listed above within seconds
    .keywords = zed folder

## Terminal

settings-appearance-group-terminal = Terminal

## Code

settings-appearance-group-code = Code
    .aside = Follows the terminal until you pick one

## Interface

settings-appearance-group-interface = Interface

## Your fonts

settings-appearance-group-your-fonts = Your fonts

## settings_window/sections/appearance.rs (labels, hints and notes drawn by this page's code)

settings-appearance-dark = Dark
settings-appearance-light = Light
settings-appearance-match-system = Match system
settings-appearance-always = Always
settings-appearance-when-scrolled = When scrolled
settings-appearance-never = Never
settings-appearance-open-folder = Open folder
settings-appearance-opaque-reduce-transparency = Reduce Transparency is on in System Settings, so every window is opaque. Your opacity applies again when it's off.
settings-appearance-opaque-increase-contrast = Increase Contrast is on in System Settings, so every window is opaque. Your opacity applies again when it's off.
settings-appearance-blur-off-warning = Without blur the desktop shows sharp behind the text: below 95 % opacity it may not read.
settings-appearance-turn-on-blur = Turn on blur
settings-appearance-liquid-glass-beta = Beta
settings-appearance-glass-held-by-accessibility = Reduce Transparency or Increase Contrast is on in System Settings, so every surface stays painted. The glass returns when it's off.
settings-appearance-settings = Settings
settings-appearance-theme-studio = Theme Studio
settings-appearance-project-settings = Project settings

## The primary colour row's quiet line, and the project theme note

# The line under "Use the theme's primary colour" once a colour was picked. { $color } is its hex code.
settings-appearance-primary-yours = Yours · { $color }, in every theme

# The same line while no colour is picked yet. { $color } is the theme's own primary, as a hex code.
settings-appearance-primary-pick = Pick a colour below · the theme's is { $color }

# The same line while the theme's primary is in use. { $color } is its hex code and { $description } the
# row's own description, which follows after a full stop.
settings-appearance-primary-theme = The theme's · { $color }. { $description }

# The line under "Branch colour" while Custom is chosen and no colour is picked yet. { $color } is the
# theme's own branch colour, as a hex code.
settings-appearance-branch-pick = Pick a colour below · the theme's is { $color }

# Over the theme picker in a project's terminal. { $project } is the project's name.
settings-appearance-project-theme-note = This terminal shows { $project }'s theme. Your own is kept for everything else.

## settings_window/sections/stage.rs (labels, hints and notes drawn by this page's code)

settings-appearance-stage-caption = Live preview: your windows over the desktop, with everything below applied as you change it (the blur behind a window isn't drawn here)

## settings_window/sections/theme_picker/mod.rs (labels, hints and notes drawn by this page's code)

settings-appearance-filter-themes = Filter themes
# Over the theme cards under Match system: the themes of each mode, one of them its pick.
settings-appearance-in-dark-mode = In Dark Mode
settings-appearance-in-light-mode = In Light Mode
settings-appearance-show-fewer = Show fewer
settings-appearance-import-theme = Import theme…
settings-appearance-no-themes-match-the-filter = No themes match the filter.

## Sentences with a hole in them

# The theme filter's placeholder. { $count } is how many themes there are to filter.
settings-appearance-filter-themes-count = { $count ->
    [one] Filter { $count } theme
   *[other] Filter { $count } themes
}

# The link under the theme cards that shows the rest. { $total } is how many cards there are.
settings-appearance-show-all-themes = Show all { $total }

## settings_window/sections/theme_picker (hiding themes)

settings-appearance-hide-theme = Hide theme
settings-appearance-move-to-trash = Move to Trash
settings-appearance-show-theme = Show

# The disclosure under the theme cards that lists the hidden ones. { $count } is how many.
settings-appearance-hidden-themes = Hidden themes ({ $count })
