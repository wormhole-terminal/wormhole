//! The color expression language theme files and the default rules are
//! written in (grammar in the `tokens` module docs).

mod lexer;
mod parser;

use std::fmt;
use std::ops::Range;

use crate::color::Color;

pub(super) use lexer::{Tok, tokenize};
pub(super) use parser::{parse_expr, parse_located};

/// A lexer or parser error and the byte range of the source it is about (an empty range
/// at the end for an expression that stops too early).
#[derive(Clone, Debug, PartialEq)]
pub(in crate::theme::tokens) struct Located {
    pub(in crate::theme::tokens) message: String,
    pub(in crate::theme::tokens) span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::theme::tokens) enum Expr {
    Lit(Color),
    Ref(String),
    Alpha(Box<Expr>, f32),
    Mix(Box<Expr>, Box<Expr>, f32),
    /// The first color composited over the second (source-over).
    Over(Box<Expr>, Box<Expr>),
    /// Signed OKLab lightness delta.
    Lighten(Box<Expr>, f32),
    Shade(Box<Expr>, f32),
    TextOn(Box<Expr>),
    Readable(Box<Expr>, Vec<Expr>, f32),
}

impl Expr {
    pub(in crate::theme::tokens) fn refs<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Expr::Lit(_) => {}
            Expr::Ref(n) => out.push(n),
            Expr::Alpha(e, _) | Expr::Lighten(e, _) | Expr::Shade(e, _) | Expr::TextOn(e) => e.refs(out),
            Expr::Mix(a, b, _) | Expr::Over(a, b) => {
                a.refs(out);
                b.refs(out);
            }
            Expr::Readable(e, bgs, _) => {
                e.refs(out);
                bgs.iter().for_each(|b| b.refs(out));
            }
        }
    }
}

/// An amount as the language writes it: `15%`, with at most two decimals.
fn percent(amount: f32) -> String {
    let text = format!("{:.2}", amount * 100.0);
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{}%", if text.is_empty() { "0" } else { text })
}

/// The expression written back in the language, so that it parses to the same tree: what
/// the Studio shows for an override it only has resolved (one a parent theme made).
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Lit(c) => write!(f, "{}", c.to_hex()),
            Expr::Ref(n) => write!(f, "{n}"),
            // Everything written here is a primary or a chain of `/`s, which `/` extends as is.
            Expr::Alpha(e, a) => write!(f, "{e} / {}", percent(*a)),
            Expr::Mix(a, b, t) => write!(f, "mix({a}, {b}, {})", percent(*t)),
            Expr::Over(a, b) => write!(f, "over({a}, {b})"),
            Expr::Lighten(e, t) if *t < 0.0 => write!(f, "darken({e}, {})", percent(-t)),
            Expr::Lighten(e, t) => write!(f, "lighten({e}, {})", percent(*t)),
            Expr::Shade(e, t) => write!(f, "shade({e}, {})", percent(*t)),
            Expr::TextOn(e) => write!(f, "text_on({e})"),
            Expr::Readable(e, bgs, ratio) => {
                write!(f, "readable({e}")?;
                for bg in bgs {
                    write!(f, ", {bg}")?;
                }
                write!(f, ", {ratio})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_expr, parse_located};

    #[test]
    fn an_expression_is_written_back_as_it_parses() {
        for src in [
            "#7aa2f7",
            "primary / 15%",
            "mix(sidebar.background, primary, 14%)",
            "darken(lighten(#123456, 12.5%), 5%) / 80%",
            "readable(text.muted, sidebar.row.selected, 4.5)",
            "readable(primary, surface.window, surface.panel, 3)",
            "text_on(shade(alpha(primary, 50%), 8%))",
            "over(band, sidebar.background)",
            "mix(a, b, 30%) / 50% / 20%",
        ] {
            let parsed = parse_expr(src).unwrap();
            let written = parsed.to_string();
            assert_eq!(parse_expr(&written).unwrap(), parsed, "{src} -> {written}");
        }
        assert_eq!(parse_expr("primary/0.15").unwrap().to_string(), "primary / 15%");
        assert_eq!(parse_expr("readable(a, b)").unwrap().to_string(), "readable(a, b, 4.5)");
        assert_eq!(parse_expr("mix(a, b, 0%)").unwrap().to_string(), "mix(a, b, 0%)");
    }

    #[test]
    fn errors_point_at_what_is_wrong() {
        let at = |src: &str| parse_located(src).unwrap_err().span;
        // The `)` where the second `,` of mix() should be.
        assert_eq!(at("mix(a, b)"), 8..9);
        assert_eq!(at("a $ b"), 2..3);
        // Stopping too early points just past the end.
        assert_eq!(at("mix(a, b, 10%"), 13..13);
        assert_eq!(at(""), 0..0);
        assert_eq!(at("a b"), 2..3);
        assert_eq!(at("blend(a, b)"), 0..5);
        assert_eq!(at("#12345"), 0..6);
        assert_eq!(at("a / 15"), 4..6);
        assert_eq!(at("readable(a)"), 0..8);
        // The message is the one theme files show.
        let e = parse_located("mix(a, b)").unwrap_err();
        assert_eq!(e.message, "expected `,` in mix(), found `)`");
    }
}
