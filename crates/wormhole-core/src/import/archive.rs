//! Terminal.app stores a font and each colour as an `NSKeyedArchiver` archive: a property list in a
//! `<data>` value, a flat list of objects that point at each other by index. Nothing here
//! unarchives anything: it reads the few plain values the archive holds by walking the list.

use plist::{Dictionary, Value};

use super::plist_value;
use super::units;

/// An archive's objects and where its root is.
pub(crate) struct Archive {
    objects: Vec<Value>,
    root: usize,
}

/// A colour as Terminal.app stores it: red, green, blue and alpha, each from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rgba {
    pub(crate) red: f32,
    pub(crate) green: f32,
    pub(crate) blue: f32,
    pub(crate) alpha: f32,
}

impl Rgba {
    /// The colour as `0xRRGGBB`.
    pub(crate) fn packed(self) -> u32 {
        let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u32;
        byte(self.red) << 16 | byte(self.green) << 8 | byte(self.blue)
    }
}

impl Archive {
    /// The archive in a `<data>` value, or `None` when it is not one.
    pub(crate) fn open(data: &[u8]) -> Option<Self> {
        let Value::Dictionary(archive) = plist_value::parse(data)? else { return None };
        let objects = archive.get("$objects")?.as_array()?.clone();
        let root = archive.get("$top")?.as_dictionary()?.get("root")?.as_uid()?.get();
        Some(Self { objects, root: usize::try_from(root).ok()? })
    }

    /// The archived object itself.
    fn root(&self) -> Option<&Dictionary> {
        self.objects.get(self.root)?.as_dictionary()
    }

    /// A value of the root, with a reference to another object followed.
    fn value(&self, key: &str) -> Option<&Value> {
        let value = self.root()?.get(key)?;
        match value.as_uid() {
            Some(uid) => self.objects.get(usize::try_from(uid.get()).ok()?),
            None => Some(value),
        }
    }

    /// An archived `NSFont`: its PostScript name and size in points.
    pub(crate) fn font(&self) -> Option<(String, f32)> {
        let name = self.value("NSName")?.as_string()?.to_string();
        let size = self.value("NSSize")?.as_real().and_then(units::finite)?;
        Some((name, size))
    }

    /// An archived `NSColor`. The components are text in a NUL-ended byte string: `NSRGB` for red,
    /// green, blue and maybe alpha, `NSWhite` for a grey and maybe alpha. Colours in a colour space of
    /// their own are read as sRGB, which is close; converting them takes ColorSync.
    pub(crate) fn color(&self) -> Option<Rgba> {
        let numbers = |key: &str| -> Option<Vec<f32>> {
            let text = match self.value(key)? {
                Value::Data(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                Value::String(text) => text.clone(),
                _ => return None,
            };
            // `NaN` and `inf` parse as numbers; neither is a colour.
            text.trim_end_matches('\0')
                .split_whitespace()
                .map(|n| n.parse::<f32>().ok().filter(|n| n.is_finite()))
                .collect()
        };
        if let Some(rgb) = numbers("NSRGB") {
            let [red, green, blue, rest @ ..] = rgb.as_slice() else { return None };
            return Some(Rgba {
                red: *red,
                green: *green,
                blue: *blue,
                alpha: rest.first().copied().unwrap_or(1.0),
            });
        }
        let white = numbers("NSWhite")?;
        let grey = *white.first()?;
        Some(Rgba { red: grey, green: grey, blue: grey, alpha: white.get(1).copied().unwrap_or(1.0) })
    }
}
