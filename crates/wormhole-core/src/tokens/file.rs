//! The on-disk theme format: [`ThemeFile`] and its parts, as written by hand,
//! by the Theme Studio, or in a project's `.wormhole/theme.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::{Appearance, Metrics};
use crate::jsonc;

/// Terminal overrides. Keys: `foreground`, `background`, `cursor`,
/// `cursor_text`, `selection_background`, `selection_foreground`, and `ansi`
/// (a map from `"0"`..`"15"` or `"black"`..`"bright_white"` to a color).
/// Values are expressions that may only reference `terminal.*`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_foreground: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ansi: BTreeMap<String, String>,
}

const ANSI_NAMES: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

impl TerminalFile {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Insert by `terminal.*` key (`background`, `ansi.4`, ...).
    pub(super) fn insert(&mut self, key: &str, value: String) {
        let slot = match key {
            "foreground" => &mut self.foreground,
            "background" => &mut self.background,
            "cursor" => &mut self.cursor,
            "cursor_text" => &mut self.cursor_text,
            "selection_background" => &mut self.selection_background,
            "selection_foreground" => &mut self.selection_foreground,
            _ => {
                if let Some(i) = key.strip_prefix("ansi.") {
                    self.ansi.insert(i.to_string(), value);
                }
                return;
            }
        };
        *slot = Some(value);
    }

    /// (`terminal.*` node name, expression) pairs, validating ANSI keys.
    pub(super) fn entries(&self) -> Result<Vec<(String, String)>, String> {
        let mut out = Vec::new();
        let fields = [
            ("foreground", &self.foreground),
            ("background", &self.background),
            ("cursor", &self.cursor),
            ("cursor_text", &self.cursor_text),
            ("selection_background", &self.selection_background),
            ("selection_foreground", &self.selection_foreground),
        ];
        for (k, v) in fields {
            if let Some(v) = v {
                out.push((format!("terminal.{k}"), v.clone()));
            }
        }
        for (k, v) in &self.ansi {
            let Some(idx) = ansi_index(k) else {
                return Err(format!(
                    "terminal.ansi: bad key {k:?} (use 0..15 or black, red, ..., bright_white)"
                ));
            };
            out.push((format!("terminal.ansi.{idx}"), v.clone()));
        }
        Ok(out)
    }
}

/// The palette index an `ansi` key names: `"0"`..`"15"` or `"black"`..`"bright_white"`.
fn ansi_index(key: &str) -> Option<usize> {
    key.parse::<usize>().ok().filter(|i| *i < 16).or_else(|| {
        let (bright, base) = key.strip_prefix("bright_").map_or((0, key), |b| (8, b));
        ANSI_NAMES.iter().position(|n| *n == base).map(|i| i + bright)
    })
}

impl TerminalFile {
    /// `self` over `under`, field by field; an ANSI color replaces the one
    /// for the same index under any of its names.
    fn layered_over(&self, under: &TerminalFile) -> TerminalFile {
        let pick = |top: &Option<String>, bottom: &Option<String>| top.clone().or_else(|| bottom.clone());
        let mut ansi = under.ansi.clone();
        for (key, value) in &self.ansi {
            if let Some(index) = ansi_index(key) {
                ansi.retain(|k, _| ansi_index(k) != Some(index));
            }
            ansi.insert(key.clone(), value.clone());
        }
        TerminalFile {
            foreground: pick(&self.foreground, &under.foreground),
            background: pick(&self.background, &under.background),
            cursor: pick(&self.cursor, &under.cursor),
            cursor_text: pick(&self.cursor_text, &under.cursor_text),
            selection_background: pick(&self.selection_background, &under.selection_background),
            selection_foreground: pick(&self.selection_foreground, &under.selection_foreground),
            ansi,
        }
    }
}

/// A syntax override: either just a color expression, or a partial style.
/// Read strictly (see the `Deserialize` impl): a misspelled style key is an
/// error, not a style that silently does nothing.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SyntaxOverride {
    Color(String),
    Style {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bold: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        italic: Option<bool>,
    },
}

impl<'de> Deserialize<'de> for SyntaxOverride {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        const FIELDS: &[&str] = &["color", "bold", "italic"];
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SyntaxOverride;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str(
                    "a color, or a style like {\"color\": \"#bb9af7\", \"bold\": true, \"italic\": false}",
                )
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<SyntaxOverride, E> {
                Ok(SyntaxOverride::Color(v.to_string()))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<SyntaxOverride, A::Error> {
                use serde::de::Error;
                let (mut color, mut bold, mut italic) = (None, None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "color" if color.is_none() => color = Some(map.next_value()?),
                        "bold" if bold.is_none() => bold = Some(map.next_value()?),
                        "italic" if italic.is_none() => italic = Some(map.next_value()?),
                        other => {
                            return Err(match FIELDS.iter().find(|f| **f == other) {
                                Some(field) => A::Error::duplicate_field(field),
                                None => A::Error::unknown_field(other, FIELDS),
                            });
                        }
                    }
                }
                Ok(SyntaxOverride::Style { color, bold, italic })
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl SyntaxOverride {
    fn parts(&self) -> (Option<&String>, Option<bool>, Option<bool>) {
        match self {
            SyntaxOverride::Color(c) => (Some(c), None, None),
            SyntaxOverride::Style { color, bold, italic } => (color.as_ref(), *bold, *italic),
        }
    }

    /// `self` over `under`, part by part, as resolution merges them.
    fn layered_over(&self, under: &SyntaxOverride) -> SyntaxOverride {
        let ((color, bold, italic), (under_color, under_bold, under_italic)) = (self.parts(), under.parts());
        let color = color.or(under_color).cloned();
        match (color, bold.or(under_bold), italic.or(under_italic)) {
            (Some(color), None, None) => SyntaxOverride::Color(color),
            (color, bold, italic) => SyntaxOverride::Style { color, bold, italic },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsFile {
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub radius_sm: Option<f32>,
    /// Optional: without it, chips follow `radius_sm`.
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub radius_chip: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub radius_lg: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub spacing: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_font_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub ui_font_size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub border_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none", serialize_with = "whole_number")]
    pub focus_ring_width: Option<f32>,
}

/// A metric as a theme file writes it: a whole number of pixels without a `.0` (`6`, as the
/// export shows), a fraction as it is.
fn whole_number<S: serde::Serializer>(value: &Option<f32>, s: S) -> Result<S::Ok, S::Error> {
    match value {
        // Far below where an f32 stops holding every integer, so the cast is exact.
        Some(v) if v.fract() == 0.0 && v.abs() < 1e7 => s.serialize_i64(*v as i64),
        Some(v) => s.serialize_f32(*v),
        None => s.serialize_none(),
    }
}

impl MetricsFile {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The value this file sets for a numeric metric, by key or `metrics.<key>`.
    pub fn get(&self, name: &str) -> Option<f32> {
        match name.strip_prefix("metrics.").unwrap_or(name) {
            "radius_sm" => self.radius_sm,
            "radius_chip" => self.radius_chip,
            "radius" => self.radius,
            "radius_lg" => self.radius_lg,
            "spacing" => self.spacing,
            "ui_font_size" => self.ui_font_size,
            "border_width" => self.border_width,
            "focus_ring_width" => self.focus_ring_width,
            _ => None,
        }
    }

    /// Set (or with `None`, drop) a numeric metric; false for an unknown name.
    pub fn set(&mut self, name: &str, value: Option<f32>) -> bool {
        match self.slot(name) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => false,
        }
    }

    fn slot(&mut self, name: &str) -> Option<&mut Option<f32>> {
        Some(match name.strip_prefix("metrics.").unwrap_or(name) {
            "radius_sm" => &mut self.radius_sm,
            "radius_chip" => &mut self.radius_chip,
            "radius" => &mut self.radius,
            "radius_lg" => &mut self.radius_lg,
            "spacing" => &mut self.spacing,
            "ui_font_size" => &mut self.ui_font_size,
            "border_width" => &mut self.border_width,
            "focus_ring_width" => &mut self.focus_ring_width,
            _ => return None,
        })
    }

    /// Later layers win field by field.
    pub(super) fn merge(&mut self, top: &MetricsFile) {
        self.radius_sm = top.radius_sm.or(self.radius_sm);
        self.radius_chip = top.radius_chip.or(self.radius_chip);
        self.radius = top.radius.or(self.radius);
        self.radius_lg = top.radius_lg.or(self.radius_lg);
        self.spacing = top.spacing.or(self.spacing);
        if top.ui_font_family.is_some() {
            self.ui_font_family.clone_from(&top.ui_font_family);
        }
        self.ui_font_size = top.ui_font_size.or(self.ui_font_size);
        self.border_width = top.border_width.or(self.border_width);
        self.focus_ring_width = top.focus_ring_width.or(self.focus_ring_width);
    }

    pub(super) fn apply(&self, m: &mut Metrics) -> Result<(), String> {
        let nums = [
            ("radius_sm", self.radius_sm, &mut m.radius_sm),
            ("radius_chip", self.radius_chip, &mut m.radius_chip),
            ("radius", self.radius, &mut m.radius),
            ("radius_lg", self.radius_lg, &mut m.radius_lg),
            ("spacing", self.spacing, &mut m.spacing),
            ("ui_font_size", self.ui_font_size, &mut m.ui_font_size),
            ("border_width", self.border_width, &mut m.border_width),
            ("focus_ring_width", self.focus_ring_width, &mut m.focus_ring_width),
        ];
        for (name, v, dst) in nums {
            if let Some(v) = v {
                if !(0.0..=200.0).contains(&v) {
                    return Err(format!("metrics.{name}: {v} is out of range (0..200)"));
                }
                *dst = v;
            }
        }
        // Derived, like a color token without an override: see `METRIC_RULES`.
        if self.radius_chip.is_none() {
            m.radius_chip = m.radius_sm;
        }
        if let Some(f) = &self.ui_font_family {
            m.ui_font_family = Some(f.clone());
        }
        Ok(())
    }
}

/// On-disk theme (JSON with comments). See the module docs for the value
/// grammar.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFile {
    /// The JSON Schema editors validate the file with; kept, not used.
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A built-in theme name or another theme file's name. Without it, the
    /// default built-in (Tokyo Night) supplies whatever is not overridden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    /// Chooses the light or dark rule set; auto-detected from the final
    /// terminal background when no file in the chain sets it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(default, skip_serializing_if = "TerminalFile::is_empty")]
    pub terminal: TerminalFile,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ui: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub syntax: BTreeMap<String, SyntaxOverride>,
    #[serde(default, skip_serializing_if = "MetricsFile::is_empty")]
    pub metrics: MetricsFile,
    /// The versions of the theme this file ships (light, dark, a minimal one, …), each a
    /// theme of its own made of its layers over the ones above; see [`super::variants`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<ThemeVariant>,
}

/// One entry of a theme file's `variants`: a version of the theme, named "<the file's name>
/// <its name>", with what it changes. Its `extends` names another variant of the file (by
/// its own name) or, failing that, any theme.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeVariant {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance: Option<Appearance>,
    #[serde(default, skip_serializing_if = "TerminalFile::is_empty")]
    pub terminal: TerminalFile,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ui: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub syntax: BTreeMap<String, SyntaxOverride>,
    #[serde(default, skip_serializing_if = "MetricsFile::is_empty")]
    pub metrics: MetricsFile,
}

impl ThemeVariant {
    /// The variant's own layers as a theme file of their own.
    pub fn to_file(&self) -> ThemeFile {
        ThemeFile {
            name: Some(self.name.clone()),
            extends: self.extends.clone(),
            appearance: self.appearance,
            terminal: self.terminal.clone(),
            ui: self.ui.clone(),
            syntax: self.syntax.clone(),
            metrics: self.metrics.clone(),
            ..Default::default()
        }
    }

    /// The layers of `file` (a single theme) as this variant's, keeping its name.
    pub fn set_layers(&mut self, file: &ThemeFile) {
        self.extends.clone_from(&file.extends);
        self.appearance = file.appearance;
        self.terminal = file.terminal.clone();
        self.ui = file.ui.clone();
        self.syntax = file.syntax.clone();
        self.metrics = file.metrics.clone();
    }
}

impl ThemeFile {
    /// One file that resolves like `self` extending `parent`: `parent`'s
    /// layers underneath, `self`'s on top, and `parent`'s own `extends`.
    /// Used to fold a draft into the theme it was made from.
    pub fn layered_over(&self, parent: &ThemeFile) -> ThemeFile {
        let mut ui = parent.ui.clone();
        ui.extend(self.ui.iter().map(|(k, v)| (k.clone(), v.clone())));
        let mut syntax = parent.syntax.clone();
        for (key, top) in &self.syntax {
            let merged = match syntax.get(key) {
                Some(under) => top.layered_over(under),
                None => top.clone(),
            };
            syntax.insert(key.clone(), merged);
        }
        let mut metrics = parent.metrics.clone();
        metrics.merge(&self.metrics);
        ThemeFile {
            schema: self.schema.clone().or_else(|| parent.schema.clone()),
            name: self.name.clone().or_else(|| parent.name.clone()),
            extends: parent.extends.clone(),
            appearance: self.appearance.or(parent.appearance),
            terminal: self.terminal.layered_over(&parent.terminal),
            ui,
            syntax,
            metrics,
            // The result is one theme: the versions a family file ships are themes of their own.
            variants: Vec::new(),
        }
    }

    pub fn to_json_pretty(&self) -> String {
        // Serializing plain structs and string-keyed maps cannot fail (NaN
        // metrics become `null`), so the fallback is never taken.
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into()) + "\n"
    }

    pub(super) fn label(&self) -> String {
        format!("theme {:?}", self.name.as_deref().unwrap_or("(unnamed)"))
    }
}

/// Largest theme file accepted. Real themes are a few KiB; the cap bounds the
/// work an untrusted file (a repository's `.wormhole/theme.json`) can cause.
pub const MAX_THEME_BYTES: usize = 256 * 1024;

pub fn parse_theme_file(text: &str) -> Result<ThemeFile, String> {
    if text.len() > MAX_THEME_BYTES {
        return Err(format!("theme file is larger than {} KiB", MAX_THEME_BYTES / 1024));
    }
    jsonc::parse(text)
}

#[cfg(test)]
mod tests {
    use super::{MetricsFile, SyntaxOverride, parse_theme_file};
    use crate::tokens::test_support::{no_lookup, resolve_str};
    use crate::tokens::{Appearance, METRIC_TOKENS, Metrics, resolve};

    const EXAMPLE: &str = r##"{
      "name": "My Night",
      "extends": "Tokyo Night",          // built-in name or another theme file name
      "appearance": "dark",              // optional, otherwise from background luminance
      "terminal": { "background": "#1a1b26", "ansi": { "4": "#7aa2f7", "bright_red": "terminal.ansi.1" } },
      "ui": { "primary": "#ff9e64", "sidebar.row.selected": "primary / 15%", },
      "syntax": { "keyword": { "color": "#bb9af7", "italic": true }, "string": "#9ece6a" },
      "metrics": { "radius": 6, "ui_font_family": "Inter" },
    }"##;

    #[test]
    fn example_file() {
        let f = parse_theme_file(EXAMPLE).unwrap();
        assert_eq!(f.extends.as_deref(), Some("Tokyo Night"));
        assert_eq!(f.appearance, Some(Appearance::Dark));
        let t = resolve(&f, &no_lookup).unwrap();
        assert_eq!(t.name, "My Night");
        assert_eq!(t.get("primary").unwrap().to_hex(), "#ff9e64");
        assert_eq!(t.get("sidebar.row.selected").unwrap().to_hex(), "#ff9e6426");
        // Dependents of an overridden token follow it.
        assert_eq!(t.get("border.focus").unwrap().to_hex(), "#ff9e64");
        assert_eq!(t.terminal.ansi[9], t.terminal.ansi[1]);
        let kw = t.syntax["keyword"];
        assert_eq!((kw.color.to_hex().as_str(), kw.bold, kw.italic), ("#bb9af7", false, true));
        assert_eq!(t.syntax["keyword.control"].color.to_hex(), "#bb9af7");
        assert_eq!(t.syntax["string"].color.to_hex(), "#9ece6a");
        assert_eq!(t.metrics.radius, 6.0);
        assert_eq!(t.metrics.ui_font_family.as_deref(), Some("Inter"));
        assert_eq!(t.metrics.radius_lg, Metrics::default().radius_lg);
    }

    #[test]
    fn schema_is_accepted_and_kept() {
        let f = parse_theme_file(r#"{"$schema": "https://example.com/theme.json", "name": "S"}"#).unwrap();
        assert_eq!(f.schema.as_deref(), Some("https://example.com/theme.json"));
        assert!(f.to_json_pretty().starts_with("{\n  \"$schema\": \"https://example.com/theme.json\","));
        assert!(resolve(&f, &no_lookup).is_ok());
    }

    #[test]
    fn syntax_styles_are_read_strictly() {
        let e = parse_theme_file(r##"{"syntax": {"keyword": {"colour": "#f00", "itallic": true}}}"##)
            .unwrap_err();
        assert!(e.contains("unknown field `colour`, expected one of `color`, `bold`, `italic`"), "{e}");
        assert!(e.starts_with("line 1, column"), "{e}");
        let e = parse_theme_file(r#"{"syntax": {"keyword": {"bold": "yes"}}}"#).unwrap_err();
        assert!(e.contains("expected a boolean"), "{e}");
        let e = parse_theme_file(r#"{"syntax": {"keyword": 7}}"#).unwrap_err();
        assert!(e.contains("expected a color, or a style"), "{e}");
        let e = parse_theme_file(r#"{"syntax": {"keyword": {"bold": true, "bold": false}}}"#).unwrap_err();
        assert!(e.contains("duplicate field"), "{e}");
        let f = parse_theme_file(r##"{"syntax": {"a": "#f00", "b": {}, "c": {"italic": true}}}"##).unwrap();
        assert_eq!(f.syntax["a"], SyntaxOverride::Color("#f00".into()));
        assert_eq!(f.syntax["b"], SyntaxOverride::Style { color: None, bold: None, italic: None });
        assert_eq!(f.syntax["c"], SyntaxOverride::Style { color: None, bold: None, italic: Some(true) });
    }

    #[test]
    fn file_errors() {
        let e = parse_theme_file(r#"{"name": "x", "colour": {}}"#).unwrap_err();
        assert!(e.contains("unknown field `colour`"), "{e}");
        let e = parse_theme_file("{\n  \"appearance\": \"dim\"\n}").unwrap_err();
        assert!(e.starts_with("line 2"), "{e}");
        let e = resolve_str(r#"{"metrics": {"radius": -1}}"#).unwrap_err();
        assert!(e.contains("metrics.radius"), "{e}");
        let e = resolve_str(r#"{"ui": {"primary": "mix(#000, #fff)"}}"#).unwrap_err();
        assert!(e.contains("primary") && e.contains("`,` in mix()"), "{e}");
    }

    /// A whole number of pixels reads `6`, not `6.0`; a fraction
    /// stays one, and the file reads back the same.
    #[test]
    fn metrics_are_written_as_whole_numbers_when_they_are() {
        let mut f = parse_theme_file(r#"{"name": "M"}"#).unwrap();
        f.metrics.set("radius_chip", Some(6.0));
        f.metrics.set("border_width", Some(1.5));
        let json = f.to_json_pretty();
        assert!(json.contains("\"radius_chip\": 6\n") || json.contains("\"radius_chip\": 6,"), "{json}");
        assert!(json.contains("\"border_width\": 1.5"), "{json}");
        assert!(!json.contains("6.0"), "{json}");
        assert_eq!(parse_theme_file(&json).unwrap(), f);
    }

    #[test]
    fn metrics_by_name() {
        let mut file = MetricsFile::default();
        assert!(file.set("metrics.radius_chip", Some(8.0)));
        assert!(!file.set("metrics.nope", Some(1.0)));
        assert_eq!(file.get("radius_chip"), Some(8.0));
        assert_eq!(file.get("metrics.radius"), None);
        let mut m = Metrics::default();
        file.apply(&mut m).unwrap();
        assert_eq!(m.get("metrics.radius_chip"), Some(8.0));
        for info in METRIC_TOKENS {
            assert!(m.get(info.name).is_some(), "{}", info.name);
            assert!(MetricsFile::default().set(info.name, Some(1.0)), "{}", info.name);
        }
        file.set("radius_chip", None);
        assert!(file.is_empty());
        // `get` reads what `set` wrote, for every key and in both spellings.
        let keys = [
            "radius_sm",
            "radius_chip",
            "radius",
            "radius_lg",
            "spacing",
            "ui_font_size",
            "border_width",
            "focus_ring_width",
        ];
        for (i, key) in keys.into_iter().enumerate() {
            let value = i as f32 + 0.5;
            assert!(file.set(key, Some(value)));
            assert_eq!((file.get(key), file.get(&format!("metrics.{key}"))), (Some(value), Some(value)));
        }
        assert_eq!(file.get("ui_font_family"), None);
    }
}
