//! Splits an expression into tokens, each with where it stands in the source.

use std::ops::Range;

use super::Located;

#[derive(Clone, Debug, PartialEq)]
pub(in crate::tokens) enum Tok {
    Hex(String),
    Ident(String),
    Num(f32, bool),
    LParen,
    RParen,
    Comma,
    Slash,
}

/// A token and its byte range in the source.
pub(in crate::tokens) type Spanned = (Tok, Range<usize>);

pub(in crate::tokens) fn tokenize(src: &str) -> Result<Vec<Spanned>, Located> {
    let mut out = Vec::new();
    let mut chars = src.char_indices().peekable();
    // The text from `start` while `f` holds, and the end of it.
    let take_while =
        |chars: &mut std::iter::Peekable<std::str::CharIndices>, start: usize, f: fn(char) -> bool| {
            let mut end = start;
            while let Some(&(i, c)) = chars.peek().filter(|(_, c)| f(*c)) {
                end = i + c.len_utf8();
                chars.next();
            }
            (src[start..end].to_string(), end)
        };
    while let Some(&(start, c)) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            '(' | ')' | ',' | '/' => {
                chars.next();
                let tok = match c {
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    ',' => Tok::Comma,
                    _ => Tok::Slash,
                };
                out.push((tok, start..start + 1));
            }
            '#' => {
                chars.next();
                let (hex, end) = take_while(&mut chars, start + 1, |c| c.is_ascii_alphanumeric());
                out.push((Tok::Hex(hex), start..end));
            }
            '0'..='9' | '.' => {
                let (s, mut end) = take_while(&mut chars, start, |c| c.is_ascii_digit() || c == '.');
                let v: f32 = s
                    .parse()
                    .map_err(|_| Located { message: format!("bad number {s:?}"), span: start..end })?;
                let pct = chars.next_if(|(_, c)| *c == '%').is_some();
                end += usize::from(pct);
                out.push((Tok::Num(v, pct), start..end));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let (name, end) = take_while(&mut chars, start, |c| {
                    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')
                });
                out.push((Tok::Ident(name), start..end));
            }
            _ => {
                return Err(Located {
                    message: format!("unexpected character {c:?}"),
                    span: start..start + c.len_utf8(),
                });
            }
        }
    }
    Ok(out)
}

pub(super) fn describe(t: &Tok) -> String {
    match t {
        Tok::Hex(h) => format!("`#{h}`"),
        Tok::Ident(s) => format!("`{s}`"),
        Tok::Num(v, true) => format!("`{v}%`"),
        Tok::Num(v, false) => format!("`{v}`"),
        Tok::LParen => "`(`".into(),
        Tok::RParen => "`)`".into(),
        Tok::Comma => "`,`".into(),
        Tok::Slash => "`/`".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Tok, tokenize};

    #[test]
    fn tokens_know_where_they_stand() {
        let toks = tokenize("mix(a.b, #fff, 30%)").unwrap();
        let spans: Vec<_> = toks.iter().map(|(_, s)| s.clone()).collect();
        assert_eq!(spans, [0..3, 3..4, 4..7, 7..8, 9..13, 13..14, 15..18, 18..19]);
        assert_eq!(toks[6].0, Tok::Num(30.0, true));
        let err = tokenize("a $ b").unwrap_err();
        assert_eq!(err.span, 2..3);
        // Positions are bytes, so a wide character before an error still lines up.
        assert_eq!(tokenize("a é").unwrap_err().span, 2..4);
    }
}
