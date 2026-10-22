# A fixture English: a plain message, a term, a message with a variable, a plural and a reference.

-brand = Wormhole

plain = Plain
    .hint = A hint

# { $section } is a page's title.
open = Open { $section }

# { $count } is a whole number.
files = { $count ->
    [one] { $count } file
   *[other] { $count } files
}

about = About { -brand }

# Shows { $section } twice, once through a reference.
twice = { open }!
