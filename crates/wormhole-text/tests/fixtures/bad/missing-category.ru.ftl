# expect: lacks [few]

files = { $count ->
    [one] { $count } файл
   *[other] { $count } файлов
}
