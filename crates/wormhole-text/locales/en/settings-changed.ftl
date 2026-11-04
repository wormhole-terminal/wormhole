# Settings window, Changed page: every setting that differs from its default, in the control of its own page.

settings-section-changed = Changed
    .intro = Every setting that differs from its default, with its own control. Reset one, or all at once.

# The line in the card over the list. { $count } is how many settings differ (at least one).
settings-changed-summary = { $count ->
    [one] { $count } setting changed
   *[other] { $count } settings changed
}
settings-changed-summary-detail = Everything else is at its default

# The same card when nothing differs.
settings-changed-none = No settings changed
settings-changed-none-detail = Everything is at its default

# The button that puts every listed setting back to its default; it asks first.
settings-changed-reset-all = Reset All…
