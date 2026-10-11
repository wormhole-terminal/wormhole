//! A small JSON reader and the string/number writers, for the results files only: std only, so
//! it handles what `ttystorm --save` writes (and any well-formed JSON) and nothing fancier.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let Value::Num(n) = self { Some(*n) } else { None }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self { Some(s) } else { None }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self { Some(*b) } else { None }
    }

    pub fn as_arr(&self) -> Option<&[Value]> {
        if let Value::Arr(a) = self { Some(a) } else { None }
    }
}

/// A string as a JSON literal, quotes included.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A number rounded to three decimals; JSON has no NaN or infinity, so those become 0.
pub fn number(n: f64) -> String {
    if !n.is_finite() {
        return "0".to_string();
    }
    let r = (n * 1000.0).round() / 1000.0;
    format!("{r}")
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(format!("unexpected text after the value at byte {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

/// Deep enough for any results file; stops a hostile one from overflowing the stack.
const MAX_DEPTH: usize = 32;

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, b: u8) -> Result<(), String> {
        if self.s.get(self.i) == Some(&b) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{}` at byte {}", b as char, self.i))
        }
    }

    fn word(&mut self, w: &str, v: Value) -> Result<Value, String> {
        if self.s[self.i..].starts_with(w.as_bytes()) {
            self.i += w.len();
            Ok(v)
        } else {
            Err(format!("unexpected text at byte {}", self.i))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("nested too deeply".to_string());
        }
        match self.s.get(self.i) {
            None => Err("unexpected end".to_string()),
            Some(b'{') => {
                self.i += 1;
                let mut members = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Value::Obj(members));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    self.eat(b':')?;
                    self.ws();
                    members.push((key, self.value(depth + 1)?));
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Value::Obj(members));
                        }
                        _ => return Err(format!("expected `,` or `}}` at byte {}", self.i)),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Value::Arr(items));
                }
                loop {
                    self.ws();
                    items.push(self.value(depth + 1)?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Arr(items));
                        }
                        _ => return Err(format!("expected `,` or `]` at byte {}", self.i)),
                    }
                }
            }
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.word("true", Value::Bool(true)),
            Some(b'f') => self.word("false", Value::Bool(false)),
            Some(b'n') => self.word("null", Value::Null),
            Some(_) => self.num(),
        }
    }

    fn num(&mut self) -> Result<Value, String> {
        let start = self.i;
        while self.i < self.s.len()
            && matches!(self.s[self.i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
        {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i])
            .ok()
            .and_then(|t| t.parse::<f64>().ok())
            .map(Value::Num)
            .ok_or_else(|| format!("not a number at byte {start}"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let h = self.s.get(self.i..self.i + 4).and_then(|b| std::str::from_utf8(b).ok());
        let n = h.and_then(|h| u32::from_str_radix(h, 16).ok()).ok_or("bad \\u escape")?;
        self.i += 4;
        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&b) = self.s.get(self.i) else {
                return Err("unterminated string".to_string());
            };
            self.i += 1;
            match b {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        return Err("unterminated string".to_string());
                    };
                    self.i += 1;
                    let c = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'u' => {
                            let mut code = self.hex4()?;
                            // A high surrogate pairs with the escape that follows it.
                            if (0xD800..0xDC00).contains(&code) && self.s[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let low = self.hex4()?;
                                code = 0x10000 + ((code - 0xD800) << 10) + low.wrapping_sub(0xDC00);
                            }
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        }
                        _ => return Err(format!("bad escape at byte {}", self.i - 1)),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                b => out.push(b),
            }
        }
        String::from_utf8(out).map_err(|_| "string is not UTF-8".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_it_writes() {
        let text = format!(
            "{{\"a\": [1, 2.5, -3e2], \"s\": {}, \"t\": true, \"n\": null, \"o\": {{}}}}",
            quote("q\"\\\n\u{1}é🙂")
        );
        let v = parse(&text).expect("valid");
        assert_eq!(v.get("a").and_then(Value::as_arr).map(<[Value]>::len), Some(3));
        assert_eq!(v.get("a").and_then(|a| a.as_arr()).and_then(|a| a[2].as_f64()), Some(-300.0));
        assert_eq!(v.get("s").and_then(Value::as_str), Some("q\"\\\n\u{1}é🙂"));
        assert_eq!(v.get("t").and_then(Value::as_bool), Some(true));
        assert_eq!(v.get("n"), Some(&Value::Null));
        assert_eq!(v.get("o"), Some(&Value::Obj(vec![])));
    }

    #[test]
    fn reads_unicode_escapes_and_surrogate_pairs() {
        let v = parse(r#""é🙂""#).expect("valid");
        assert_eq!(v.as_str(), Some("é🙂"));
    }

    #[test]
    fn rejects_broken_input() {
        for bad in ["", "{", "[1,", "{\"a\" 1}", "[1] x", "\"abc", "tru", "{\"a\":}"] {
            assert!(parse(bad).is_err(), "{bad:?} should not parse");
        }
        assert!(parse(&"[".repeat(100)).is_err());
    }

    #[test]
    fn numbers_are_short_and_finite() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(0.123456), "0.123");
        assert_eq!(number(f64::NAN), "0");
        assert_eq!(number(1234.5678), "1234.568");
    }
}
