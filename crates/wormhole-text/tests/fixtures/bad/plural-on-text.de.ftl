# expect: selects plurally on $section, which English does not count

open = { $section ->
    [one] { $section } öffnen
   *[other] { $section } öffnen
}
