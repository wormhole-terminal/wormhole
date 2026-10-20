//! Glyphs stretched to twice their width, for the rows DECDWL asks for.
//!
//! GPUI paints a glyph at a font size, both dimensions at once, and has no transform for it. What
//! it does have is `Window::paint_svg`: an alpha mask rasterized from SVG data, cached in the
//! atlas by `(path, size)` and coloured in the shader. So a glyph of a double-width row is its
//! outline from the font (CoreText's `CTFontCreatePathForGlyph`, in `face`) with the stretch baked
//! into the coordinates, written as a one-path SVG in device pixels. Nothing here is resampled:
//! the vector goes to resvg already stretched.
//!
//! - **Which face?** GPUI's `FontId` is an index in its own font table and says nothing about the
//!   font behind it (a fallback font, found while shaping, isn't even registered under a `Font`).
//!   So the face is learned while a word is shaped, where the text, the run's `Font` and the
//!   shaped glyphs are all at hand (`Outlines::learn`): the run's font by name, weight and
//!   slant, or for a fallback font what CoreText picks for the run's character. A face counts only
//!   once one of the run's glyphs is the very glyph the face has for its character; a font that
//!   can't be pinned down this way keeps the centred, unstretched glyphs `sized` falls back to.
//! - **Cache.** An SVG is built once per (font, glyph, size, scale, stretch, quarter pixel) and
//!   kept in a bounded map, so a steady frame builds and allocates nothing: it looks the sprite
//!   up and hands GPUI the same bytes and the same path key, which GPUI finds in its atlas.
//! - **Position.** `paint_svg` snaps its bounds to device pixels, so the glyph's quarter-pixel
//!   x (what `paint_glyph` keeps as a subpixel variant) is baked into the path, and the baseline
//!   is rounded as `paint_glyph` does.
//!
//! Colour glyphs (emoji) are bitmaps and never come here.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use gpui_kit::{
    Bounds, FontId, GlyphId, Pixels, Point, ShapedLine, SharedString, TextRun, WindowTextSystem, point, px,
    size,
};

mod face;

use face::Face;

/// How many sprites are kept; past that the map starts over, as the word cache does.
const MAX_SPRITES: usize = 4096;

/// Quarter-pixel positions, as GPUI's own glyphs have.
const SUBPIXEL_VARIANTS_X: f32 = 4.0;

/// How often a face is tried for a font before it is given up on (a word shaped each time).
const LEARN_ATTEMPTS: u8 = 8;

/// The largest sprite, in device pixels along either side: a glyph outline can't make a mask
/// bigger than a window.
const MAX_SIDE: i32 = 2048;

/// How much a glyph is stretched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct Stretch {
    pub x: u8,
    pub y: u8,
}

impl Stretch {
    /// DECDWL: twice as wide, as tall as ever.
    pub const WIDE: Stretch = Stretch { x: 2, y: 1 };
}

/// One piece of an outline, in device pixels from the glyph's origin, y down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Quad([f32; 2], [f32; 2]),
    Cubic([f32; 2], [f32; 2], [f32; 2]),
    Close,
}

/// A glyph ready to hand to `paint_svg`.
pub(super) struct Sprite {
    /// What GPUI's atlas keys the mask by; unique to the content.
    path: SharedString,
    svg: Box<[u8]>,
    /// Where the mask's top left is from the glyph's integer device-pixel origin, and its size,
    /// in device pixels.
    left: i32,
    top: i32,
    width: u32,
    height: u32,
}

impl Sprite {
    pub fn path(&self) -> &SharedString {
        &self.path
    }

    pub fn svg(&self) -> &[u8] {
        &self.svg
    }
}

/// What a sprite is a drawing of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SpriteKey {
    font: FontId,
    glyph: u32,
    /// The font size in points, as bits (a size is never NaN).
    size: u32,
    scale: u32,
    stretch: Stretch,
    /// The glyph's quarter pixel, 0 to 3.
    sub: u8,
}

/// Where a glyph's origin falls on the device pixels: its integer pixel and quarter pixel.
fn quantize(origin: Point<Pixels>, scale: f32) -> (i32, i32, u8) {
    let x = round_half_toward_zero(f32::from(origin.x) * scale * SUBPIXEL_VARIANTS_X) / SUBPIXEL_VARIANTS_X;
    let y = round_half_toward_zero(f32::from(origin.y) * scale);
    let sub = ((x - x.trunc()).abs() * SUBPIXEL_VARIANTS_X) as u8;
    (x.trunc() as i32, y as i32, sub.min(3))
}

/// Rounds half toward zero, as GPUI does for glyph positions.
fn round_half_toward_zero(value: f32) -> f32 {
    (value.abs() - 0.5).ceil().copysign(value)
}

/// The faces learned and the sprites built.
#[derive(Default)]
pub(super) struct Outlines {
    faces: HashMap<FontId, Learned>,
    sprites: HashMap<SpriteKey, Option<Rc<Sprite>>>,
}

enum Learned {
    Face(Face),
    /// Not pinned down yet: how many words have tried.
    Trying(u8),
}

impl Outlines {
    /// A word was just shaped into `line`: learn the face of each font its glyphs use that isn't
    /// known yet. `text` is what was shaped and `runs` the text runs it was shaped with.
    pub fn learn(&mut self, text_system: &WindowTextSystem, text: &str, runs: &[TextRun], line: &ShapedLine) {
        for run in &line.runs {
            let known = match self.faces.get(&run.font_id) {
                Some(Learned::Face(_)) => true,
                Some(Learned::Trying(attempts)) => *attempts >= LEARN_ATTEMPTS,
                None => false,
            };
            if known || run.glyphs.iter().all(|glyph| glyph.is_emoji) {
                continue;
            }
            let attempts = match self.faces.get(&run.font_id) {
                Some(Learned::Trying(attempts)) => *attempts,
                _ => 0,
            };
            let learned = face_of_run(text_system, text, runs, run)
                .map_or(Learned::Trying(attempts + 1), Learned::Face);
            self.faces.insert(run.font_id, learned);
        }
    }

    /// The sprite of `glyph` at `origin` (its baseline's left end) stretched, and where to paint
    /// it: `None` when the glyph has no known outline, which is what a blank has, or when its
    /// font couldn't be pinned down.
    pub fn stretched(
        &mut self,
        (font, glyph): (FontId, GlyphId),
        font_size: f32,
        scale: f32,
        stretch: Stretch,
        origin: Point<Pixels>,
    ) -> Option<(Rc<Sprite>, Bounds<Pixels>)> {
        let (ix, iy, sub) = quantize(origin, scale);
        let key = SpriteKey {
            font,
            glyph: glyph.0,
            size: font_size.to_bits(),
            scale: scale.to_bits(),
            stretch,
            sub,
        };
        let faces = &self.faces;
        let sprite = sprite_with(&mut self.sprites, key, || {
            let Some(Learned::Face(face)) = faces.get(&font) else { return None };
            let shift = f32::from(sub) / SUBPIXEL_VARIANTS_X;
            let segs = face.outline(
                glyph.0,
                font_size,
                (stretch.x as f32 * scale, stretch.y as f32 * scale),
                shift,
            )?;
            build(&segs, &key)
        })?;
        let bounds = Bounds::new(
            point(px((ix + sprite.left) as f32 / scale), px((iy + sprite.top) as f32 / scale)),
            size(px(sprite.width as f32 / scale), px(sprite.height as f32 / scale)),
        );
        Some((sprite, bounds))
    }
}

/// The sprite under `key`, built by `build` when it isn't there (a failed build is remembered
/// too, so a blank glyph isn't asked for again). The map starts over at its cap.
fn sprite_with(
    sprites: &mut HashMap<SpriteKey, Option<Rc<Sprite>>>,
    key: SpriteKey,
    build: impl FnOnce() -> Option<Sprite>,
) -> Option<Rc<Sprite>> {
    if let Some(found) = sprites.get(&key) {
        return found.clone();
    }
    if sprites.len() >= MAX_SPRITES {
        sprites.clear();
    }
    let built = build().map(Rc::new);
    sprites.insert(key, built.clone());
    built
}

/// The face behind a shaped run of `text`, if one can be told: the face of the text run's font
/// when GPUI used it as asked, else (a fallback font) what CoreText picks for the character; and
/// then only if a glyph of the run is the face's own glyph for its character.
fn face_of_run(
    text_system: &WindowTextSystem,
    text: &str,
    runs: &[TextRun],
    run: &gpui_kit::ShapedRun,
) -> Option<Face> {
    let first = run.glyphs.iter().find(|glyph| !glyph.is_emoji)?;
    let run_font = text_run_at(runs, first.index)?;
    let base = Face::named(
        &run_font.font.family,
        run_font.font.weight.0 >= 600.0,
        run_font.font.style != gpui_kit::FontStyle::Normal,
    )?;
    let asked = text_system.resolve_font(&run_font.font) == run.font_id;
    // A glyph that is its character's own, in the face a candidate would be.
    let proves = |face: &Face| {
        run.glyphs.iter().filter(|glyph| !glyph.is_emoji).any(|glyph| {
            text.get(glyph.index..)
                .and_then(|rest| rest.chars().next())
                .is_some_and(|c| face.glyph_of(c).is_some_and(|id| u32::from(id) == glyph.id.0))
        })
    };
    if asked {
        return proves(&base).then_some(base);
    }
    // A fallback font: the one CoreText picks for a character of the run, which is the one
    // GPUI's shaping got.
    for glyph in run.glyphs.iter().filter(|glyph| !glyph.is_emoji) {
        let Some(c) = text.get(glyph.index..).and_then(|rest| rest.chars().next()) else { continue };
        if let Some(candidate) = base.for_char(c)
            && proves(&candidate)
        {
            return Some(candidate);
        }
    }
    None
}

/// The text run that covers byte `index`.
fn text_run_at(runs: &[TextRun], index: usize) -> Option<&TextRun> {
    let mut start = 0;
    runs.iter().find(|run| {
        let end = start + run.len;
        let hit = (start..end).contains(&index);
        start = end;
        hit
    })
}

/// A glyph's outline as a sprite: the bounds of its points, a pixel of room all round, and the
/// path as an SVG as big as that in device pixels. `None` for a blank glyph and for one too big
/// to be real.
fn build(segs: &[Seg], key: &SpriteKey) -> Option<Sprite> {
    let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
    let mut grow = |p: [f32; 2]| {
        for axis in 0..2 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    };
    for seg in segs {
        match *seg {
            Seg::Move(x, y) | Seg::Line(x, y) => grow([x, y]),
            Seg::Quad(a, b) => [a, b].into_iter().for_each(&mut grow),
            Seg::Cubic(a, b, c) => [a, b, c].into_iter().for_each(&mut grow),
            Seg::Close => {}
        }
    }
    if min[0] > max[0] || !min.iter().chain(&max).all(|v| v.is_finite()) {
        return None;
    }
    let (left, top) = (min[0].floor() as i32 - 1, min[1].floor() as i32 - 1);
    let (right, bottom) = (max[0].ceil() as i32 + 1, max[1].ceil() as i32 + 1);
    let (width, height) = (right - left, bottom - top);
    if width > MAX_SIDE || height > MAX_SIDE {
        return None;
    }
    let (dx, dy) = (-left as f32, -top as f32);
    let mut svg = String::with_capacity(64 + segs.len() * 24);
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\"><path d=\""
    );
    let pt = |svg: &mut String, p: [f32; 2]| {
        let _ = write!(svg, " {:.2} {:.2}", p[0] + dx, p[1] + dy);
    };
    for seg in segs {
        match *seg {
            Seg::Move(x, y) => {
                svg.push('M');
                pt(&mut svg, [x, y]);
            }
            Seg::Line(x, y) => {
                svg.push('L');
                pt(&mut svg, [x, y]);
            }
            Seg::Quad(a, b) => {
                svg.push('Q');
                pt(&mut svg, a);
                pt(&mut svg, b);
            }
            Seg::Cubic(a, b, c) => {
                svg.push('C');
                pt(&mut svg, a);
                pt(&mut svg, b);
                pt(&mut svg, c);
            }
            Seg::Close => svg.push('Z'),
        }
    }
    svg.push_str("\"/></svg>");
    let path = format!(
        "wormhole-glyph/{}/{}/{:x}/{:x}/{}x{}/{}",
        key.font.0, key.glyph, key.size, key.scale, key.stretch.x, key.stretch.y, key.sub
    );
    Some(Sprite {
        path: path.into(),
        svg: svg.into_bytes().into(),
        left,
        top,
        width: width as u32,
        height: height as u32,
    })
}

#[cfg(test)]
mod tests;
