# Settings window, Changed page: every setting that differs from its default, in its own control.

settings-section-changed = Changed
    .intro = Every setting that differs from its default, with its default beside it. Reset them one by one, or all at once.

# The line over the list. { $count } is how many settings differ (at least one).
settings-changed-summary = { $count ->
    [one] { $count } setting changed
   *[other] { $count } settings changed
}
settings-changed-summary-detail = Everything else is at its default

# The same line when nothing differs.
settings-changed-none = Every setting is at its default
settings-changed-none-detail = Nothing differs from what Wormhole ships with

settings-changed-copy = Copy as JSON
settings-changed-copied = Copied
settings-changed-reset-all = Reset All…

# Under a row's name: what the setting is by default. { $value } is the default in the words of the row's
# own control.
settings-changed-default = Default: { $value }

# The same with the row's group before it. { $group } is the group's title, { $value } as above.
settings-changed-default-in-group = { $group } · Default: { $value }

# A switch's default.
settings-changed-on = On
settings-changed-off = Off

# The default of a row whose control is not a switch or a menu: nothing is set.
settings-changed-default-empty = none

# The question before everything on the page goes back to its default. { $count } is how many settings.
settings-changed-question = { $count ->
    [one] Reset { $count } setting to its default?
   *[other] Reset { $count } settings to their defaults?
}
settings-changed-question-detail = Your terminals, notes and themes are kept.
settings-changed-question-confirm = Reset
