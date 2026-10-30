# Settings window, advanced page: the page's title and intro, its groups and the rows in them.
# A row is its label, with the quiet line under it (.description) and the extra words search finds it by (.keywords).

settings-section-advanced = Advanced
    .intro = Files, folders and a way back to the defaults.

## Files

settings-advanced-group-files = Files

settings-advanced-open-config = Settings file
    .description = config.json: edits apply live
    .keywords = json config

settings-advanced-open-data = Data folder
    .description = Terminals, notes, themes, logs
    .keywords = finder database

settings-advanced-open-logs = Logs
    .description = Set WORMHOLE_LOG=debug for more detail
    .keywords = debug log

settings-advanced-reset-settings = Reset all settings
    .description = Your terminals, notes and themes are kept
    .keywords = defaults reset restore

## Tips

settings-advanced-group-tips = Tips

settings-advanced-reset-tips = Closed tips
    .description = Bring back the tips closed with “Don't show again”, such as the sidebar's coding agents note
    .keywords = hints tips reset show again dismissed

## Programs

settings-advanced-group-programs = New programs

settings-advanced-skip-scans = Skip malware scans of new programs
    .description = macOS scans each new program for malware the first time it runs, which slows builds, tests and coding agents. Adding Wormhole to Developer Tools lets programs started in Wormhole skip that scan. Only do this if you trust what you run. Terminals opened after you restart Wormhole get it. If Developer Tools isn't listed, run spctl developer-mode enable-terminal in a terminal first.
    .keywords = developer tools xprotect gatekeeper malware scan speed slow first run build privacy security spctl

## settings_window/sections/advanced.rs (labels, hints and notes drawn by this page's code)

settings-advanced-open = Open
settings-advanced-open-developer-tools = Open Developer Tools Settings
settings-advanced-show-in-finder = Show in Finder
settings-advanced-reset-tips-button = Reset tips
settings-advanced-no-tip-was-closed-for-good = No tip was closed for good
settings-advanced-they-show-again-where-they-belong = They show again where they belong
settings-advanced-reset-to-defaults = Reset to defaults…
settings-advanced-every-setting-is-at-its-default = Every setting is at its default
settings-advanced-asks-first = Asks first
settings-advanced-cancel = Cancel
settings-advanced-reset = Reset

## The reset question

# The question over the Reset button. { $count } is how many settings differ from their defaults (at least
# one: the button is off otherwise).
settings-advanced-reset-question = { $count ->
    [one] Reset { $count } changed setting to its default? Your terminals, notes and themes are kept.
   *[other] Reset { $count } changed settings to their default? Your terminals, notes and themes are kept.
}
