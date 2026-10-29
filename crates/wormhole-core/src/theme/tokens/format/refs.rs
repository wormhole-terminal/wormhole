//! Rewriting the names an expression refers to, token by token.
//!
//! Done on the lexer's tokens, not on the parsed tree and not with a pattern: an expression
//! is written back exactly as it was except for the names that change (its spacing, its
//! `15%` and its `#abc` stay), and a function's name (`mix(`) is never taken for a color's.

use super::super::expr::{Tok, tokenize};

/// What to do with one name an expression refers to.
pub(super) enum Rename {
    Keep,
    /// Write this instead (an expression is wrapped in parentheses by the caller).
    To(String),
    /// The name must not be used here: the message says why.
    Refuse(String),
}

/// `src` with each referenced name passed through `f`. An expression that does not tokenize is
/// returned as it is: reading it is the resolver's job and reports it with its position.
pub(super) fn rewrite_refs(src: &str, f: &dyn Fn(&str) -> Rename) -> Result<String, String> {
    let Ok(tokens) = tokenize(src) else { return Ok(src.to_string()) };
    let mut out = String::with_capacity(src.len());
    let mut at = 0;
    for (ix, (tok, span)) in tokens.iter().enumerate() {
        let Tok::Ident(name) = tok else { continue };
        if matches!(tokens.get(ix + 1), Some((Tok::LParen, _))) {
            continue;
        }
        match f(name) {
            Rename::Keep => {}
            Rename::To(new) => {
                out.push_str(&src[at..span.start]);
                out.push_str(&new);
                at = span.end;
            }
            Rename::Refuse(why) => return Err(why),
        }
    }
    out.push_str(&src[at..]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{Rename, rewrite_refs};

    fn swap(name: &str) -> Rename {
        match name {
            "accent" => Rename::To("primary".into()),
            "no" => Rename::Refuse("no".into()),
            _ => Rename::Keep,
        }
    }

    #[test]
    fn only_names_change_and_the_rest_is_kept_as_written() {
        assert_eq!(
            rewrite_refs("mix( accent ,#abc,  30% ) / 0.5", &swap).unwrap(),
            "mix( primary ,#abc,  30% ) / 0.5"
        );
        assert_eq!(
            rewrite_refs("readable(accent, accent.hover)", &swap).unwrap(),
            "readable(primary, accent.hover)"
        );
        assert_eq!(rewrite_refs("mix(a, no, 1%)", &swap).unwrap_err(), "no");
    }

    #[test]
    fn a_function_is_not_a_reference() {
        let f = |n: &str| if n == "lighten" { Rename::To("x".into()) } else { Rename::Keep };
        assert_eq!(rewrite_refs("lighten(lighten, 5%)", &f).unwrap(), "lighten(x, 5%)");
    }

    #[test]
    fn text_that_does_not_tokenize_is_left_alone() {
        assert_eq!(rewrite_refs("accent $ 1", &swap).unwrap(), "accent $ 1");
    }
}
