//! Pixels of animation frames: a canvas of one colour, and a rectangle of one picture put onto
//! another by kitty's two modes (`X` of `a=f`, `C` of `a=c`): alpha blending, or replacing.
//!
//! Spec: kitty, "Animation". The pixels are straight (not premultiplied) RGBA, as the store
//! keeps them. Blending is source over destination, rounded to the nearest 8-bit value.

use wormhole_vt::image::{Blend, FrameRect};

use super::Image;

/// A canvas of `width` by `height` pixels of one colour, `0xRRGGBBAA` as kitty's `Y` has it.
pub fn solid(width: u32, height: u32, color: u32) -> Image {
    let pixel = color.to_be_bytes();
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for _ in 0..width as usize * height as usize {
        rgba.extend_from_slice(&pixel);
    }
    Image { width, height, rgba }
}

/// One pixel of `src` over one of `dst`.
fn over(dst: [u8; 4], src: [u8; 4]) -> [u8; 4] {
    let (sa, da) = (u32::from(src[3]), u32::from(dst[3]));
    if sa == 255 {
        return src;
    }
    if sa == 0 {
        return dst;
    }
    // The weights of the two colours, each scaled by 255 * 255: what the source covers, and what
    // of the destination shows through it.
    let (ws, wd) = (sa * 255, da * (255 - sa));
    let total = ws + wd;
    if total == 0 {
        return [0; 4];
    }
    let mix = |s: u8, d: u8| ((u32::from(s) * ws + u32::from(d) * wd + total / 2) / total) as u8;
    [mix(src[0], dst[0]), mix(src[1], dst[1]), mix(src[2], dst[2]), ((total + 127) / 255).min(255) as u8]
}

/// Put the rectangle `from` of `src` onto `dst` with its corner at `to`. `false`, and nothing
/// done, when a rectangle is not inside its picture; `src` and `dst` are different pictures (a
/// frame onto itself is two clones, made by the caller).
pub fn blit(dst: &mut Image, src: &Image, from: FrameRect, to: (u32, u32), blend: Blend) -> bool {
    let inside = |x: u32, y: u32, width: u32, height: u32, of: &Image| {
        u64::from(x) + u64::from(from.width) <= u64::from(width)
            && u64::from(y) + u64::from(from.height) <= u64::from(height)
            && of.rgba.len() == width as usize * height as usize * 4
    };
    if !inside(from.x, from.y, src.width, src.height, src) || !inside(to.0, to.1, dst.width, dst.height, dst)
    {
        return false;
    }
    let row_bytes = from.width as usize * 4;
    for row in 0..from.height as usize {
        let s = ((from.y as usize + row) * src.width as usize + from.x as usize) * 4;
        let d = ((to.1 as usize + row) * dst.width as usize + to.0 as usize) * 4;
        let (src_row, dst_row) = (&src.rgba[s..s + row_bytes], &mut dst.rgba[d..d + row_bytes]);
        match blend {
            Blend::Replace => dst_row.copy_from_slice(src_row),
            // `Blend` may grow; a mode this does not know is the default one.
            _ => {
                // Whole pixels: both rows are `width * 4` bytes.
                for (d, s) in dst_row.as_chunks_mut::<4>().0.iter_mut().zip(src_row.as_chunks::<4>().0) {
                    *d = over(*d, *s);
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use wormhole_vt::image::{Blend, FrameRect};

    use super::{Image, blit, over, solid};

    fn rect(x: u32, y: u32, width: u32, height: u32) -> FrameRect {
        FrameRect { x, y, width, height }
    }

    #[test]
    fn a_solid_canvas_is_one_colour_in_rgba_order() {
        let canvas = solid(2, 1, 0x11_22_33_44);
        assert_eq!((canvas.width, canvas.height), (2, 1));
        assert_eq!(canvas.rgba, [0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44]);
        assert!(solid(0, 5, 0).rgba.is_empty());
    }

    #[test]
    fn source_over_destination_in_straight_alpha() {
        assert_eq!(over([1, 2, 3, 4], [9, 9, 9, 255]), [9, 9, 9, 255], "opaque source");
        assert_eq!(over([1, 2, 3, 4], [9, 9, 9, 0]), [1, 2, 3, 4], "invisible source");
        assert_eq!(over([0; 4], [10, 20, 30, 128]), [10, 20, 30, 128], "over nothing, the source stays");
        assert_eq!(over([0, 0, 0, 255], [255, 255, 255, 128]), [128, 128, 128, 255], "half white on black");
        // Half red over half blue: alpha 0.75, red weighs two thirds.
        let out = over([0, 0, 255, 128], [255, 0, 0, 128]);
        assert_eq!(out[3], 192);
        assert!(out[0] > out[2] && out[1] == 0, "{out:?}");
        assert_eq!(over([5, 5, 5, 0], [9, 9, 9, 0]), [5, 5, 5, 0]);
    }

    #[test]
    fn a_rectangle_is_blended_or_replaced_into_place() {
        let src = Image {
            width: 2,
            height: 2,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 9, 9, 9, 255],
        };
        let mut blended = solid(3, 3, 0x00_00_00_FF);
        assert!(blit(&mut blended, &src, rect(0, 0, 2, 2), (1, 1), Blend::Over));
        let at = |image: &Image, x: usize, y: usize| image.rgba[(y * 3 + x) * 4..][..4].to_vec();
        assert_eq!(at(&blended, 0, 0), [0, 0, 0, 255], "outside the rectangle");
        assert_eq!(at(&blended, 1, 1), [255, 0, 0, 255]);
        assert_eq!(at(&blended, 2, 1), [0, 128, 0, 255], "half green over black");
        assert_eq!(at(&blended, 1, 2), [0, 0, 0, 255], "an invisible pixel leaves the destination");
        assert_eq!(at(&blended, 2, 2), [9, 9, 9, 255]);
        let mut replaced = solid(3, 3, 0x00_00_00_FF);
        assert!(blit(&mut replaced, &src, rect(0, 0, 2, 2), (1, 1), Blend::Replace));
        assert_eq!(at(&replaced, 1, 2), [0, 0, 255, 0], "replacing copies the alpha too");
    }

    #[test]
    fn a_part_of_the_source_goes_to_a_corner_of_the_destination() {
        let src = Image { width: 2, height: 1, rgba: vec![1, 1, 1, 255, 2, 2, 2, 255] };
        let mut dst = solid(2, 2, 0);
        assert!(blit(&mut dst, &src, rect(1, 0, 1, 1), (0, 1), Blend::Replace));
        assert_eq!(dst.rgba[8..12], [2, 2, 2, 255]);
        assert_eq!(dst.rgba[0..8], [0; 8]);
    }

    #[test]
    fn a_rectangle_outside_either_picture_does_nothing() {
        let src = solid(2, 2, 0xFF);
        let mut dst = solid(2, 2, 0);
        let before = dst.rgba.clone();
        for (from, to) in [
            (rect(1, 0, 2, 1), (0, 0)),
            (rect(0, 0, 2, 2), (1, 0)),
            (rect(0, 0, u32::MAX, 1), (0, 0)),
            (rect(0, 0, 1, 1), (u32::MAX, u32::MAX)),
        ] {
            assert!(!blit(&mut dst, &src, from, to, Blend::Over));
        }
        assert_eq!(dst.rgba, before);
        let short = Image { width: 2, height: 2, rgba: vec![0; 4] };
        assert!(!blit(&mut dst, &short, rect(0, 0, 1, 1), (0, 0), Blend::Over), "pixels that are not there");
    }
}
