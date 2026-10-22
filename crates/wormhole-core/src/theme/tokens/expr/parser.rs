//! Recursive-descent parser from tokens to [`Expr`].

use std::ops::Range;

use super::lexer::{Spanned, Tok, describe, tokenize};
use super::{Expr, Located};
use crate::color::Color;

/// How deep an expression may nest: calls, parentheses and `/` alphas alike.
/// The built-in rules need 4. A repository's `.wormhole/theme.json` is applied
/// without asking, and without a bound its nesting could overflow the stack while
/// parsing, evaluating or dropping the tree, which aborts the whole app.
pub(in crate::theme::tokens) const MAX_EXPR_DEPTH: usize = 32;

/// Expressions longer than this are shortened when quoted in an error.
const QUOTE_LIMIT: usize = 120;

/// A parsed expression and the depth of its tree.
type Parsed = (Expr, usize);

fn too_deep() -> String {
    format!("expression nests deeper than {MAX_EXPR_DEPTH}")
}

struct Parser {
    toks: Vec<Spanned>,
    pos: usize,
    /// `primary` calls in progress: the parser's own recursion, which
    /// parentheses add to without adding to the tree.
    open: usize,
    /// Where the source ends: what "found end of expression" points at.
    end: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(t, _)| t)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).map(|(t, _)| t.clone());
        self.pos += 1;
        t
    }

    /// Where the token at `index` stands, or the end of the source past the last one.
    fn span_of(&self, index: usize) -> Range<usize> {
        self.toks.get(index).map_or(self.end..self.end, |(_, s)| s.clone())
    }

    /// An error about the token `next` just returned (or the end, if it returned none).
    fn at_last(&self, message: String) -> Located {
        Located { message, span: self.span_of(self.pos.saturating_sub(1)) }
    }

    /// An error about the token `peek` sees.
    fn at_next(&self, message: String) -> Located {
        Located { message, span: self.span_of(self.pos) }
    }

    fn expect(&mut self, want: Tok, what: &str) -> Result<(), Located> {
        match self.next() {
            Some(t) if t == want => Ok(()),
            Some(t) => Err(self.at_last(format!("expected {what}, found {}", describe(&t)))),
            None => Err(self.at_last(format!("expected {what}, found end of expression"))),
        }
    }

    fn expr(&mut self) -> Result<Parsed, Located> {
        let (mut e, mut depth) = self.primary()?;
        while self.peek() == Some(&Tok::Slash) {
            self.next();
            depth += 1;
            if depth > MAX_EXPR_DEPTH {
                return Err(self.at_last(too_deep()));
            }
            e = Expr::Alpha(Box::new(e), self.fraction("alpha after `/`")?);
        }
        Ok((e, depth))
    }

    /// A 0..1 amount written as `15%` or `0.15`.
    fn fraction(&mut self, what: &str) -> Result<f32, Located> {
        let message = match self.next() {
            Some(Tok::Num(v, true)) if (0.0..=100.0).contains(&v) => return Ok(v / 100.0),
            Some(Tok::Num(v, false)) if (0.0..=1.0).contains(&v) => return Ok(v),
            Some(Tok::Num(v, false)) if v <= 100.0 => {
                format!("{what}: {v} is ambiguous, write {v}% or {}", v / 100.0)
            }
            Some(Tok::Num(v, pct)) => format!("{what}: {v}{} is out of range", if pct { "%" } else { "" }),
            Some(t) => format!("{what}: expected a percentage, found {}", describe(&t)),
            None => format!("{what}: expected a percentage"),
        };
        Err(self.at_last(message))
    }

    fn primary(&mut self) -> Result<Parsed, Located> {
        if self.open >= MAX_EXPR_DEPTH {
            return Err(self.at_next(too_deep()));
        }
        self.open += 1;
        let parsed = self.primary_inner();
        self.open -= 1;
        parsed
    }

    fn primary_inner(&mut self) -> Result<Parsed, Located> {
        match self.next() {
            Some(Tok::Hex(h)) => Color::from_hex(&h).map(|c| (Expr::Lit(c), 1)).map_err(|e| self.at_last(e)),
            Some(Tok::LParen) => {
                let parsed = self.expr()?;
                self.expect(Tok::RParen, "`)`")?;
                Ok(parsed)
            }
            Some(Tok::Ident(name)) if self.peek() == Some(&Tok::LParen) => {
                let name_at = self.pos - 1;
                self.next();
                self.call(&name, name_at)
            }
            Some(Tok::Ident(name)) => Ok((Expr::Ref(name), 1)),
            Some(t) => Err(self.at_last(format!("expected a color, found {}", describe(&t)))),
            None => Err(self.at_last("expected a color, found end of expression".into())),
        }
    }

    fn comma(&mut self, func: &str) -> Result<(), Located> {
        self.expect(Tok::Comma, &format!("`,` in {func}()"))
    }

    /// The arguments of `func(`, whose name is the token at `name_at`.
    fn call(&mut self, func: &str, name_at: usize) -> Result<Parsed, Located> {
        let known = ["mix", "over", "lighten", "darken", "alpha", "shade", "text_on", "readable"];
        if !known.contains(&func) {
            return Err(Located {
                message: format!(
                    "unknown function `{func}` (expected mix, over, lighten, darken, alpha, shade, \
                     text_on or readable)"
                ),
                span: self.span_of(name_at),
            });
        }
        let (first, mut deepest) = self.expr()?;
        let first = Box::new(first);
        let e = match func {
            "mix" => {
                self.comma(func)?;
                let (other, depth) = self.expr()?;
                deepest = deepest.max(depth);
                self.comma(func)?;
                Expr::Mix(first, Box::new(other), self.fraction("mix() amount")?)
            }
            "over" => {
                self.comma(func)?;
                let (under, depth) = self.expr()?;
                deepest = deepest.max(depth);
                Expr::Over(first, Box::new(under))
            }
            "lighten" | "darken" | "alpha" | "shade" => {
                self.comma(func)?;
                let amount = self.fraction(&format!("{func}() amount"))?;
                match func {
                    "lighten" => Expr::Lighten(first, amount),
                    "darken" => Expr::Lighten(first, -amount),
                    "alpha" => Expr::Alpha(first, amount),
                    _ => Expr::Shade(first, amount),
                }
            }
            "text_on" => Expr::TextOn(first),
            _ => {
                let mut bgs = Vec::new();
                let mut ratio = 4.5;
                while self.peek() == Some(&Tok::Comma) {
                    self.next();
                    if let Some(Tok::Num(v, false)) = self.peek().cloned() {
                        self.next();
                        if !(1.0..=21.0).contains(&v) {
                            return Err(self.at_last(format!("readable() ratio {v} must be within 1..21")));
                        }
                        ratio = v;
                        break;
                    }
                    let (bg, depth) = self.expr()?;
                    deepest = deepest.max(depth);
                    bgs.push(bg);
                }
                if bgs.is_empty() {
                    return Err(Located {
                        message: "readable() needs at least one background".into(),
                        span: self.span_of(name_at),
                    });
                }
                Expr::Readable(first, bgs, ratio)
            }
        };
        self.expect(Tok::RParen, &format!("`)` closing {func}()"))?;
        if deepest + 1 > MAX_EXPR_DEPTH {
            return Err(self.at_last(too_deep()));
        }
        Ok((e, deepest + 1))
    }
}

/// Parses `src`, saying where it went wrong if it doesn't parse.
pub(in crate::theme::tokens) fn parse_located(src: &str) -> Result<Expr, Located> {
    let mut p = Parser { toks: tokenize(src)?, pos: 0, open: 0, end: src.len() };
    let (e, _) = p.expr()?;
    if let Some(t) = p.peek() {
        return Err(p.at_next(format!("unexpected {} after the color", describe(t))));
    }
    Ok(e)
}

pub(in crate::theme::tokens) fn parse_expr(src: &str) -> Result<Expr, String> {
    parse_located(src).map_err(|e| format!("in {:?}: {}", quoted(src), e.message))
}

/// `src` for an error message, cut short if it is long enough to flood one.
fn quoted(src: &str) -> String {
    match src.char_indices().nth(QUOTE_LIMIT) {
        Some((cut, _)) => format!("{}…", &src[..cut]),
        None => src.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{Expr, parse_expr};
    use crate::tokens::test_support::{hex, on_small_stack};

    #[test]
    fn expression_parsing() {
        let lit = |s: &str| Expr::Lit(hex(s));
        let r = |s: &str| Expr::Ref(s.into());
        assert_eq!(parse_expr("#fff").unwrap(), lit("#fff"));
        assert_eq!(parse_expr(" terminal.ansi.4 ").unwrap(), r("terminal.ansi.4"));
        assert_eq!(parse_expr("primary / 15%").unwrap(), Expr::Alpha(Box::new(r("primary")), 0.15));
        assert_eq!(parse_expr("primary/0.5").unwrap(), Expr::Alpha(Box::new(r("primary")), 0.5));
        assert_eq!(
            parse_expr("mix(a, #000, 30%) / 50%").unwrap(),
            Expr::Alpha(Box::new(Expr::Mix(Box::new(r("a")), Box::new(lit("#000")), 0.3)), 0.5)
        );
        assert_eq!(parse_expr("darken(x, 10%)").unwrap(), Expr::Lighten(Box::new(r("x")), -0.1));
        assert_eq!(parse_expr("(x / 10%)").unwrap(), Expr::Alpha(Box::new(r("x")), 0.1));
        assert_eq!(
            parse_expr("readable(x, a, b, 7)").unwrap(),
            Expr::Readable(Box::new(r("x")), vec![r("a"), r("b")], 7.0)
        );
        assert_eq!(
            parse_expr("readable(x, a / 50%)").unwrap(),
            Expr::Readable(Box::new(r("x")), vec![Expr::Alpha(Box::new(r("a")), 0.5)], 4.5)
        );
        assert!(parse_expr("lighten(mix(text.primary, darken(#123, 5%), 50%), 10%) / 80%").is_ok());
        assert!(parse_expr("text_on(shade(alpha(primary, 50%), 8%))").is_ok());
        assert_eq!(
            parse_expr("over(band, sidebar.background)").unwrap(),
            Expr::Over(Box::new(r("band")), Box::new(r("sidebar.background")))
        );
    }

    #[test]
    fn expression_errors() {
        let err = |s: &str| parse_expr(s).unwrap_err();
        assert!(err("").contains("expected a color"));
        assert!(err("mix(a, b)").contains("`,` in mix()"), "{}", err("mix(a, b)"));
        assert!(err("over(a)").contains("`,` in over()"));
        assert!(err("over(a, b, c)").contains("closing over()"));
        assert!(err("blend(a, b)").contains("unknown function `blend`"));
        assert!(err("a / 15").contains("ambiguous, write 15% or 0.15"));
        assert!(err("a / 150%").contains("out of range"));
        assert!(err("a b").contains("unexpected `b`"));
        assert!(err("mix(a, b, 10%").contains("closing mix()"));
        assert!(err("#12345").contains("invalid hex"));
        assert!(err("a $ b").contains("unexpected character"));
        assert!(err("readable(a)").contains("at least one background"));
        assert!(err("readable(a, b, 40)").contains("1..21"));
        assert!(err("lighten(a, b)").contains("expected a percentage"));
        // The source is quoted for context.
        assert!(err("mix(a, b)").starts_with("in \"mix(a, b)\""));
    }

    #[test]
    fn deep_nesting_is_an_error_not_a_crash() {
        let errors = on_small_stack(|| {
            let parens = format!("{}#000{}", "(".repeat(10_000), ")".repeat(10_000));
            let calls = format!("{}#000{}", "lighten(".repeat(10_000), ", 1%)".repeat(10_000));
            let alphas = format!("#000{}", " / 1%".repeat(10_000));
            // Neither 20 calls nor 20 alphas alone are too deep, but together they are.
            let mixed = format!("{}#000{}", "lighten(".repeat(20), " / 1%, 1%)".repeat(20));
            [parens, calls, alphas, mixed].map(|src| parse_expr(&src).map(|_| ()).unwrap_err())
        });
        for e in errors {
            assert!(e.contains("nests deeper than 32"), "{e}");
        }
        // What the built-in rules need is far inside the limit.
        let deep_enough = format!("{}#000{}", "(".repeat(30), ")".repeat(30));
        assert!(parse_expr(&deep_enough).is_ok());
        assert!(parse_expr(&format!("#000{}", " / 1%".repeat(31))).is_ok());
        assert!(parse_expr(&format!("#000{}", " / 1%".repeat(32))).is_err());
    }
}
