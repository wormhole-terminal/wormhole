# Settings window, terminal page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-terminal = Terminal
    .intro = How the terminal behaves. Its font is in Fonts.

## Behaviour

settings-terminal-group-behaviour = Behaviour

settings-terminal-scrollback = Scrollback
    .description = Lines kept per terminal (applies to new terminals)
    .keywords = history buffer lines

settings-terminal-restore-text = Restore terminal text after restart
    .description = Encrypted and kept only on this Mac. Off erases what's saved
    .keywords = scrollback history saved restore privacy secrets encrypted keychain

settings-terminal-minimap = Show minimap
    .description = The scrollback drawn small along the terminal's right edge. Click or drag it to scroll
    .keywords = overview scrollbar map outline

settings-terminal-split-gaps = Gaps between split terminals
    .description = Each terminal of a split is its own rounded surface, a small gap apart. Off joins them with a hairline
    .keywords = panes divider spacing rounded margin

settings-terminal-meta = Use Option as Meta
    .description = For emacs and readline shortcuts. A side set to Meta can't type @ { "{" } [ on some keyboard layouts
    .keywords = alt option escape keyboard left right

settings-terminal-font = Font
    .keywords = typeface size line height

## Opening

settings-terminal-group-opening = Opening

settings-terminal-open-in = Open folders in
    .description = The Open in… button in a terminal's title row opens its folder here; its menu lists the others
    .keywords = editor ide zed code vscode cursor sublime xcode jetbrains phpstorm finder app

settings-terminal-default-terminal = Default terminal
    .description = Double-clicked .command files and programs run in a new { -brand } terminal instead of Terminal
    .keywords = default terminal app command script executable finder open with launch services

## Shell

settings-terminal-group-shell = Shell

settings-terminal-default-shell = Default shell
    .description = What a new terminal runs: your login shell, a shell on this Mac, or a profile below. Right-click a + for the others
    .keywords = shell zsh bash fish nu nushell login program default

settings-terminal-shells = Shells
    .description = Profiles for New Terminal With: a program with its arguments, listed in config.json
    .keywords = shell profiles custom command arguments args claude program

## Worktrees

settings-terminal-group-worktrees = Worktrees

settings-terminal-worktree-base = Worktrees folder
    .description = Where Shell → New Terminal in New Worktree puts a worktree: beside the repository in <name>-worktrees, or under the folder worktree_base names in config.json
    .keywords = worktree worktrees git branch folder base parallel agents

## Bell

settings-terminal-group-bell = Bell

settings-terminal-bell-sound = Play a sound
    .description = The system beep when a program in a terminal rings the bell
    .keywords = bell beep audible alert

settings-terminal-bell-visual = Flash the window edge
    .description = A short glow along the terminal's edge when it rings the bell
    .keywords = bell visual flash glow

## Privacy

settings-terminal-group-privacy = Privacy

settings-terminal-clipboard-read = Programs reading the clipboard
    .description = What a program gets when it asks for the clipboard's text (OSC 52), over ssh too. Ask puts a question over that terminal; the clipboard often holds a password
    .keywords = osc 52 clipboard read paste copy password ask allow deny security ssh

settings-terminal-clipboard-write = Programs setting the clipboard
    .description = Lets a program put text on the clipboard (OSC 52), which replaces what you copied. The terminal says when one does
    .keywords = osc 52 clipboard write copy set security ssh notice

settings-terminal-screen-checksum = Let programs read the screen back
    .description = Answers a program's request for the checksum of part of the screen (DECRQCRA), which also lets a program, over ssh too, read the screen cell by cell. Applies to new terminals
    .keywords = decrqcra checksum screen read capture esctest security ssh

settings-terminal-window-ops = Let programs control the window
    .description = Lets a program move, resize, minimise, raise or full-screen the window (xterm's window operations). Applies to new terminals
    .keywords = xtwinops window operations resize move iconify fullscreen maximize security ssh

settings-terminal-title-report = Let programs read the window title back
    .description = Answers a program's request for the window title and icon name (XTWINOPS 20 and 21). A title is text a program chose, and the answer is typed into the shell, so over ssh this lets a program type a command line. Applies to new terminals
    .keywords = xtwinops title icon name report read back window security ssh esctest
