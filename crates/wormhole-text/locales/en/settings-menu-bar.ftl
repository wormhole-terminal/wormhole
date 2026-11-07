# Settings window, menu-bar page: the page's title and intro and the rows on it.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-menu-bar = Menu bar
    .intro = { -brand } in the macOS menu bar.

settings-menu-bar-menu-bar = Show { -brand } in the menu bar
    .description = Terminals by status and quick actions
    .keywords = status bar tray item menubar

settings-menu-bar-icon = Icon shows
    .description = The dot and count on the status item
    .keywords = status item icon count

settings-menu-bar-animate = Animate menu bar icon while working
    .description = The prompt in the icon types itself out while something runs. Off: a still, filled icon
    .keywords = animation

## settings_window/sections/menu_bar.rs (labels drawn by this page's code)

settings-menu-bar-status-and-count = Status and count
settings-menu-bar-status = Status
settings-menu-bar-icon-only = Icon only

## settings_window/sections/menu_bar_preview.rs

settings-menu-bar-preview-needs-input = Needs input
settings-menu-bar-preview-working = Working
settings-menu-bar-preview-finished = Finished
settings-menu-bar-preview-failed = Failed
