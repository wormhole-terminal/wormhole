//! CoreText's outlines. This is the module's only unsafe code: the CoreText calls and the
//! callback `CGPathApply` takes.

use std::ffi::c_void;
use std::ptr::{self, NonNull};

use objc2_core_foundation::{CFData, CFRange, CFRetained, CFString, CGAffineTransform};
use objc2_core_graphics::{CGPath, CGPathElement, CGPathElementType};
use objc2_core_text::{CTFont, CTFontManagerCreateFontDescriptorFromData, CTFontSymbolicTraits};
use wormhole_core::fonts::bundled;

use crate::platform::font_face::Seg;

/// A face's size when it is made: its outlines are asked for at the size they are drawn at.
const BASE_SIZE: f64 = 12.0;

/// A CoreText font.
pub(crate) struct Face(CFRetained<CTFont>);

impl Face {
    /// The face of `family` with the slant and weight asked for. CoreText gives a default font
    /// for a family it doesn't know; that shows up as glyphs that aren't the shaped ones.
    #[allow(unsafe_code)]
    pub fn named(family: &str, bold: bool, italic: bool) -> Option<Face> {
        // The bundled font is only in GPUI's tables, not CoreText's: its bytes make the face.
        if family == bundled::FAMILY {
            return Face::from_bundled(bold, italic);
        }
        let name = CFString::from_str(family);
        // SAFETY: `name` is a live CFString and a null matrix means identity.
        let font = unsafe { CTFont::with_name(&name, BASE_SIZE, ptr::null()) };
        let mut traits = 0;
        if bold {
            traits |= CTFontSymbolicTraits::TraitBold.0;
        }
        if italic {
            traits |= CTFontSymbolicTraits::TraitItalic.0;
        }
        if traits == 0 {
            return Some(Face(font));
        }
        let traits = CTFontSymbolicTraits(traits);
        // SAFETY: as above; the traits are both the value and the mask, so only these change.
        let styled = unsafe { font.copy_with_symbolic_traits(BASE_SIZE, ptr::null(), traits, traits) };
        // A family without a bold or an italic face: the plain one, as GPUI's font match gives.
        Some(Face(styled.unwrap_or(font)))
    }

    /// The bundled font's face, made from the bytes the app ships (regular, bold, italic or bold
    /// italic) without registering it with CoreText, which would show it to the whole process.
    #[allow(unsafe_code)]
    fn from_bundled(bold: bool, italic: bool) -> Option<Face> {
        let bytes = bundled::FACES.get(usize::from(bold) + 2 * usize::from(italic))?;
        let data = CFData::from_bytes(bytes);
        // SAFETY: `data` is a live CFData; CoreText copies what it keeps.
        let descriptor = unsafe { CTFontManagerCreateFontDescriptorFromData(&data) }?;
        // SAFETY: a null matrix means identity.
        Some(Face(unsafe { CTFont::with_font_descriptor(&descriptor, BASE_SIZE, ptr::null()) }))
    }

    /// The font CoreText substitutes for `c` in this one, as it does for text it shapes.
    #[allow(unsafe_code)]
    pub fn for_char(&self, c: char) -> Option<Face> {
        let text = CFString::from_str(c.encode_utf8(&mut [0; 4]));
        let range = CFRange { location: 0, length: c.len_utf16() as isize };
        // SAFETY: the range is the whole of `text`, which is one character.
        Some(Face(unsafe { self.0.for_string(&text, range) }))
    }

    /// The face's glyph for `c`, if it has one.
    #[allow(unsafe_code)]
    pub fn glyph_of(&self, c: char) -> Option<u16> {
        let mut units = [0u16; 2];
        let count = c.encode_utf16(&mut units).len();
        let mut glyphs = [0u16; 2];
        // SAFETY: both buffers have `count` (at most two) elements and outlive the call.
        let mapped = unsafe {
            self.0.glyphs_for_characters(
                NonNull::from(&mut units).cast(),
                NonNull::from(&mut glyphs).cast(),
                count as isize,
            )
        };
        (mapped && glyphs[0] != 0).then_some(glyphs[0])
    }

    /// The outline of `glyph` at `size` points, scaled by `scale` along x and y (y is down in the
    /// result) and moved `shift_x` to the right. `None` for a glyph with no outline.
    #[allow(unsafe_code)]
    pub fn outline(&self, glyph: u32, size: f32, scale: (f32, f32), shift_x: f32) -> Option<Vec<Seg>> {
        let glyph = u16::try_from(glyph).ok()?;
        // SAFETY: a null matrix keeps the font's own, and no descriptor adds nothing.
        let font = unsafe { self.0.copy_with_attributes(f64::from(size), ptr::null(), None) };
        let matrix = CGAffineTransform {
            a: f64::from(scale.0),
            b: 0.0,
            c: 0.0,
            d: -f64::from(scale.1),
            tx: f64::from(shift_x),
            ty: 0.0,
        };
        // SAFETY: `matrix` is a valid transform for the duration of the call.
        let path = unsafe { font.path_for_glyph(glyph, &matrix) }?;
        let mut segs: Vec<Seg> = Vec::new();
        // SAFETY: `info` is `segs`, which stays put and is touched by nothing else until the call
        // returns, and `collect` casts it back to what it is.
        unsafe { CGPath::apply(Some(&path), ptr::from_mut(&mut segs).cast(), Some(collect)) };
        Some(segs)
    }
}

/// `CGPathApply`'s callback: one path element into the `Vec<Seg>` behind `info`.
///
/// # Safety
///
/// `info` is a `*mut Vec<Seg>` no one else uses, and `element` points at a valid element whose
/// `points` has as many points as its type needs (one, one, two, three, none).
#[allow(unsafe_code)]
unsafe extern "C-unwind" fn collect(info: *mut c_void, element: NonNull<CGPathElement>) {
    // SAFETY: the contract above.
    let (segs, element) = unsafe { (&mut *info.cast::<Vec<Seg>>(), element.as_ref()) };
    let at = |i: usize| {
        // SAFETY: `i` is below the number of points the element's type has.
        let p = unsafe { *element.points.as_ptr().add(i) };
        [p.x as f32, p.y as f32]
    };
    let kind = element.r#type;
    if kind == CGPathElementType::MoveToPoint {
        let p = at(0);
        segs.push(Seg::Move(p[0], p[1]));
    } else if kind == CGPathElementType::AddLineToPoint {
        let p = at(0);
        segs.push(Seg::Line(p[0], p[1]));
    } else if kind == CGPathElementType::AddQuadCurveToPoint {
        segs.push(Seg::Quad(at(0), at(1)));
    } else if kind == CGPathElementType::AddCurveToPoint {
        segs.push(Seg::Cubic(at(0), at(1), at(2)));
    } else if kind == CGPathElementType::CloseSubpath {
        segs.push(Seg::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::{Face, Seg};

    /// The width of an outline's points.
    fn width(segs: &[Seg]) -> f32 {
        let xs: Vec<f32> = segs
            .iter()
            .flat_map(|seg| match *seg {
                Seg::Move(x, _) | Seg::Line(x, _) => vec![x],
                Seg::Quad(a, b) => vec![a[0], b[0]],
                Seg::Cubic(a, b, c) => vec![a[0], b[0], c[0]],
                Seg::Close => vec![],
            })
            .collect();
        xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min)
    }

    #[test]
    fn menlos_h_is_twice_as_wide_stretched_and_as_tall_as_ever() {
        let face = Face::named("Menlo", false, false).expect("a face");
        let glyph = face.glyph_of('H').expect("Menlo has an H");
        let plain = face.outline(u32::from(glyph), 14.0, (2.0, 2.0), 0.0).expect("an outline");
        let wide = face.outline(u32::from(glyph), 14.0, (4.0, 2.0), 0.0).expect("an outline");
        assert!((width(&wide) - 2.0 * width(&plain)).abs() < 0.01, "{} vs {}", width(&wide), width(&plain));
        let height = |segs: &[Seg]| {
            let ys: Vec<f32> = segs
                .iter()
                .filter_map(|s| if let Seg::Line(_, y) | Seg::Move(_, y) = *s { Some(y) } else { None })
                .collect();
            ys.iter().cloned().fold(f32::MIN, f32::max) - ys.iter().cloned().fold(f32::MAX, f32::min)
        };
        assert!((height(&wide) - height(&plain)).abs() < 0.01);
        // y is down: the letter sits above its baseline.
        assert!(wide.iter().any(|s| matches!(s, Seg::Line(_, y) if *y < -5.0)));
    }

    #[test]
    fn the_bundled_font_has_faces_from_its_bytes_and_a_fallback_for_what_it_lacks() {
        for (bold, italic) in [(false, false), (true, false), (false, true), (true, true)] {
            let face = Face::named("JetBrains Mono", bold, italic).expect("a bundled face");
            let glyph = face.glyph_of('H').expect("an H");
            assert!(face.outline(u32::from(glyph), 14.0, (4.0, 2.0), 0.0).is_some_and(|s| !s.is_empty()));
        }
        let face = Face::named("JetBrains Mono", false, false).expect("a face");
        assert!(face.glyph_of('日').is_none());
        let fallback = face.for_char('日').expect("a substitute");
        assert!(fallback.glyph_of('日').is_some(), "the substitute has the character");
    }

    #[test]
    fn a_shift_moves_the_outline_right() {
        let face = Face::named("Menlo", false, false).expect("a face");
        let glyph = u32::from(face.glyph_of('l').expect("an l"));
        let first = |segs: Vec<Seg>| match segs[0] {
            Seg::Move(x, _) => x,
            other => panic!("{other:?}"),
        };
        let at = first(face.outline(glyph, 14.0, (2.0, 1.0), 0.0).expect("outline"));
        let shifted = first(face.outline(glyph, 14.0, (2.0, 1.0), 0.5).expect("outline"));
        assert!((shifted - at - 0.5).abs() < 0.001);
    }

    #[test]
    fn a_space_has_no_outline_and_a_cjk_character_finds_its_fallback() {
        let face = Face::named("Menlo", false, false).expect("a face");
        let space = u32::from(face.glyph_of(' ').expect("a space"));
        assert!(face.outline(space, 14.0, (2.0, 1.0), 0.0).is_none_or(|segs| segs.is_empty()));
        assert!(face.glyph_of('日').is_none(), "Menlo has no CJK");
        let fallback = face.for_char('日').expect("CoreText substitutes a font");
        let glyph = fallback.glyph_of('日').expect("the substitute has it");
        assert!(fallback.outline(u32::from(glyph), 14.0, (2.0, 1.0), 0.0).is_some());
    }
}
