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
studio-menu-wormhole = Wormhole
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
# { $from } is the theme this one was copied from.
studio-menu-update-current = Up to date with { $from }
# { $from } is the theme this one was copied from.
studio-menu-update-missing = { $from } is unavailable
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
# { $when } is when the save was made.
studio-versions-original = { $when } · original file
studio-versions-restore = Restore
studio-versions-done = Done
studio-versions-note = Older saves thin out to one an hour, then one a day. The original file is always kept.
studio-versions-empty = No earlier saves yet. Every save keeps the file it replaces.
studio-versions-unreadable = Can’t be read
studio-versions-first = first save
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
studio-newer-duplicate = Duplicate

## Previewing a theme that is not in use

studio-preview-chip = Preview
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

## What saving, exporting and copying say

studio-name-first = Give the theme a name first.
# { $error } is why it failed.
studio-save-failed = Could not save: { $error }
# { $path } is where the theme was written.
studio-saved-set = Saved to { $path } and set as your theme.
# { $path } is where the theme was written.
studio-saved = Saved to { $path }.
# { $name } is the copy's name, { $path } where it was written.
studio-copy-saved = Saved a copy as “{ $name }” to { $path }.
# { $error } is why it failed.
studio-copy-failed = Could not save a copy: { $error }
# { $error } is why it failed.
studio-export-failed = Could not export: { $error }
# { $path } is where the file was written.
studio-exported = Exported to { $path }.
studio-copied-json = Copied the theme's JSON.
# { $appearance } is Light or Dark.
studio-only-shown = Only the { $appearance } appearance of this theme was written.
# { $error } is why it failed.
studio-copy-json-failed = Could not copy: { $error }
studio-other-project = The terminal in front is in another project.
# { $path } is where the theme was written.
studio-project-saved = Saved to { $path }: terminals in this project use it.
# { $error } is why it failed.
studio-project-failed = Could not save to the project: { $error }
studio-no-terminal = Open a terminal in a project first.
studio-no-project-folder = The terminal in front isn't in a project with a .wormhole folder.
studio-reveal = Reveal in Finder

## Questions on the window

# { $name } is the theme's name.
studio-changed-message = “{ $name }” was changed by another application.
studio-changed-detail = Saving replaces those changes. The file as it is now is kept in the theme’s saved versions.
studio-overwrite = Overwrite
# { $name } is the theme's name.
studio-exists-message = “{ $name }” already exists. Do you want to replace it?
studio-exists-detail = A theme with the same name is in your themes folder. Replacing it overwrites that theme with this draft.
studio-replace = Replace
studio-cancel = Cancel
studio-save = Save
studio-dont-save = Don’t Save
# { $name } is the theme's name.
studio-close-message = Do you want to save the changes to “{ $name }”?
# { $count } is how many values changed.
studio-unsaved-detail = { $count ->
    [one] 1 change. Your changes are lost if you don’t save them.
   *[other] { $count } changes. Your changes are lost if you don’t save them.
}

## The title row

studio-title = Theme Studio
studio-unsaved = Unsaved
studio-export = Export…
studio-save-theme = Save theme
studio-export-json = Export JSON…
studio-copy-json = Copy JSON
studio-use-in-project = Use in this project…
# { $from } is the theme it was copied from, { $day } when, { $appearance } Light or Dark.
studio-title-based-on = based on { $from } · copied { $day } · { $appearance }
# { $base } is the theme it extends, { $appearance } Light or Dark.
studio-title-extends = extends { $base } · { $appearance }
# { $from } is the theme it was generated from, { $appearance } Light or Dark.
studio-title-generated = generated from { $from } · { $appearance }
# { $appearance } is Light or Dark.
studio-title-own = its own palette · { $appearance }
# { $day } { $month } { $year } as a date: 29 Oct 2026.
studio-date = { $day } { $month ->
    [one] Jan
    [2] Feb
    [3] Mar
    [4] Apr
    [5] May
    [6] Jun
    [7] Jul
    [8] Aug
    [9] Sep
    [10] Oct
    [11] Nov
    [12] Dec
   *[other] { $month }
} { $year }

## The token editor and the contrast cards

studio-pick-a-token = Pick a token in the list, or click a color in the gallery, to edit it here.
# { $description } says what the token is for, { $count } how many tokens follow it.
studio-followers = { $count ->
    [one] { $description }. 1 token follows it.
   *[other] { $description }. { $count } tokens follow it.
}
studio-tag-last-good = last good value
studio-section-value = Value
studio-srgb-opaque = sRGB · opaque
# { $percent } is the opacity, e.g. 40.
studio-srgb-alpha = sRGB · { $percent }% alpha
# { $source } is another metric, { $value } its size in points.
studio-metric-follow = Follow { $source } ({ $value } px)
studio-metric-help-width = A number of pixels. No color picker for metrics.
studio-metric-help-radius = A number of pixels; a radius above half the height is fully round. No color picker for metrics.
# { $value } is a size in points.
studio-metric-px = { $value } px
# { $value } is a size in points.
studio-metric-preview = Preview at { $value }
# { $count } is how many components use the token.
studio-metric-uses = { $count ->
    [one] 1 use
   *[other] { $count } uses
}
studio-rule-set-here = This theme sets it, so the rule isn't used; Reset goes back to the rule.
studio-rule-fixed = A fixed value. Typing a value overrides it; Reset goes back to the rule.
# { $names } are the tokens it follows, in words.
studio-rule-follows = Follows { $names }. Typing a value overrides it; Reset goes back to the rule.
# { $theme } is the theme whose palette it comes from.
studio-rule-palette = From the palette of { $theme }. Typing a value overrides it.
studio-section-rule = Rule
studio-section-used-by = Used by
studio-used-by-none = No component draws with it directly; what follows it does.
studio-used-by-outlined = Outlined in the gallery while selected.
# { $slot } is 0 to 15.
studio-describe-ansi = Terminal palette color { $slot }
# { $key } is a syntax key, e.g. keyword.
studio-describe-syntax = Code highlighting: { $key }
studio-describe-background = The terminal's background
studio-describe-foreground = The terminal's text
studio-describe-cursor = The terminal's cursor
studio-describe-selection = Selected text in the terminal
# { $rest } are names joined by commas, { $last } the final one.
studio-and-list = { $rest } and { $last }
studio-did-you-mean = Did you mean
studio-forms = Forms
studio-expression-placeholder = #hex or expression, e.g. primary / 15%
# { $name } is a token, { $area } the part of the app the gallery shows.
studio-not-in-area = { $name } isn’t used in { $area }.
# { $name } is a metric, { $value } what was typed.
studio-not-pixels = { $name }: “{ $value }” is not a number of pixels
# { $count } tokens fall short of a contrast minimum.
studio-low-contrast-count = Low contrast · { $count }
studio-contrast-show = Show
# { $token } is the token whose fix also clears this.
studio-contrast-follows = Follows { $token }: its fix clears this too.
# { $background } is the colour it is drawn on.
studio-contrast-on = on { $background }
studio-contrast-on-fill = Contrast on this fill
studio-contrast = Contrast
# { $ratio } is the minimum ratio.
studio-contrast-min = min { $ratio }

## Names a theme gets

# { $base } is the theme it started from.
studio-custom-name = { $base } Custom
# { $name } is the theme copied.
studio-copy-name = { $name } copy
# { $name } is the theme copied, { $n } which copy.
studio-copy-name-n = { $name } copy { $n }
studio-default-name = Theme
