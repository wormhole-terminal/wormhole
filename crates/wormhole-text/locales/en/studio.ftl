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
