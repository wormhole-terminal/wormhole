# expect: is not a plural category of de

files = { $count ->
    [one] { $count } Datei
    [few] { $count } Dateien
   *[other] { $count } Dateien
}
