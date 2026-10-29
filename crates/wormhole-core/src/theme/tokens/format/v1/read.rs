//! Reading a version 1 file (or a newer one, leniently) from its JSON value into the
//! resolver's model.
//!
//! Strict for version 1: a key this reader does not know, a value of the wrong kind or a
//! metric out of range is an error that says where. Lenient for a newer version: the same
//! things are dropped and counted instead, so a file made by a newer Wormhole loads with what
//! this one understands and says how much it skipped. Caps, ranges and the rule that
//! terminal colors are opaque are enforced either way (the last by the resolver).

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::super::keys::{KEY_TABLE, new_key, new_node, old_node};
use super::super::refs::{Rename, rewrite_refs};
use super::super::slug;
use super::super::{BaseLayer, CURRENT_VERSION, ReadReport};
use crate::storage::jsonc::suggest::did_you_mean;
use crate::theme::tokens::catalog::UI_TOKENS;
use crate::theme::tokens::expr::parse_expr;
use crate::theme::tokens::file::{MetricsFile, SyntaxOverride, TerminalFile, ThemeFile, ThemeVariant};
use crate::theme::tokens::model::Appearance;
use crate::theme::tokens::variants::MAX_VARIANTS;

const TOP_KEYS: &[&str] = &[
    "$schema",
    "version",
    "id",
    "name",
    "base",
    "appearance",
    "terminal",
    "ui",
    "syntax",
    "metrics",
    "variants",
];
const BASE_KEYS: &[&str] = &["from", "copied", "appearance", "terminal", "ui", "syntax", "metrics"];
const VARIANT_KEYS: &[&str] = &["name", "base", "appearance", "terminal", "ui", "syntax", "metrics"];
const SYNTAX_STYLE_KEYS: &[&str] = &["color", "bold", "italic"];
const TERMINAL_PLAIN: &[&str] = &["foreground", "background", "cursor"];
/// The longest `copied` date or `ui_font.family` accepted.
const MAX_TEXT: usize = 256;

/// Read the file the value `root` holds, `source_version` being the version it says.
pub(in super::super) fn read(root: &Value, source_version: u32) -> Result<ThemeFile, String> {
    let newer = source_version > CURRENT_VERSION;
    let Some(map) = root.as_object() else { return Err("a theme file is a JSON object".into()) };
    let mut r = Reader { lenient: newer, skipped: 0 };
    let mut file = r.file(map)?;
    file.version = Some(CURRENT_VERSION);
    file.read = ReadReport { source_version, newer, skipped: r.skipped, notices: Vec::new() };
    Ok(file)
}

struct Reader {
    lenient: bool,
    skipped: usize,
}

type Fallible<T> = Result<Option<T>, String>;

impl Reader {
    /// Report `problem`: an error for a version this reader knows, a skipped entry for a newer
    /// one.
    fn reject<T>(&mut self, problem: impl FnOnce() -> String) -> Fallible<T> {
        if self.lenient {
            self.skipped += 1;
            Ok(None)
        } else {
            Err(problem())
        }
    }

    fn unknown_key<T>(&mut self, ctx: &str, key: &str, known: &[&str]) -> Fallible<T> {
        self.reject(|| {
            let hint = did_you_mean(key, known.iter().copied());
            format!("{ctx}: unknown key {key:?}{hint}")
        })
    }

    fn string(&mut self, v: &Value, ctx: &str) -> Fallible<String> {
        match v.as_str() {
            Some(s) => Ok(Some(s.to_string())),
            None => self.reject(|| format!("{ctx}: expected a string")),
        }
    }

    fn object<'a>(&mut self, v: &'a Value, ctx: &str) -> Fallible<&'a Map<String, Value>> {
        match v.as_object() {
            Some(m) => Ok(Some(m)),
            None => self.reject(|| format!("{ctx}: expected an object")),
        }
    }

    fn file(&mut self, map: &Map<String, Value>) -> Result<ThemeFile, String> {
        let mut file = ThemeFile::default();
        for (key, v) in map {
            match key.as_str() {
                "version" => {}
                "$schema" => file.schema = self.string(v, "$schema")?,
                "name" => file.name = self.string(v, "name")?,
                "id" => {
                    if let Some(id) = self.string(v, "id")? {
                        if slug::is_slug(&id) {
                            file.id = Some(id);
                        } else {
                            self.reject::<()>(|| format!("id: {id:?} is not a slug ({})", slug::GRAMMAR))?;
                        }
                    }
                }
                "base" => file.base = self.base(v, "base")?.map(Box::new),
                "variants" => file.variants = self.variants(v)?,
                "extends" => {
                    self.reject::<()>(|| {
                        "extends: a version 1 theme has a `base` (the values it started from) instead".into()
                    })?;
                }
                other => {
                    if !self.layer(other, v, &mut file, other)? {
                        self.unknown_key::<()>("theme", other, TOP_KEYS)?;
                    }
                }
            }
        }
        Ok(file)
    }

    fn base(&mut self, v: &Value, ctx: &str) -> Fallible<BaseLayer> {
        let Some(map) = self.object(v, ctx)? else { return Ok(None) };
        let mut base = BaseLayer::default();
        let mut from = None;
        for (key, v) in map {
            let at = format!("{ctx}.{key}");
            match key.as_str() {
                "from" => from = self.string(v, &at)?,
                "copied" => {
                    base.copied = self.string(v, &at)?.filter(|c| c.len() <= MAX_TEXT);
                }
                other => {
                    if !self.layer(other, v, &mut base.file, &at)? {
                        self.unknown_key::<()>(ctx, other, BASE_KEYS)?;
                    }
                }
            }
        }
        match from {
            Some(from) if slug::is_slug(&from) => base.from = from,
            Some(from) => {
                return self.reject(|| format!("{ctx}.from: {from:?} is not a slug ({})", slug::GRAMMAR));
            }
            None => return self.reject(|| format!("{ctx}: `from` says which theme it was copied from")),
        }
        Ok(Some(base))
    }

    fn variants(&mut self, v: &Value) -> Result<Vec<ThemeVariant>, String> {
        let Some(items) = v.as_array() else {
            self.reject::<()>(|| "variants: expected a list".into())?;
            return Ok(Vec::new());
        };
        if items.len() > MAX_VARIANTS {
            if !self.lenient {
                return Err(format!("variants: more than {MAX_VARIANTS}"));
            }
            self.skipped += items.len() - MAX_VARIANTS;
        }
        let mut out: Vec<ThemeVariant> = Vec::new();
        for (ix, item) in items.iter().take(MAX_VARIANTS).enumerate() {
            let ctx = format!("variants[{ix}]");
            let Some(map) = self.object(item, &ctx)? else { continue };
            let mut variant = ThemeVariant::default();
            let mut named = false;
            let mut layers = ThemeFile::default();
            for (key, v) in map {
                let at = format!("{ctx}.{key}");
                match key.as_str() {
                    "name" => {
                        if let Some(name) = self.string(v, &at)? {
                            if slug::is_part(&name) {
                                variant.name = name;
                                named = true;
                            } else {
                                self.reject::<()>(|| {
                                    format!("{at}: {name:?} must be lowercase letters, digits and `_`")
                                })?;
                            }
                        }
                    }
                    "base" => variant.base = self.base(v, &at)?.map(Box::new),
                    other => {
                        if !self.layer(other, v, &mut layers, &at)? {
                            self.unknown_key::<()>(&ctx, other, VARIANT_KEYS)?;
                        }
                    }
                }
            }
            variant.set_layers(&layers);
            if !named {
                self.reject::<()>(|| format!("{ctx}: a version needs a `name`"))?;
                continue;
            }
            if out.iter().any(|w| w.name == variant.name) {
                self.reject::<()>(|| format!("{ctx}: two versions are called {:?}", variant.name))?;
                continue;
            }
            out.push(variant);
        }
        Ok(out)
    }

    /// One of the five blocks every layer has (`appearance`, `terminal`, `ui`, `syntax`,
    /// `metrics`); false when `key` is not one of them.
    fn layer(&mut self, key: &str, v: &Value, into: &mut ThemeFile, ctx: &str) -> Result<bool, String> {
        match key {
            "appearance" => {
                if let Some(a) = self.appearance(v, ctx)? {
                    into.appearance = Some(a);
                }
            }
            "terminal" => {
                if let Some(t) = self.terminal(v, ctx)? {
                    into.terminal = t;
                }
            }
            "ui" => {
                if let Some(u) = self.ui(v, ctx)? {
                    into.ui = u;
                }
            }
            "syntax" => {
                if let Some(s) = self.syntax(v, ctx)? {
                    into.syntax = s;
                }
            }
            "metrics" => {
                if let Some(m) = self.metrics(v, ctx)? {
                    into.metrics = m;
                }
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn appearance(&mut self, v: &Value, ctx: &str) -> Fallible<Appearance> {
        match v.as_str() {
            Some("light") => Ok(Some(Appearance::Light)),
            Some("dark") => Ok(Some(Appearance::Dark)),
            _ => self.reject(|| format!("{ctx}: expected \"light\" or \"dark\"")),
        }
    }

    /// An expression as the resolver wants it: version 1 node names turned into the resolver's,
    /// and, for a newer file, one this reader cannot parse dropped (a function it lacks).
    fn expression(&mut self, v: &Value, ctx: &str) -> Fallible<String> {
        let Some(src) = self.string(v, ctx)? else { return Ok(None) };
        let renamed = rewrite_refs(&src, &|name| {
            if let Some(new) = old_node(name) {
                return Rename::To(new);
            }
            match new_node(name) {
                Some(now) => {
                    Rename::Refuse(format!("{ctx}: `{name}` is called `{now}` in a version 1 theme"))
                }
                None => Rename::Keep,
            }
        });
        match renamed {
            Ok(text) if self.lenient && parse_expr(&text).is_err() => {
                self.reject(|| format!("{ctx}: not an expression"))
            }
            Ok(text) => Ok(Some(text)),
            Err(why) => self.reject(|| why),
        }
    }

    fn terminal(&mut self, v: &Value, ctx: &str) -> Fallible<TerminalFile> {
        let Some(map) = self.object(v, ctx)? else { return Ok(None) };
        let mut t = TerminalFile::default();
        for (key, v) in map {
            let at = format!("{ctx}.{key}");
            let internal = if TERMINAL_PLAIN.contains(&key.as_str()) {
                Some(key.clone())
            } else if let Some(old) =
                KEY_TABLE.iter().find(|r| r.block == "terminal" && r.new == key).map(|r| r.old)
            {
                Some(old.to_string())
            } else {
                key.strip_prefix("ansi.")
                    .and_then(|i| {
                        i.parse::<usize>().ok().filter(|i| *i < 16 && i.to_string() == key["ansi.".len()..])
                    })
                    .map(|i| format!("ansi.{i}"))
            };
            let Some(internal) = internal else {
                // The old spelling gets its own hint: it is the likely mistake.
                let hint = KEY_TABLE.iter().find(|r| r.block == "terminal" && r.old == key);
                if let (Some(r), false) = (hint, self.lenient) {
                    return Err(format!("{at}: this key is called `{}` in a version 1 theme", r.new));
                }
                let names = super::super::keys::terminal_keys();
                self.unknown_key::<()>(ctx, key, &names.iter().map(String::as_str).collect::<Vec<_>>())?;
                continue;
            };
            if let Some(value) = self.expression(v, &at)? {
                t.insert(&internal, value);
            }
        }
        Ok(Some(t))
    }

    fn ui(&mut self, v: &Value, ctx: &str) -> Fallible<BTreeMap<String, String>> {
        let Some(map) = self.object(v, ctx)? else { return Ok(None) };
        let mut ui = BTreeMap::new();
        for (key, v) in map {
            let at = format!("{ctx}.{key}");
            // A token this reader has never heard of may be one a newer Wormhole added.
            if self.lenient && !UI_TOKENS.iter().any(|t| t.name == key) {
                self.skipped += 1;
                continue;
            }
            if let Some(value) = self.expression(v, &at)? {
                ui.insert(key.clone(), value);
            }
        }
        Ok(Some(ui))
    }

    fn syntax(&mut self, v: &Value, ctx: &str) -> Fallible<BTreeMap<String, SyntaxOverride>> {
        let Some(map) = self.object(v, ctx)? else { return Ok(None) };
        let mut out = BTreeMap::new();
        for (key, v) in map {
            let at = format!("{ctx}.{key}");
            let style = match v {
                Value::String(_) => self.expression(v, &at)?.map(SyntaxOverride::Color),
                Value::Object(parts) => self.syntax_style(parts, &at)?,
                _ => self.reject(|| {
                    format!(
                        "{at}: expected a color, or a style like {{\"color\": \"#bb9af7\", \"italic\": true}}"
                    )
                })?,
            };
            if let Some(style) = style {
                out.insert(key.clone(), style);
            }
        }
        Ok(Some(out))
    }

    fn syntax_style(&mut self, parts: &Map<String, Value>, ctx: &str) -> Fallible<SyntaxOverride> {
        let (mut color, mut bold, mut italic) = (None, None, None);
        for (key, v) in parts {
            let at = format!("{ctx}.{key}");
            match key.as_str() {
                "color" => color = self.expression(v, &at)?,
                "bold" | "italic" => {
                    let slot = if key == "bold" { &mut bold } else { &mut italic };
                    match v.as_bool() {
                        Some(b) => *slot = Some(b),
                        None => {
                            self.reject::<()>(|| format!("{at}: expected true or false"))?;
                        }
                    }
                }
                other => {
                    self.unknown_key::<()>(ctx, other, SYNTAX_STYLE_KEYS)?;
                }
            }
        }
        Ok(Some(SyntaxOverride::Style { color, bold, italic }))
    }

    fn metrics(&mut self, v: &Value, ctx: &str) -> Fallible<MetricsFile> {
        let Some(map) = self.object(v, ctx)? else { return Ok(None) };
        let mut m = MetricsFile::default();
        for (key, v) in map {
            let at = format!("{ctx}.{key}");
            let old = if super::super::keys::METRIC_ORDER.contains(&key.as_str()) {
                super::super::keys::metric_internal(key)
            } else {
                None
            };
            let Some(old) = old else {
                if let (Some(now), false) = (new_key("metrics", key), self.lenient) {
                    return Err(format!("{at}: this key is called `{now}` in a version 1 theme"));
                }
                self.unknown_key::<()>(ctx, key, super::super::keys::METRIC_ORDER)?;
                continue;
            };
            if old == "ui_font_family" {
                if let Some(f) = self.string(v, &at)? {
                    if f.len() <= MAX_TEXT {
                        m.ui_font_family = Some(f);
                    } else {
                        self.reject::<()>(|| format!("{at}: longer than {MAX_TEXT} characters"))?;
                    }
                }
                continue;
            }
            match v.as_f64() {
                Some(n) if (0.0..=200.0).contains(&n) => {
                    m.set(old, Some(n as f32));
                }
                Some(n) => {
                    self.reject::<()>(|| format!("{at}: {n} is out of range (0..200)"))?;
                }
                None => {
                    self.reject::<()>(|| format!("{at}: expected a number"))?;
                }
            }
        }
        Ok(Some(m))
    }
}
