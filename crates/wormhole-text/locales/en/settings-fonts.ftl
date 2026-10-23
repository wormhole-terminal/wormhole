# Settings window, fonts page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-fonts = Fonts
    .intro = Terminal first: it's what you read most. Code follows it and Interface uses the system font until you pick one.

settings-fonts-font-specimen = Specimen
    .keywords = preview sample

settings-fonts-font-terminal = Font
    .description = The terminal grid
    .keywords = terminal font typeface size monospace jetbrains bundled

settings-fonts-line-height = Line height
    .description = Terminal rows, as a multiple of the font size
    .keywords = spacing leading

settings-fonts-font-ligatures = Ligatures
    .description = Join sequences like => and != into one glyph
    .keywords = ligature

settings-fonts-font-code = Font
    .description = Diffs, commit hashes, file lists, Theme Studio
    .keywords = code font typeface size monospace diff

settings-fonts-font-interface = Font
    .description = Sidebar, panels, settings and controls. The size scales the whole interface
    .keywords = interface font typeface size system ui

settings-fonts-folder = Fonts folder
    .description = JetBrains Mono comes with { -brand }. To use other fonts without installing them, put .ttf, .otf or .ttc files (or folders of them) here: they're listed under Custom within seconds
    .keywords = custom bundled ttf otf ttc folder

settings-fonts-custom-fonts = Custom fonts
    .keywords = custom families

## settings_window/sections/fonts.rs (labels, hints and notes drawn by this page's code)

settings-fonts-open-fonts-folder = Open fonts folder
settings-fonts-none-yet = None yet: the fonts folder is empty.
settings-fonts-ligatures-on = ligatures on
settings-fonts-ligatures-off = ligatures off
settings-fonts-hover-a-font-in-a-list = Hover a font in a list to preview it here

## settings_window/font_picker.rs (labels, hints and notes drawn by this page's code)

settings-fonts-search-fonts = Search fonts
settings-fonts-bundled = Bundled
settings-fonts-loading-installed-fonts = Loading installed fonts…
settings-fonts-finding-monospace-fonts = Finding monospace fonts…
settings-fonts-monospace = Monospace
settings-fonts-all = All

## Sentences with a hole in them

# Under the font pickers: how many fonts the list holds. { $count } is a whole number.
settings-fonts-count = { $count ->
    [one] { $count } font
   *[other] { $count } fonts
}

# Under the Fonts folder row: the families found there. { $count } is a whole number and { $families }
# their names, separated by commas.
settings-fonts-families-found = { $count ->
    [one] { $count } family found: { $families }
   *[other] { $count } families found: { $families }
}

# The font size beside a font picker, in points. { $size } is a whole number already written out.
settings-fonts-size-points = { $size } pt

# The specimen's footer: the font's name and size in points.
settings-fonts-specimen-size = { $family } { $size } pt

# The specimen's footer: the terminal's line height, as a multiple. { $value } is already written out.
settings-fonts-specimen-line-height = line height { $value }

# The specimen's footer: the theme whose colours the specimen is drawn in. { $theme } is its name.
settings-fonts-specimen-colours = { $theme } colours
