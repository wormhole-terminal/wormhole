

## settings_window/sections/mod.rs (labels, hints and notes drawn by this page's code)

settings-window-rescan-path = Rescan PATH
settings-window-new-profile = New profile…
settings-window-import-theme = Import theme

## settings_window/window.rs (labels, hints and notes drawn by this page's code)

settings-window-search-settings = Search settings
settings-window-search = Search
settings-window-no-results = No setting has these words in its name, description or config key.
settings-window-results = Results
settings-window-clears-the-search-then-closes = clears the search, then closes
settings-window-feature-flags = Feature flags
settings-window-open = Open
settings-window-config-json-cant-be-read-so = config.json can't be read, so changes made here aren't saved until it is fixed (see Advanced).
settings-window-some-settings-live-only-in-config = Some settings live only in config.json, and projects can bring their own actions.
settings-window-open-config-json = Open config.json
settings-window-clear-search = Clear search

## settings_window/nav.rs (labels, hints and notes drawn by this page's code)

settings-window-settings = Settings

## settings_window/open.rs (labels, hints and notes drawn by this page's code)


## settings_window/registry.rs (labels, hints and notes drawn by this page's code)

settings-window-edit-in-project = Edit in project

## settings_window/preview_column.rs (labels, hints and notes drawn by this page's code)

settings-window-window = Window
settings-window-fonts = Fonts
settings-window-terminals-list = Terminals list
settings-window-preview-window-label = Preview of the window with your current settings
settings-window-preview-fonts-label = Preview of your fonts in a sample of terminal text
settings-window-preview-terminals-label = Preview of the Terminals list with your current settings
settings-window-preview = Preview
settings-window-preview-column-hint = Shown in the preview column at the right of the window

## Sentences with a hole in them

# The command palette's line that opens a page. { $section } is that page's title, so this is a
# sentence with a hole: keep the word order your language needs.
settings-window-open-section = Open { $section }

# The notice when a theme file could not be imported. { $error } is the reason, in English.
settings-window-import-failed = Import failed: { $error }

# Over the results of a search. { $hits } is how many settings were found, { $sections } in how many pages
# and { $query } what was typed.
settings-window-results-summary = { $hits ->
    [one] { $hits } setting in { $sections ->
        [one] { $sections } section
       *[other] { $sections } sections
    } match “{ $query }”. Pick a section to see it in context.
   *[other] { $hits } settings in { $sections ->
        [one] { $sections } section
       *[other] { $sections } sections
    } match “{ $query }”. Pick a section to see it in context.
}

# The note over a page that is still being built. { $flag } is the feature flag's name.
settings-window-preview-feature-note = Preview feature, behind the “{ $flag }” flag — a release build doesn't have it yet.

# Over the Profile page when the list of profiles was damaged. { $message } is what was wrong, in English.
settings-window-profiles-unreadable = The list of profiles couldn't be read, so it was set aside and rebuilt: { $message }

# The card that shows when a search finds nothing. { $query } is what was typed.
settings-window-nothing-matches = Nothing matches “{ $query }”

# A row's quiet line while a project sets it. { $project } and { $dir } name the project and its
# folder, and { $file } is the project's file that sets it (theme.json or config.json).
settings-window-set-by-project = Set by { $project }/{ $dir }/{ $file } while one of its terminals is in front; yours is kept
