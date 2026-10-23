# Settings window, menu-bar page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-menu-bar = Menu bar & hotkey
    .intro = { -brand } in the macOS menu bar, and a shortcut that works from any app.

## Menu bar

settings-menu-bar-group-menu-bar = Menu bar

settings-menu-bar-menu-bar = Show { -brand } in the menu bar
    .description = Terminals by status and quick actions
    .keywords = status bar tray item menubar

settings-menu-bar-icon = Icon shows
    .description = The dot and count on the status item
    .keywords = status item icon count

settings-menu-bar-animate = Animate menu bar icon while working
    .description = The prompt in the icon types itself out while something runs. Off: a still, filled icon
    .keywords = animation

## Global hotkey

settings-menu-bar-group-global-hotkey = Global hotkey

settings-menu-bar-hotkey = Shortcut
    .description = Works from any app. Click, then press the keys
    .keywords = global shortcut quake record hotkey

settings-menu-bar-hotkey-action = Hotkey opens
    .description = Pressing it again with the panel open shows { -brand }
    .keywords = quick panel toggle

settings-menu-bar-jump-waiting-key = Jump to next waiting terminal
    .description = Inside { -brand }, in any window
    .keywords = needs you waiting shortcut
