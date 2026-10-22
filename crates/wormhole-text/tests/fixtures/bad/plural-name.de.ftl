# expect: [plenty] is not a plural category

files = { $count ->
    [one] { $count } Datei
    [plenty] { $count } Dateien
   *[other] { $count } Dateien
}
