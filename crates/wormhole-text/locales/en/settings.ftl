# The Settings window: what its pages share. Each page's own words are in settings-<page>.ftl.

## The nav's heading over the pages still being built

settings-nav-preview = Preview

## What VoiceOver reads for a row of the nav: the page's name, then what the row marks

# { $name } is the page's name.
settings-nav-spoken-attention = { $name }, needs attention
# { $name } is the page's name.
settings-nav-spoken-preview = { $name }, preview
# { $name } is the page's name; a search found nothing on it.
settings-nav-spoken-no-matches = { $name }, no matches
# { $name } is the page's name and { $count } how many settings a search found on it.
settings-nav-spoken-matches = { $name }, { $count ->
    [one] 1 match
   *[other] { $count } matches
}

# The name VoiceOver reads for the (i) after a setting. What it opens, the description, is the row's own.
settings-more-information = More information

## A setting's config key, under the pointer or with ⌥ held, and the row's menu

# What the key says for a moment after a click has copied it; VoiceOver says the same.
settings-row-copied = Copied
# The menu item, and the key's name for VoiceOver (the key itself is its description).
settings-row-copy-key = Copy Key
settings-row-copy-json = Copy as JSON
# The menu item. The arrow beside a changed value is named the same, in sentence case.
settings-row-reset-menu = Reset to Default
settings-row-reset = Reset to default
