# The Settings window: the section titles in the nav and over each page, and a few messages with variables.


# The Mode row of Appearance. The description is the quiet line under the label, the keywords find
# the row in search besides its label and description.
settings-theme-mode = Mode
    .description = Match system follows macOS: your dark pick in Dark Mode, your light pick in Light Mode
    .keywords = dark light system appearance match night day

# The command palette's line that opens a page. { $section } is that page's title, so this is a
# sentence with a hole: keep the word order your language needs.
settings-open-section = Open { $section }

# Under the font pickers: how many fonts the folder holds. { $count } is a whole number.
settings-font-count = { $count ->
    [one] { $count } font
   *[other] { $count } fonts
}

# The tab width stepper's value. { $count } is a whole number of spaces.
settings-tab-width = { $count ->
    [one] { $count } space
   *[other] { $count } spaces
}

# The line under the title of a page that fills in from { -brand }'s defaults.
settings-about-defaults = Defaults of { -brand } for this profile
