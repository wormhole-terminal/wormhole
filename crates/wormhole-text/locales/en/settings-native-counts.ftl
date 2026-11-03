# The native Settings window's messages that say a number (a plural per language: Fluent picks the form).

# The Changed page's heading. { $count } is a whole number.
settings-native-changed-count = { $count ->
    [one] { $count } setting changed
   *[other] { $count } settings changed
}

# The question before resetting every changed setting. { $count } is a whole number.
settings-native-reset-count = { $count ->
    [one] Reset { $count } setting?
   *[other] Reset { $count } settings?
}
