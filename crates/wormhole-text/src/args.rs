//! What a generated function hands to the formatter.

/// A variable's value. The generated functions pick the variant from the English message: a
/// variable a plural selects on is a whole number, any other is text.
pub(crate) enum Arg<'a> {
    Str(&'a str),
    Int(i64),
}
