//! A Lua lexer, only as far as reading a WezTerm config needs: names, strings, numbers, symbols,
//! and the comments and long strings that must not be mistaken for code. It never evaluates
//! anything; a token is text and a line number.

/// What a token is.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Kind {
    /// An identifier or keyword.
    Name(String),
    /// A string literal, its escapes resolved.
    Str(String),
    Num(f64),
    /// Punctuation, one character or a two- or three-character operator (`==`, `..`).
    Sym(String),
}

/// A token and the line it starts on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Token {
    pub(crate) kind: Kind,
    pub(crate) line: usize,
}

/// Most tokens read from one file; a config is a few hundred.
const MAX_TOKENS: usize = 200_000;

/// The tokens of a Lua source. Text that is not valid Lua yields what could be read: this is not a
/// checker, and an unterminated string or comment ends at the end of the file.
pub(crate) fn tokenize(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() && tokens.len() < MAX_TOKENS {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '-' && chars.get(i + 1) == Some(&'-') {
            i += 2;
            match long_bracket(&chars, i) {
                Some((level, start)) => {
                    let (_, next) = long_text(&chars, start, level);
                    line += chars[i..next].iter().filter(|&&c| c == '\n').count();
                    i = next;
                }
                None => {
                    while i < chars.len() && chars[i] != '\n' {
                        i += 1;
                    }
                }
            }
        } else if c == '[' && long_bracket(&chars, i).is_some() {
            let (level, start) = long_bracket(&chars, i).unwrap_or((0, i));
            let (text, next) = long_text(&chars, start, level);
            tokens.push(Token { kind: Kind::Str(text), line });
            line += chars[i..next].iter().filter(|&&c| c == '\n').count();
            i = next;
        } else if c == '"' || c == '\'' {
            let (text, next, lines) = quoted(&chars, i);
            tokens.push(Token { kind: Kind::Str(text), line });
            line += lines;
            i = next;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let (number, next) = number(&chars, i);
            tokens.push(Token { kind: Kind::Num(number), line });
            i = next;
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token { kind: Kind::Name(chars[start..i].iter().collect()), line });
        } else {
            let three: String = chars[i..chars.len().min(i + 3)].iter().collect();
            let two: String = three.chars().take(2).collect();
            let sym = if three == "..." {
                three
            } else if ["==", "~=", "<=", ">=", "..", "::", "//", "<<", ">>"].contains(&two.as_str()) {
                two
            } else {
                c.to_string()
            };
            i += sym.chars().count();
            tokens.push(Token { kind: Kind::Sym(sym), line });
        }
    }
    tokens
}

/// `[[`, `[=[`, `[==[` at `at`: the level (number of `=`) and where the text starts.
fn long_bracket(chars: &[char], at: usize) -> Option<(usize, usize)> {
    if chars.get(at) != Some(&'[') {
        return None;
    }
    let level = chars[at + 1..].iter().take_while(|&&c| c == '=').count();
    (chars.get(at + 1 + level) == Some(&'[')).then_some((level, at + 2 + level))
}

/// The text of a long bracket from `start` to its closing `]==]`, and where the source goes on.
fn long_text(chars: &[char], start: usize, level: usize) -> (String, usize) {
    let closing: Vec<char> =
        std::iter::once(']').chain(std::iter::repeat_n('=', level)).chain([']']).collect();
    let mut i = start;
    while i + closing.len() <= chars.len() {
        if chars[i..i + closing.len()] == closing[..] {
            return (chars[start..i].iter().collect(), i + closing.len());
        }
        i += 1;
    }
    (chars[start..].iter().collect(), chars.len())
}

/// A quoted string from its opening quote: its text, where the source goes on, and the lines it
/// spans (a `\` before a newline continues it).
fn quoted(chars: &[char], start: usize) -> (String, usize, usize) {
    let quote = chars[start];
    let mut text = String::new();
    let mut lines = 0;
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            c if c == quote => return (text, i + 1, lines),
            // An unterminated string ends at the line's end, as Lua's error would.
            '\n' => return (text, i, lines),
            '\\' => {
                i += 1;
                match chars.get(i) {
                    Some('n') => text.push('\n'),
                    Some('t') => text.push('\t'),
                    Some('r') => text.push('\r'),
                    Some('\n') => {
                        text.push('\n');
                        lines += 1;
                    }
                    Some(&c) => text.push(c),
                    None => {}
                }
            }
            c => text.push(c),
        }
        i += 1;
    }
    (text, i, lines)
}

/// A number from `start`: decimal with a fraction and exponent, or hexadecimal.
fn number(chars: &[char], start: usize) -> (f64, usize) {
    let mut i = start;
    if chars[i] == '0' && matches!(chars.get(i + 1), Some('x' | 'X')) {
        i += 2;
        let digits_from = i;
        while i < chars.len() && chars[i].is_ascii_hexdigit() {
            i += 1;
        }
        let digits: String = chars[digits_from..i].iter().collect();
        return (u64::from_str_radix(&digits, 16).map_or(0.0, |n| n as f64), i);
    }
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }
    if matches!(chars.get(i), Some('e' | 'E')) {
        let mut j = i + 1;
        if matches!(chars.get(j), Some('+' | '-')) {
            j += 1;
        }
        if chars.get(j).is_some_and(char::is_ascii_digit) {
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let text: String = chars[start..i].iter().collect();
    (text.parse().unwrap_or(0.0), i)
}

#[cfg(test)]
mod tests {
    use super::{Kind, tokenize};

    fn kinds(source: &str) -> Vec<Kind> {
        tokenize(source).into_iter().map(|t| t.kind).collect()
    }

    fn name(text: &str) -> Kind {
        Kind::Name(text.into())
    }

    fn sym(text: &str) -> Kind {
        Kind::Sym(text.into())
    }

    #[test]
    fn names_strings_numbers_and_symbols() {
        assert_eq!(
            kinds("config.size = 12.5 -- note\nx = 'a\\'b' .. \"c\""),
            [
                name("config"),
                sym("."),
                name("size"),
                sym("="),
                Kind::Num(12.5),
                name("x"),
                sym("="),
                Kind::Str("a'b".into()),
                sym(".."),
                Kind::Str("c".into()),
            ]
        );
    }

    #[test]
    fn comments_and_long_strings_hide_code_and_keep_line_numbers() {
        let source = "--[==[ config.a = 1\n]] config.b = 2 ]==]\nx = [[long\ntext = 3]]\ny = 4";
        let tokens = tokenize(source);
        let names: Vec<_> = tokens
            .iter()
            .filter_map(|t| if let Kind::Name(n) = &t.kind { Some(n.as_str()) } else { None })
            .collect();
        assert_eq!(names, ["x", "y"]);
        assert_eq!(
            tokens.iter().find(|t| t.kind == Kind::Str("long\ntext = 3".into())).map(|t| t.line),
            Some(3)
        );
        assert_eq!(tokens.last().map(|t| t.line), Some(5));
    }

    #[test]
    fn hex_numbers_exponents_and_unterminated_strings() {
        assert_eq!(kinds("0xff 1e3 .5"), [Kind::Num(255.0), Kind::Num(1000.0), Kind::Num(0.5)]);
        // An unterminated string ends at its line.
        assert_eq!(kinds("'abc\nx"), [Kind::Str("abc".into()), name("x")]);
    }
}
