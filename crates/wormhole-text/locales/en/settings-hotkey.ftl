# Settings window, hotkey page: the page's title and intro and the rows on it.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-hotkey = Hotkey
    .intro = A shortcut that works from any app.

settings-hotkey-hotkey = Shortcut
    .description = Works from any app. Click, then press the keys
    .keywords = global shortcut quake record hotkey

settings-hotkey-action = Hotkey opens
    .description = Pressing it again with the panel open shows { -brand }
    .keywords = quick panel toggle

settings-hotkey-jump-waiting-key = Jump to next waiting terminal
    .description = Inside { -brand }, in any window
    .keywords = needs input needs you waiting shortcut

## settings_window/sections/hotkey.rs (labels drawn by this page's code)

settings-hotkey-quick-panel = Quick panel
settings-hotkey-show-or-hide-wormhole = Show or hide { -brand }

## Sentences with a hole in them

# Under the shortcut recorder when the keys are taken. { $conflict } says by what (a sentence, in English
# for now).
settings-hotkey-key-conflict = { $conflict }. Press other keys.

## settings_window/sections/hotkey_preview.rs

settings-hotkey-preview-search = Search terminals and actions
