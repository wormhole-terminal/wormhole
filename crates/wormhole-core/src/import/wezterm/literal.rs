//! The values of a WezTerm config that are written out: numbers, strings, booleans, tables of
//! those, and the two calls everybody writes (`wezterm.font`, `wezterm.font_with_fallback`).
//! Anything computed (a variable, a function call, arithmetic, a condition) is [`Lit::Dynamic`]:
//! its tokens are skipped, never evaluated.

use super::lexer::{Kind, Token};

/// A value read from the config.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lit {
    Str(String),
    Num(f64),
    Bool(bool),
    Nil,
    /// A table: `{ a = 1, 'b' }`; positional entries have no key.
    Table(Vec<(Option<String>, Lit)>),
    /// `wezterm.font('A')`, or the families of `wezterm.font_with_fallback{ 'A', 'B' }`.
    Font(Vec<String>),
    /// Anything that is not written out.
    Dynamic,
}

impl Lit {
    /// The value as it would be written, for the review list.
    pub(crate) fn show(&self) -> String {
        match self {
            Lit::Str(text) => format!("\"{text}\""),
            Lit::Num(n) => n.to_string(),
            Lit::Bool(on) => on.to_string(),
            Lit::Nil => "nil".to_string(),
            Lit::Font(names) => format!(
                "wezterm.font({})",
                names.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", ")
            ),
            Lit::Table(items) => {
                let parts: Vec<String> = items
                    .iter()
                    .map(|(key, value)| match key {
                        Some(key) => format!("{key} = {}", value.show()),
                        None => value.show(),
                    })
                    .collect();
                format!("{{ {} }}", parts.join(", "))
            }
            Lit::Dynamic => "…".to_string(),
        }
    }

    /// The entry of a table by key.
    pub(crate) fn field(&self, name: &str) -> Option<&Lit> {
        match self {
            Lit::Table(items) => {
                items.iter().rev().find(|(key, _)| key.as_deref() == Some(name)).map(|(_, v)| v)
            }
            _ => None,
        }
    }

    /// The positional entries of a table.
    pub(crate) fn list(&self) -> Vec<&Lit> {
        match self {
            Lit::Table(items) => items.iter().filter(|(key, _)| key.is_none()).map(|(_, v)| v).collect(),
            _ => Vec::new(),
        }
    }
}

/// A position in the tokens.
pub(crate) struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
    /// How many expressions are being read inside one another (a table in a call in a table…).
    depth: usize,
}

/// The deepest nesting of expressions read. A real config is five or six deep; past this the
/// expression is skipped as [`Lit::Dynamic`], so neither the parser's recursion nor the value's
/// drop and display can be made to overflow the stack.
pub(crate) const MAX_DEPTH: usize = 32;

/// Operators that continue an expression, so what came before is only a part of it.
const BINARY: [&str; 15] = ["+", "-", "*", "/", "%", "^", "..", "==", "~=", "<", ">", "<=", ">=", "//", "#"];

impl<'a> Cursor<'a> {
    pub(crate) fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0, depth: 0 }
    }

    pub(crate) fn peek(&self) -> Option<&'a Kind> {
        self.tokens.get(self.pos).map(|t| &t.kind)
    }

    pub(crate) fn peek_at(&self, ahead: usize) -> Option<&'a Kind> {
        self.tokens.get(self.pos + ahead).map(|t| &t.kind)
    }

    pub(crate) fn bump(&mut self) {
        self.pos += 1;
    }

    fn is_sym(&self, ahead: usize, sym: &str) -> bool {
        matches!(self.peek_at(ahead), Some(Kind::Sym(s)) if s == sym)
    }

    /// An expression: a value written out, or [`Lit::Dynamic`] when anything computes it.
    pub(crate) fn value(&mut self) -> Lit {
        if self.depth >= MAX_DEPTH {
            self.skip_expression();
            return Lit::Dynamic;
        }
        self.depth += 1;
        let lit = self.primary();
        self.depth -= 1;
        let continues = match self.peek() {
            Some(Kind::Sym(sym)) => BINARY.contains(&sym.as_str()),
            Some(Kind::Name(name)) => name == "and" || name == "or",
            _ => false,
        };
        if continues {
            self.skip_expression();
            return Lit::Dynamic;
        }
        lit
    }

    fn primary(&mut self) -> Lit {
        let Some(kind) = self.peek() else { return Lit::Dynamic };
        match kind {
            Kind::Str(text) => {
                self.bump();
                Lit::Str(text.clone())
            }
            Kind::Num(n) => {
                self.bump();
                Lit::Num(*n)
            }
            Kind::Sym(sym) if sym == "-" && matches!(self.peek_at(1), Some(Kind::Num(_))) => {
                self.bump();
                let Some(Kind::Num(n)) = self.peek() else { return Lit::Dynamic };
                self.bump();
                Lit::Num(-n)
            }
            Kind::Sym(sym) if sym == "{" => self.table(),
            Kind::Name(name) if name == "true" || name == "false" => {
                self.bump();
                Lit::Bool(name == "true")
            }
            Kind::Name(name) if name == "nil" => {
                self.bump();
                Lit::Nil
            }
            Kind::Name(name) if name == "wezterm" && self.is_sym(1, ".") => self.wezterm_call(),
            _ => {
                self.skip_expression();
                Lit::Dynamic
            }
        }
    }

    /// `wezterm.font(…)`, `wezterm.font_with_fallback(…)`, `wezterm.color.parse(…)`.
    fn wezterm_call(&mut self) -> Lit {
        let (function, color) = match (self.peek_at(2), self.is_sym(3, "."), self.peek_at(4)) {
            (Some(Kind::Name(f)), false, _) => (f.clone(), false),
            (Some(Kind::Name(c)), true, Some(Kind::Name(f))) if c == "color" => (f.clone(), true),
            _ => {
                self.skip_expression();
                return Lit::Dynamic;
            }
        };
        let after_name = if color { 5 } else { 3 };
        // A call is `f(args)`, `f{…}` or `f 'text'`; anything else (`wezterm.target_triple`) is a value
        // of WezTerm's.
        let called = self.is_sym(after_name, "(")
            || self.is_sym(after_name, "{")
            || matches!(self.peek_at(after_name), Some(Kind::Str(_)));
        let wanted =
            matches!((color, function.as_str()), (false, "font" | "font_with_fallback") | (true, "parse"));
        if !called || !wanted {
            self.skip_expression();
            return Lit::Dynamic;
        }
        self.pos += after_name;
        let args = self.arguments();
        let first = args.first();
        match (color, function.as_str(), first) {
            (true, _, Some(Lit::Str(text))) => Lit::Str(text.clone()),
            (false, "font", Some(Lit::Str(name))) => Lit::Font(vec![name.clone()]),
            (false, "font_with_fallback", Some(table @ Lit::Table(_))) => {
                let names: Vec<String> = table
                    .list()
                    .into_iter()
                    .filter_map(|entry| match entry {
                        Lit::Str(name) => Some(name.clone()),
                        Lit::Font(names) => names.first().cloned(),
                        _ => None,
                    })
                    .collect();
                if names.is_empty() { Lit::Dynamic } else { Lit::Font(names) }
            }
            _ => Lit::Dynamic,
        }
    }

    /// A call's arguments: `(a, b)`, a table, or a string.
    fn arguments(&mut self) -> Vec<Lit> {
        match self.peek() {
            Some(Kind::Sym(sym)) if sym == "(" => {
                self.bump();
                let mut args = Vec::new();
                while let Some(kind) = self.peek() {
                    match kind {
                        Kind::Sym(sym) if sym == ")" => {
                            self.bump();
                            break;
                        }
                        Kind::Sym(sym) if sym == "," => self.bump(),
                        _ => {
                            let before = self.pos;
                            args.push(self.value());
                            if self.pos == before {
                                self.bump();
                            }
                        }
                    }
                }
                args
            }
            Some(Kind::Sym(sym)) if sym == "{" => vec![self.table()],
            Some(Kind::Str(text)) => {
                self.bump();
                vec![Lit::Str(text.clone())]
            }
            _ => Vec::new(),
        }
    }

    /// A table constructor, from its `{`.
    pub(crate) fn table(&mut self) -> Lit {
        self.bump();
        let mut items = Vec::new();
        while let Some(kind) = self.peek() {
            let before = self.pos;
            match kind {
                Kind::Sym(sym) if sym == "}" => {
                    self.bump();
                    break;
                }
                Kind::Sym(sym) if sym == "," || sym == ";" => self.bump(),
                Kind::Name(name) if self.is_sym(1, "=") => {
                    let key = name.clone();
                    self.pos += 2;
                    items.push((Some(key), self.value()));
                }
                Kind::Sym(sym) if sym == "[" => {
                    // `["key"] = value`; a key that is computed is kept as no key at all.
                    let key = match (self.peek_at(1), self.is_sym(2, "]")) {
                        (Some(Kind::Str(key)), true) => Some(key.clone()),
                        _ => None,
                    };
                    while self.peek().is_some() && !self.is_sym(0, "]") {
                        self.bump();
                    }
                    self.bump();
                    if self.is_sym(0, "=") {
                        self.bump();
                    }
                    items.push((key, self.value()));
                }
                _ => items.push((None, self.value())),
            }
            if self.pos == before {
                self.bump();
            }
        }
        Lit::Table(items)
    }

    /// Skip an expression that is not read: to the `,`, `;` or closing bracket that ends it, or the
    /// end of its line when nothing continues it. Brackets and the `function … end` blocks inside
    /// are skipped whole.
    pub(crate) fn skip_expression(&mut self) {
        let mut depth = 0usize;
        let mut last_line = self.tokens.get(self.pos).map(|t| t.line);
        let mut after_operator = true;
        while let Some(token) = self.tokens.get(self.pos) {
            if depth == 0 {
                let ends =
                    matches!(&token.kind, Kind::Sym(s) if matches!(s.as_str(), "," | ";" | "}" | ")" | "]"));
                let continues = match &token.kind {
                    Kind::Sym(s) => {
                        BINARY.contains(&s.as_str()) || matches!(s.as_str(), "." | ":" | "(" | "{" | "[")
                    }
                    Kind::Name(n) => n == "and" || n == "or",
                    _ => false,
                };
                let new_line = last_line.is_some_and(|line| token.line > line);
                if ends || (new_line && !after_operator && !continues) {
                    break;
                }
            }
            last_line = Some(token.line);
            after_operator = false;
            match &token.kind {
                Kind::Sym(s) if matches!(s.as_str(), "(" | "[" | "{") => depth += 1,
                Kind::Sym(s) if matches!(s.as_str(), ")" | "]" | "}") => depth = depth.saturating_sub(1),
                Kind::Name(n) if matches!(n.as_str(), "function" | "if" | "do" | "repeat") => depth += 1,
                Kind::Name(n) if matches!(n.as_str(), "end" | "until") => depth = depth.saturating_sub(1),
                Kind::Sym(s)
                    if BINARY.contains(&s.as_str()) || matches!(s.as_str(), "," | "=" | "." | ":") =>
                {
                    after_operator = true;
                }
                Kind::Name(n) if n == "and" || n == "or" || n == "not" => after_operator = true,
                _ => {}
            }
            self.pos += 1;
        }
    }
}
