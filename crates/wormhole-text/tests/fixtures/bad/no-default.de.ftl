# expect: does not parse

files = { $count ->
    [one] { $count } Datei
    [other] { $count } Dateien
}
