//! iTerm2's inline images: the arguments of `OSC 1337 ; File = key=value;… : <base64> ST`, and
//! of its multipart form (`MultipartFile=`, then `FilePart=` chunks, then `FileEnd`).

use base64::Engine;

/// How wide or tall the image is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    /// The image's own size.
    Auto,
    Cells(u32),
    Pixels(u32),
    /// Of the terminal's width or height.
    Percent(u32),
}

impl Dimension {
    /// `auto`, `N`, `Npx` or `N%`; anything else is `auto`.
    pub fn parse(value: &str) -> Self {
        let value = value.trim();
        let number = |digits: &str| digits.parse::<u32>().ok();
        if let Some(px) = value.strip_suffix("px") {
            number(px).map_or(Dimension::Auto, Dimension::Pixels)
        } else if let Some(percent) = value.strip_suffix('%') {
            number(percent).map_or(Dimension::Auto, |p| Dimension::Percent(p.min(100)))
        } else {
            number(value).map_or(Dimension::Auto, Dimension::Cells)
        }
    }

    /// The cells this takes, given the cell size in pixels and the terminal's extent in cells,
    /// or `None` for `auto`.
    pub fn cells(self, cell_px: f32, extent: u32) -> Option<u32> {
        Some(match self {
            Dimension::Auto => return None,
            Dimension::Cells(n) => n,
            Dimension::Pixels(px) => (px as f32 / cell_px.max(1.0)).ceil() as u32,
            Dimension::Percent(p) => (extent * p).div_ceil(100),
        })
    }
}

/// `File=`'s arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileArgs {
    /// The file's name (sent base64-encoded), when readable.
    pub name: Option<String>,
    pub size: Option<usize>,
    pub width: Dimension,
    pub height: Dimension,
    pub preserve_aspect_ratio: bool,
    /// Shown in the terminal (otherwise iTerm2 downloads the file: nothing to show).
    pub inline: bool,
}

impl Default for FileArgs {
    fn default() -> Self {
        Self {
            name: None,
            size: None,
            width: Dimension::Auto,
            height: Dimension::Auto,
            preserve_aspect_ratio: true,
            inline: false,
        }
    }
}

/// The `key=value;…` before the colon.
pub fn parse_args(args: &str) -> FileArgs {
    let mut out = FileArgs::default();
    for pair in args.split(';') {
        let Some((key, value)) = pair.split_once('=') else { continue };
        match key.trim() {
            "name" => {
                out.name = base64::engine::general_purpose::STANDARD
                    .decode(value.trim())
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .map(|name| name.chars().filter(|c| !c.is_control()).take(255).collect());
            }
            "size" => out.size = value.trim().parse().ok(),
            "width" => out.width = Dimension::parse(value),
            "height" => out.height = Dimension::parse(value),
            "preserveAspectRatio" => out.preserve_aspect_ratio = value.trim() != "0",
            "inline" => out.inline = value.trim() == "1",
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Dimension, FileArgs, parse_args};

    #[test]
    fn the_arguments_read_with_iterms_units() {
        let args =
            parse_args("name=Y2F0LnBuZw==;size=1234;width=40;height=50%;preserveAspectRatio=0;inline=1");
        assert_eq!(
            args,
            FileArgs {
                name: Some("cat.png".into()),
                size: Some(1234),
                width: Dimension::Cells(40),
                height: Dimension::Percent(50),
                preserve_aspect_ratio: false,
                inline: true,
            }
        );
        assert_eq!(parse_args(""), FileArgs::default());
        assert_eq!(parse_args("inline=1;width=300px;height=auto").width, Dimension::Pixels(300));
        assert_eq!(parse_args("width=lots;name=!!").width, Dimension::Auto);
        assert_eq!(parse_args("width=250%").width, Dimension::Percent(100));
    }

    #[test]
    fn dimensions_turn_into_cells() {
        assert_eq!(Dimension::Auto.cells(8.0, 80), None);
        assert_eq!(Dimension::Cells(12).cells(8.0, 80), Some(12));
        assert_eq!(Dimension::Pixels(100).cells(8.0, 80), Some(13));
        assert_eq!(Dimension::Percent(50).cells(8.0, 81), Some(41));
    }
}
