//! Terminal color themes: parsers for Ghostty, iTerm2 (.itermcolors) and
//! Alacritty (.toml) formats, plus a set of bundled themes.
//!
//! All colors are packed as `0xRRGGBB`.
//!
//! The domain is everything that decides how Wormhole looks, as data: these terminal themes, the
//! design tokens (`tokens`), the theme files found on disk and in the gallery (`registry`,
//! `catalog`), file icon themes (`icon_theme`), a folder's colour (`folder_color`) and fonts
//! (`fonts`). Tier 1: it uses only `storage`, to read theme and font files. It never reads
//! settings; the chosen theme comes to it as a value, so settings, views and the Studio can all
//! build on it.

pub mod catalog;
pub mod folder_color;
pub mod fonts;
pub mod icon_theme;
pub mod registry;
pub mod tokens;

use std::path::Path;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub foreground: u32,
    pub background: u32,
    pub cursor: u32,
    pub cursor_text: Option<u32>,
    pub selection_background: u32,
    pub selection_foreground: Option<u32>,
    pub palette: [u32; 16],
}

/// Standard xterm 16-color palette, used for missing entries.
pub const XTERM_PALETTE: [u32; 16] = [
    0x000000, 0xcd0000, 0x00cd00, 0xcdcd00, 0x0000ee, 0xcd00cd, 0x00cdcd, 0xe5e5e5, //
    0x7f7f7f, 0xff0000, 0x00ff00, 0xffff00, 0x5c5cff, 0xff00ff, 0x00ffff, 0xffffff,
];

const DEFAULT_FG: u32 = 0xe5e5e5;
const DEFAULT_BG: u32 = 0x000000;

/// Partially-specified theme collected by a parser before defaults are applied.
#[derive(Default)]
struct Builder {
    foreground: Option<u32>,
    background: Option<u32>,
    cursor: Option<u32>,
    cursor_text: Option<u32>,
    selection_background: Option<u32>,
    selection_foreground: Option<u32>,
    palette: [Option<u32>; 16],
}

impl Builder {
    fn finish(self, name: &str) -> Theme {
        let foreground = self.foreground.unwrap_or(DEFAULT_FG);
        let background = self.background.unwrap_or(DEFAULT_BG);
        let mut palette = XTERM_PALETTE;
        for (dst, src) in palette.iter_mut().zip(self.palette) {
            if let Some(c) = src {
                *dst = c;
            }
        }
        Theme {
            name: name.to_string(),
            foreground,
            background,
            cursor: self.cursor.unwrap_or(foreground),
            cursor_text: self.cursor_text,
            selection_background: self
                .selection_background
                .unwrap_or_else(|| mix(foreground, background, 0.3)),
            selection_foreground: self.selection_foreground,
            palette,
        }
    }
}

/// Linear blend: `t` = 0 gives `b`, `t` = 1 gives `a`.
fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |c: u32, s: u32| ((c >> s) & 0xff) as f32;
    let blend = |s: u32| ((ch(a, s) * t + ch(b, s) * (1.0 - t)).round() as u32).min(255) << s;
    blend(16) | blend(8) | blend(0)
}

/// Parse `#rrggbb`, `rrggbb`, `0xrrggbb`, or 3-digit `#rgb`.
fn parse_hex(s: &str) -> Result<u32, String> {
    let t = s.trim().trim_matches(|c| c == '"' || c == '\'');
    let h =
        t.strip_prefix('#').or_else(|| t.strip_prefix("0x")).or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    let bad = || format!("invalid color {s:?}");
    if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    match h.len() {
        6 => u32::from_str_radix(h, 16).map_err(|_| bad()),
        3 => {
            let v = u32::from_str_radix(h, 16).map_err(|_| bad())?;
            let (r, g, b) = ((v >> 8) & 0xf, (v >> 4) & 0xf, v & 0xf);
            Ok((r * 0x11) << 16 | (g * 0x11) << 8 | (b * 0x11))
        }
        _ => Err(bad()),
    }
}

// ---------------------------------------------------------------- Ghostty

pub fn parse_ghostty(name: &str, text: &str) -> Result<Theme, String> {
    let mut b = Builder::default();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |e: String| format!("{name}:{}: {e}", lineno + 1);
        let Some((key, value)) = line.split_once('=') else {
            return Err(err(format!("expected `key = value`, got {line:?}")));
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "palette" => {
                let (idx, color) =
                    value.split_once('=').ok_or_else(|| err(format!("bad palette entry {value:?}")))?;
                let idx: usize = idx.trim().parse().map_err(|_| err(format!("bad palette index {idx:?}")))?;
                let color = parse_hex(color).map_err(err)?;
                // Ghostty supports 256 entries; we only keep the first 16.
                if idx < 16 {
                    b.palette[idx] = Some(color);
                } else if idx > 255 {
                    return Err(err(format!("palette index {idx} out of range")));
                }
            }
            "background" => b.background = Some(parse_hex(value).map_err(err)?),
            "foreground" => b.foreground = Some(parse_hex(value).map_err(err)?),
            "cursor-color" => b.cursor = Some(parse_hex(value).map_err(err)?),
            "cursor-text" => b.cursor_text = Some(parse_hex(value).map_err(err)?),
            "selection-background" => b.selection_background = Some(parse_hex(value).map_err(err)?),
            "selection-foreground" => b.selection_foreground = Some(parse_hex(value).map_err(err)?),
            _ => {} // unrelated ghostty config keys are ignored
        }
    }
    Ok(b.finish(name))
}

// ---------------------------------------------------------------- iTerm2

pub fn parse_iterm(name: &str, xml: &str) -> Result<Theme, String> {
    let value =
        plist::Value::from_reader_xml(xml.as_bytes()).map_err(|e| format!("{name}: invalid plist: {e}"))?;
    let dict = value.as_dictionary().ok_or_else(|| format!("{name}: plist root is not a dictionary"))?;

    let color = |key: &str| -> Result<Option<u32>, String> {
        let Some(entry) = dict.get(key) else { return Ok(None) };
        let d = entry.as_dictionary().ok_or_else(|| format!("{name}: {key:?} is not a dictionary"))?;
        let comp = |c: &str| -> Result<u32, String> {
            let v = match d.get(c) {
                Some(plist::Value::Real(r)) => *r,
                Some(plist::Value::Integer(i)) => i.as_signed().unwrap_or(0) as f64,
                Some(plist::Value::String(s)) => {
                    s.trim().parse().map_err(|_| format!("{name}: {key:?} {c} is not a number"))?
                }
                Some(_) => return Err(format!("{name}: {key:?} {c} is not a number")),
                None => return Err(format!("{name}: {key:?} missing {c}")),
            };
            Ok((v.clamp(0.0, 1.0) * 255.0).round() as u32)
        };
        // "Color Space" (sRGB / Calibrated / P3) is accepted but not converted.
        let (r, g, b) = (comp("Red Component")?, comp("Green Component")?, comp("Blue Component")?);
        Ok(Some(r << 16 | g << 8 | b))
    };

    let mut b = Builder {
        foreground: color("Foreground Color")?,
        background: color("Background Color")?,
        cursor: color("Cursor Color")?,
        cursor_text: color("Cursor Text Color")?,
        selection_background: color("Selection Color")?,
        selection_foreground: color("Selected Text Color")?,
        ..Default::default()
    };
    for i in 0..16 {
        b.palette[i] = color(&format!("Ansi {i} Color"))?;
    }
    Ok(b.finish(name))
}

// ---------------------------------------------------------------- Alacritty

pub fn parse_alacritty(name: &str, toml_text: &str) -> Result<Theme, String> {
    let root: toml::Table = toml_text.parse().map_err(|e| format!("{name}: invalid TOML: {e}"))?;
    let colors = match root.get("colors") {
        Some(toml::Value::Table(t)) => t.clone(),
        Some(_) => return Err(format!("{name}: `colors` is not a table")),
        None => return Err(format!("{name}: missing [colors] section")),
    };
    let get = |section: &str, key: &str| -> Result<Option<u32>, String> {
        let Some(sec) = colors.get(section) else { return Ok(None) };
        let sec = sec.as_table().ok_or_else(|| format!("{name}: colors.{section} is not a table"))?;
        match sec.get(key) {
            None => Ok(None),
            Some(toml::Value::String(s)) => {
                // Alacritty allows "CellForeground"/"CellBackground" sentinels;
                // treat them as unset.
                if s.starts_with("Cell") {
                    return Ok(None);
                }
                parse_hex(s).map(Some).map_err(|e| format!("{name}: colors.{section}.{key}: {e}"))
            }
            Some(_) => Err(format!("{name}: colors.{section}.{key} is not a string")),
        }
    };

    const NAMES: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];
    let mut b = Builder {
        foreground: get("primary", "foreground")?,
        background: get("primary", "background")?,
        cursor: get("cursor", "cursor")?,
        cursor_text: get("cursor", "text")?,
        selection_background: get("selection", "background")?,
        selection_foreground: get("selection", "text")?,
        ..Default::default()
    };
    for (i, n) in NAMES.iter().enumerate() {
        b.palette[i] = get("normal", n)?;
        b.palette[i + 8] = get("bright", n)?;
    }
    Ok(b.finish(name))
}

// ---------------------------------------------------------------- Loading

pub fn load_file(path: &Path) -> Result<Theme, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_file(path, &text)
}

/// `text` as the theme file at `path` would be read: the format follows the
/// extension, the name the file stem.
pub fn parse_file(path: &Path, text: &str) -> Result<Theme, String> {
    let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Unnamed".into());
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
    match ext.as_deref() {
        Some("itermcolors") => parse_iterm(&name, text),
        Some("toml") => parse_alacritty(&name, text),
        _ => parse_ghostty(&name, text),
    }
}

/// Bundled palettes (display name, Ghostty-format source), the imported themes. First is the
/// default. Wormhole's own themes are theme files (`tokens::own`), listed after these.
const BUILTIN: &[(&str, &str)] = &[
    ("Tokyo Night", include_str!("themes/TokyoNight")),
    ("Catppuccin Mocha", include_str!("themes/Catppuccin Mocha")),
    ("Catppuccin Latte", include_str!("themes/Catppuccin Latte")),
    ("Dracula", include_str!("themes/Dracula")),
    ("Nord", include_str!("themes/Nord")),
    ("Gruvbox Dark", include_str!("themes/Gruvbox Dark")),
    ("One Dark", include_str!("themes/Atom One Dark")),
    ("Solarized Dark", include_str!("themes/Solarized Dark Patched")),
    ("Rose Pine", include_str!("themes/Rose Pine")),
    ("Kanagawa Wave", include_str!("themes/Kanagawa Wave")),
    ("Everforest Dark", include_str!("themes/Everforest Dark Hard")),
    ("GitHub Dark", include_str!("themes/GitHub Dark")),
    ("GitHub Light", include_str!("themes/GitHub Light Default")),
    ("Monokai Pro", include_str!("themes/Monokai Pro")),
];

/// All bundled themes; the first entry (Tokyo Night) is the default. A copy
/// of [`builtins`].
pub fn builtin() -> Vec<Theme> {
    builtins().to_vec()
}

/// All bundled themes, parsed once; the first entry (Tokyo Night) is the default.
pub fn builtins() -> &'static [Theme] {
    static THEMES: LazyLock<Vec<Theme>> = LazyLock::new(|| {
        BUILTIN
            .iter()
            .map(|(name, src)| {
                parse_ghostty(name, src).unwrap_or_else(|e| panic!("bundled theme {name}: {e}"))
            })
            .collect()
    });
    &THEMES
}

/// True if the background's WCAG relative luminance is past the point where
/// dark text contrasts better than light text.
pub fn is_light(theme: &Theme) -> bool {
    is_light_background(theme.background)
}

/// [`is_light`] for a packed `0xRRGGBB` background.
pub fn is_light_background(bg: u32) -> bool {
    fn lin(c: u32) -> f64 {
        let v = c as f64 / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    }
    let l = 0.2126 * lin(bg >> 16 & 0xff) + 0.7152 * lin(bg >> 8 & 0xff) + 0.0722 * lin(bg & 0xff);
    // 0.179 is the luminance where black and white text have equal contrast.
    l > 0.179
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_forms() {
        assert_eq!(parse_hex("#1a2b3c"), Ok(0x1a2b3c));
        assert_eq!(parse_hex("1A2B3C"), Ok(0x1a2b3c));
        assert_eq!(parse_hex("0x1a2b3c"), Ok(0x1a2b3c));
        assert_eq!(parse_hex("\"#abc\""), Ok(0xaabbcc));
        assert!(parse_hex("#12345").is_err());
        assert!(parse_hex("#zzzzzz").is_err());
        assert!(parse_hex("+12345").is_err());
    }

    #[test]
    fn ghostty_full() {
        let src = "# a comment\n\n  palette = 0=#010203\npalette=15=ffeedd\n\
                   background = #102030\nforeground=e0e0e0\n  cursor-color = #ff0000  \n\
                   cursor-text = #000000\nselection-background = #333333\n\
                   selection-foreground = #eeeeee\nfont-size = 12\npalette = 200=#123456\n";
        let t = parse_ghostty("G", src).unwrap();
        assert_eq!(t.name, "G");
        assert_eq!(t.palette[0], 0x010203);
        assert_eq!(t.palette[15], 0xffeedd);
        assert_eq!(t.palette[1], XTERM_PALETTE[1]);
        assert_eq!(t.background, 0x102030);
        assert_eq!(t.foreground, 0xe0e0e0);
        assert_eq!(t.cursor, 0xff0000);
        assert_eq!(t.cursor_text, Some(0));
        assert_eq!(t.selection_background, 0x333333);
        assert_eq!(t.selection_foreground, Some(0xeeeeee));
    }

    #[test]
    fn ghostty_defaults_and_errors() {
        let t = parse_ghostty("D", "background = #000000\nforeground = #ffffff\n").unwrap();
        assert_eq!(t.cursor, 0xffffff);
        assert_eq!(t.cursor_text, None);
        assert_eq!(t.selection_background, mix(0xffffff, 0, 0.3));
        assert_eq!(t.selection_background, 0x4d4d4d);
        assert_eq!(t.palette, XTERM_PALETTE);
        assert!(parse_ghostty("E", "background #000").is_err());
        assert!(parse_ghostty("E", "background = nothex").is_err());
        assert!(parse_ghostty("E", "palette = x=#000000").is_err());
    }

    const ITERM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Ansi 0 Color</key>
  <dict>
    <key>Alpha Component</key><real>1</real>
    <key>Blue Component</key><real>0.0</real>
    <key>Color Space</key><string>sRGB</string>
    <key>Green Component</key><real>0.0</real>
    <key>Red Component</key><real>0.0</real>
  </dict>
  <key>Ansi 1 Color</key>
  <dict>
    <key>Blue Component</key><real>0.0</real>
    <key>Green Component</key><real>0.0</real>
    <key>Red Component</key><real>1</real>
  </dict>
  <key>Background Color</key>
  <dict>
    <key>Blue Component</key><real>0.14901961386203766</real>
    <key>Green Component</key><real>0.10588235408067703</real>
    <key>Red Component</key><real>0.10196078568696976</real>
    <key>Color Space</key><string>Calibrated</string>
  </dict>
  <key>Foreground Color</key>
  <dict>
    <key>Blue Component</key><real>0.96078431606292725</real>
    <key>Green Component</key><real>0.792156994342804</real>
    <key>Red Component</key><real>0.75294119119644165</real>
  </dict>
  <key>Selection Color</key>
  <dict>
    <key>Blue Component</key><real>0.5</real>
    <key>Green Component</key><real>0.5</real>
    <key>Red Component</key><real>0.5</real>
  </dict>
  <key>Selected Text Color</key>
  <dict>
    <key>Blue Component</key><real>1</real>
    <key>Green Component</key><real>1</real>
    <key>Red Component</key><real>1</real>
  </dict>
  <key>Cursor Text Color</key>
  <dict>
    <key>Blue Component</key><real>0</real>
    <key>Green Component</key><real>0</real>
    <key>Red Component</key><real>0</real>
  </dict>
</dict>
</plist>"#;

    #[test]
    fn iterm() {
        let t = parse_iterm("I", ITERM).unwrap();
        assert_eq!(t.background, 0x1a1b26);
        assert_eq!(t.foreground, 0xc0caf5);
        assert_eq!(t.cursor, t.foreground); // missing -> foreground
        assert_eq!(t.cursor_text, Some(0));
        assert_eq!(t.selection_background, 0x808080);
        assert_eq!(t.selection_foreground, Some(0xffffff));
        assert_eq!(t.palette[0], 0x000000);
        assert_eq!(t.palette[1], 0xff0000);
        assert_eq!(t.palette[2], XTERM_PALETTE[2]);
        assert!(parse_iterm("bad", "<not a plist").is_err());
    }

    #[test]
    fn alacritty() {
        let src = r##"
[colors.primary]
background = "#1a1b26"
foreground = "0xc0caf5"

[colors.cursor]
cursor = "#ff9e64"
text = "CellBackground"

[colors.selection]
background = "#33467c"
text = "#c0caf5"

[colors.normal]
black = "#15161e"
red = "#f7768e"

[colors.bright]
white = "0xffffff"
"##;
        let t = parse_alacritty("A", src).unwrap();
        assert_eq!(t.background, 0x1a1b26);
        assert_eq!(t.foreground, 0xc0caf5);
        assert_eq!(t.cursor, 0xff9e64);
        assert_eq!(t.cursor_text, None);
        assert_eq!(t.selection_background, 0x33467c);
        assert_eq!(t.selection_foreground, Some(0xc0caf5));
        assert_eq!(t.palette[0], 0x15161e);
        assert_eq!(t.palette[1], 0xf7768e);
        assert_eq!(t.palette[2], XTERM_PALETTE[2]);
        assert_eq!(t.palette[15], 0xffffff);
        assert!(parse_alacritty("A", "[colors.primary]\nbackground = \"#zz\"").is_err());
        assert!(parse_alacritty("A", "x = 1").is_err());
    }

    #[test]
    fn load_file_dispatch() {
        let dir = std::env::temp_dir().join(format!("wormhole-themes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cases = [
            ("Mine.itermcolors", ITERM, 0x1a1b26),
            ("Ala.toml", "[colors.primary]\nbackground = \"#010101\"\n", 0x010101),
            ("Ghost", "background = 020202\n", 0x020202),
        ];
        for (file, body, bg) in cases {
            let p = dir.join(file);
            std::fs::write(&p, body).unwrap();
            let t = load_file(&p).unwrap();
            assert_eq!(t.background, bg, "{file}");
            assert_eq!(t.name, Path::new(file).file_stem().unwrap().to_str().unwrap());
        }
        assert!(load_file(&dir.join("missing")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn builtins_parse() {
        let themes = builtin();
        assert_eq!(themes.len(), BUILTIN.len());
        assert_eq!(themes[0].name, "Tokyo Night");
        for (t, (_, src)) in themes.iter().zip(BUILTIN) {
            // Every bundled file should specify the full 16-color palette and core colors.
            for i in 0..16 {
                assert!(src.contains(&format!("palette = {i}=")), "{} lacks palette {i}", t.name);
            }
            assert!(src.contains("background ="), "{}", t.name);
            assert!(src.contains("foreground ="), "{}", t.name);
            assert_ne!(t.background, t.foreground, "{}", t.name);
        }
        let mut names: Vec<_> = themes.iter().map(|t| &t.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), themes.len(), "duplicate theme names");
    }

    #[test]
    fn light_detection() {
        let by = |n: &str| builtin().into_iter().find(|t| t.name == n).unwrap();
        assert!(!is_light(&by("Tokyo Night")));
        assert!(!is_light(&by("Dracula")));
        assert!(is_light(&by("Catppuccin Latte")));
        assert!(is_light(&by("GitHub Light")));
        let lights: Vec<_> = builtin().into_iter().filter(is_light).map(|t| t.name).collect();
        assert_eq!(lights, ["Catppuccin Latte", "GitHub Light"]);
    }
}
