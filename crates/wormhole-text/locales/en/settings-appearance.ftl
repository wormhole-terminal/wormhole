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

## Window

settings-appearance-group-window = Window

settings-appearance-opacity = Background opacity
    .description = 30–100 %. Below 100 % the desktop shows through
    .keywords = transparency translucent see through

settings-appearance-blur = Blur what's behind the window
    .description = Only visible below 100 % opacity
    .keywords = vibrancy frosted

settings-appearance-liquid-glass = Liquid Glass
    .description = Glass under the sidebar's and the panel's cards on macOS 26 and later. Off: every surface painted.
    .keywords = glass translucent material apple tahoe cards

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

## Title row

settings-appearance-group-title-row = Title row

settings-appearance-title-branch-status = Branch status
    .description = A dot while the tree has uncommitted changes, and how many commits the branch is ahead of or behind its upstream (↑2 ↓1)
    .keywords = git dirty uncommitted ahead behind upstream push pull sync commits chip

settings-appearance-title-agent-gauge = Agent model and context
    .description = The model a coding agent runs and how much of its context window is used, when the agent reports it (Claude Code does, through its status line)
    .keywords = claude model context window tokens percent compact meter usage

settings-appearance-title-ports = Open ports
    .description = The ports a terminal's programs listen on, as chips that open localhost in your browser
    .keywords = localhost server dev http listen browser network

settings-appearance-title-pull-request = Pull request
    .description = The branch's pull request and how its checks and reviews stand: a link in the title row and a mark on the folder's band in the sidebar, with a notification when its checks finish. Asked of the GitHub CLI (gh), only in a trusted project, and only when gh is installed and signed in
    .keywords = github gh pr merge review link ci checks actions build failed passed draft approved

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
