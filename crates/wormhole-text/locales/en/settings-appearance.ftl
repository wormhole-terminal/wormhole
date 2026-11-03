# Settings window, appearance page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-appearance = Appearance
    .intro = What every window looks like. Changes apply as you make them.

settings-appearance-preview = Live preview
    .keywords = preview wallpaper window

## Theme

settings-appearance-group-theme = Theme

settings-appearance-theme-pairing = Choose
    .keywords = pair one theme both dark light separately

# The two ways to choose themes: a dark and light pair on one card, or a card for each.
settings-appearance-pairing-both = One theme for both
settings-appearance-pairing-separate = Dark and light separately

settings-appearance-theme-mode = Mode
    .description = Match system follows macOS: your dark pick in Dark Mode, your light pick in Light Mode
    .keywords = dark light system appearance match night day

settings-appearance-theme = Theme
    .keywords = colour color scheme palette dark light studio import ghostty iterm2 alacritty

## Accent

settings-appearance-group-accent = Accent

settings-appearance-primary-color = Primary colour
    .description = Buttons, focus rings, selection and links, in every theme
    .keywords = accent highlight color colour swatch custom own

settings-appearance-branch-color = Branch colour
    .description = The git branch on folders, in the title row and in the status bar. Any colour is kept readable
    .keywords = git branch chip accent color colour magenta purple primary swatch custom own

## Window

settings-appearance-group-window = Window

settings-appearance-group-details = Details

settings-appearance-opacity = Opacity
    .description = 30–100 %. Below 100 % the desktop shows through
    .keywords = transparency translucent see through

settings-appearance-blur = Blur behind the window
    .description = Below 100 % opacity
    .keywords = vibrancy frosted

settings-appearance-liquid-glass = Liquid Glass
    .description = Glass under the sidebar's and the panel's cards. Off: every surface painted.
    .keywords = glass translucent material apple tahoe cards beta

settings-appearance-translucent = Also translucent
    .description = Use the opacity and blur above for these windows too
    .keywords = settings theme studio window

settings-appearance-borders = Borders
    .description = Lines around cards, columns and the sidebar. Off: surfaces are told apart by their shade alone
    .keywords = lines outline outlines border hairline rule ring frame cards rows controls menus

settings-appearance-column-dividers = Column dividers
    .description = The lines between the sidebar, the terminal and the right panel. Off hides them in every theme; the edges still resize. Increase Contrast keeps them
    .keywords = border hairline rule separator edge columns divider

settings-appearance-status-bar = Status bar
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

settings-appearance-icon-theme = Icon theme
    .description = Zed icon themes work too
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

## settings_window/sections/appearance.rs (labels, hints and notes drawn by this page's code)

settings-appearance-dark = Dark
settings-appearance-light = Light
settings-appearance-match-system = Match system

## The primary colour row's quiet line, and the project theme note

## settings_window/sections/stage.rs (labels, hints and notes drawn by this page's code)

## settings_window/sections/theme_picker/mod.rs (labels, hints and notes drawn by this page's code)

## Sentences with a hole in them

## settings_window/sections/theme_picker (hiding themes)

settings-appearance-move-to-trash = Move to Trash
# The question before a theme file goes to the Trash (the Settings card and the Theme Studio both ask it).
# { $name } is the theme.
settings-appearance-trash-message = Move “{ $name }” to the Trash?
settings-appearance-trash-detail = You can get it back from the Trash.
# Which pick changes when the theme is one: { $to } is the theme the app falls back to.
settings-appearance-trash-dark = Your dark theme changes to { $to }.
# { $to } is the theme the app falls back to.
settings-appearance-trash-light = Your light theme changes to { $to }.
# { $dark } and { $light } are the themes the app falls back to.
settings-appearance-trash-both = Your dark theme changes to { $dark } and your light theme to { $light }.
settings-appearance-trash-edits = Its unsaved changes are lost.
# { $name } is the theme.
settings-appearance-trash-failed = “{ $name }” could not be moved to the Trash.

