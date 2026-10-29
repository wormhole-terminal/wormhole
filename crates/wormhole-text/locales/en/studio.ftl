# The Theme Studio: the window where a theme's colours and metrics are edited.

## The Edit menu's Undo and Redo while the Studio is the window in front
## ("Undo Change “primary”"; the menu puts the word before).

# { $token } is the name of a token, e.g. primary.
studio-undo-change = Change “{ $token }”
# { $token } is the name of a token, e.g. primary.
studio-undo-reset = Reset “{ $token }”
studio-undo-rename = Rename
studio-undo-revert = Revert to Saved

## Opening on a built-in theme

# { $from } is the name of the theme it starts from, e.g. Nord.
studio-untitled-from = Untitled (from { $from })

# Under the title row when a theme with no name yet is saved.
studio-name-prompt = Name your theme, then press Return.

## The theme menu: the name in the title row opens it

studio-menu-your-themes = Your Themes
studio-menu-this-project = This Project
studio-menu-project-theme = The project’s theme
studio-menu-built-in = Built-in
studio-menu-new = New Theme…
# { $name } is the name of the theme being edited.
studio-menu-duplicate = Duplicate “{ $name }”
studio-menu-rename = Rename…
studio-menu-revert = Revert to Saved
studio-tooltip-menu = Open a theme, or duplicate, rename and revert this one

## Opening another theme

# { $current } is the theme being edited, { $next } the one that was chosen.
studio-switch-message = Do you want to save the changes to “{ $current }” before opening “{ $next }”?
studio-open-failed = That theme could not be opened.

## Saving

# Added to "Saved to …" when a theme in the older file format is saved: the file is in the new format.
studio-saved-converted = Saved in the new theme format: older releases of Wormhole can’t open this file. The previous file is kept in the theme’s saved versions.
# Added when the file that was replaced had comments.
studio-saved-comments = Comments in the file were not kept.
studio-convert-failed = Could not convert this theme safely, so it was not changed.
# Saving under the name of one version of a theme that ships several (a light and a dark one): a file
# of several versions is never written over by the Studio. { $name } is the name.
studio-name-is-a-version = “{ $name }” is the name of one version of a theme that comes in several. Choose another name.
# Saving or renaming to the name another theme has. { $name } is the name.
studio-name-taken = Another theme is already called “{ $name }”. Choose another name.
# Saving a theme file made by a newer Wormhole: this one would drop what it doesn’t understand.
studio-newer-file = This theme was made by a newer Wormhole. Saving here would lose what this version doesn’t understand, so it was not changed. Duplicate keeps a copy.


## The token list: where each colour comes from

studio-filter-placeholder = Filter tokens
studio-show-all = All
# { $count } is how many tokens the theme sets itself.
studio-show-yours = Yours · { $count }
# { $count } is how many tokens are still as they were copied.
studio-show-base = Base · { $count }
# { $count } is how many tokens fall short of a contrast minimum.
studio-show-low = Low contrast · { $count }
studio-origin-yours = yours
studio-origin-base = base
studio-origin-worked-out = worked out
studio-origin-tip-yours = This theme sets this value itself.
# { $from } is the theme this one was copied from.
studio-origin-tip-base = Copied from { $from } and not changed. Editing it makes it yours.
studio-origin-tip-worked-out = Wormhole works this out from other colours, so it follows your edits. Editing it makes it yours.
studio-reset = Reset
studio-reset-base = Reset to base

## A theme with a light and a dark appearance

studio-appearance-light = Light
studio-appearance-dark = Dark
studio-appearance-label = Appearance
studio-reach-both = both
studio-reach-light = light
studio-reach-dark = dark
# { $appearance } is Light or Dark.
studio-reach-tip-both = Both appearances use this value. Click to set it for { $appearance } only.
# { $appearance } is Light or Dark.
studio-reach-tip-this = Only { $appearance } uses this value. Click to set it for both appearances.
studio-menu-add-dark = Add Dark Appearance
studio-menu-add-light = Add Light Appearance
studio-add-failed = Could not add an appearance to this theme.
# { $kept } is Light or Dark, { $removed } the other one; { $token } a token, { $value } the removed value.
studio-moved-kept = Now set for both: kept the { $kept } value of “{ $token }”. The { $removed } value ({ $value }) was removed.
studio-moved-undo = Undo
studio-undo-add-appearance = Add Appearance
# { $token } is the name of a token.
studio-undo-move = Move “{ $token }”
studio-use-with-match-system = Use with Match system
# { $name } is the theme's name.
studio-pair-used = “{ $name }” follows the system’s light and dark.

## Update from the theme it was copied from

# { $from } is the theme this one was copied from.
studio-update-from = Update from { $from }…
studio-update-up-to-date = Up to date
studio-update-source-missing = Source unavailable
# { $from } is the theme this one was copied from.
studio-update-title = Update from { $from }?
# { $count } values changed; { $date } is the day it was copied.
studio-update-count = { $count ->
    [one] 1 value changed since you copied it on { $date }. Your own changes stay.
   *[other] { $count } values changed since you copied it on { $date }. Your own changes stay.
}
studio-update-kept = yours · kept
studio-update-cancel = Cancel
studio-update-apply = Update
studio-undo-update-from = Update from Base

## Versions: earlier saves of a theme

studio-menu-revert-to = Revert To
studio-menu-versions = Browse All Versions…
# { $name } is the theme's name.
studio-versions-title = Versions of “{ $name }”
studio-versions-current = current
studio-versions-original = original file
studio-versions-restore = Restore
studio-versions-done = Done
studio-versions-note = Older saves thin out to one an hour, then one a day. The original file is always kept.
studio-versions-empty = No earlier saves yet. Every save keeps the file it replaces.
studio-versions-unreadable = Can’t be read
studio-versions-first = first save
# { $count } values differ from the save before it.
studio-versions-changed = { $count ->
    [one] 1 value changed
   *[other] { $count } values changed
}
# { $when } is a date and time; { $what } says how it differs from the save before.
studio-versions-label = Version from { $when }, { $what }
studio-undo-restore = Restore Version

## A theme made with a newer Wormhole

studio-newer-chip = Made with a newer Wormhole
# { $count } entries were skipped.
studio-newer-explain = { $count ->
    [one] 1 value this version doesn’t know was skipped.
   *[other] { $count } values this version doesn’t know were skipped.
} It shows what it can; the rest is worked out. Saving would drop them, so Theme Studio won’t save over it.
studio-newer-save-blocked = This theme was made with a newer Wormhole; saving would drop what this version doesn’t know. Duplicate it to keep your edits.
studio-newer-duplicate = Duplicate

## Previewing a theme that is not in use

studio-preview-chip = Previewing, not in use
studio-preview-use = Use
studio-preview-tooltip = Every window shows this theme while Theme Studio is open. Closing Theme Studio puts your theme back.
studio-versions-just-now = just now
# { $count } minutes ago.
studio-versions-minutes-ago = { $count ->
    [one] 1 minute ago
   *[other] { $count } minutes ago
}
# { $count } hours ago.
studio-versions-hours-ago = { $count ->
    [one] 1 hour ago
   *[other] { $count } hours ago
}

## New themes

studio-menu-new-from = New Theme From
# The name a new theme has until it is named.
studio-untitled = Untitled
