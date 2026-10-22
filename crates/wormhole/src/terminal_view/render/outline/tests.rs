use std::collections::HashMap;

use gpui_kit::{FontId, point, px};

use super::{MAX_SPRITES, Seg, SpriteKey, Stretch, build, quantize, sprite_with};
use crate::terminal_view::render::bench::allocations;

fn key(glyph: u32, sub: u8) -> SpriteKey {
    SpriteKey {
        font: FontId(1),
        glyph,
        size: 14.0f32.to_bits(),
        scale: 2.0f32.to_bits(),
        stretch: Stretch::WIDE,
        sub,
    }
}

/// A box from (2, -10) to (9, 0): a glyph's stem, say.
fn stem() -> Vec<Seg> {
    vec![Seg::Move(2.0, -10.0), Seg::Line(9.0, -10.0), Seg::Line(9.0, 0.0), Seg::Line(2.0, 0.0), Seg::Close]
}

#[test]
fn a_sprite_is_an_svg_as_big_as_the_outline_with_a_pixel_of_room() {
    let sprite = build(&stem(), &key(7, 0)).expect("a box has an outline");
    assert_eq!((sprite.left, sprite.top, sprite.width, sprite.height), (1, -11, 9, 12));
    let svg = std::str::from_utf8(sprite.svg()).expect("text");
    assert!(svg.contains("width=\"9\" height=\"12\""), "{svg}");
    // The path starts where the box does, moved so the sprite's top left is its origin.
    assert!(svg.contains("M 1.00 1.00L 8.00 1.00L 8.00 11.00L 1.00 11.00Z"), "{svg}");
}

#[test]
fn curves_keep_their_control_points() {
    let segs = [
        Seg::Move(0.0, 0.0),
        Seg::Quad([4.0, -6.0], [8.0, 0.0]),
        Seg::Cubic([8.0, 2.0], [4.0, 3.0], [0.0, 0.0]),
        Seg::Close,
    ];
    let sprite = build(&segs, &key(8, 0)).expect("outline");
    let svg = std::str::from_utf8(sprite.svg()).expect("text");
    assert!(
        svg.contains("Q 5.00 1.00 9.00 7.00") && svg.contains("C 9.00 9.00 5.00 10.00 1.00 7.00"),
        "{svg}"
    );
}

#[test]
fn a_blank_or_absurd_outline_makes_no_sprite() {
    assert!(build(&[], &key(1, 0)).is_none(), "a space has no outline");
    assert!(
        build(&[Seg::Move(0.0, 0.0), Seg::Line(9000.0, 0.0)], &key(2, 0)).is_none(),
        "bigger than a window"
    );
    assert!(build(&[Seg::Move(f32::NAN, 0.0)], &key(3, 0)).is_none());
}

#[test]
fn a_sprites_path_names_everything_it_is_a_drawing_of() {
    let paths: Vec<_> = [
        key(7, 0),
        key(8, 0),
        key(7, 1),
        SpriteKey { size: 15.0f32.to_bits(), ..key(7, 0) },
        SpriteKey { scale: 1.0f32.to_bits(), ..key(7, 0) },
        SpriteKey { font: FontId(2), ..key(7, 0) },
        SpriteKey { stretch: Stretch { x: 2, y: 2 }, ..key(7, 0) },
    ]
    .iter()
    .map(|k| build(&stem(), k).expect("outline").path().to_string())
    .collect();
    let mut unique = paths.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), paths.len(), "{paths:?}");
}

#[test]
fn a_glyph_lands_on_the_device_pixel_and_quarter_it_is_at() {
    // 2x: 10.3 pt is 20.6 px, which GPUI's glyphs keep as 20 and a half (round to the quarter, 20.5).
    assert_eq!(quantize(point(px(10.3), px(5.2)), 2.0), (20, 10, 2));
    assert_eq!(quantize(point(px(10.0), px(5.0)), 2.0), (20, 10, 0));
    assert_eq!(quantize(point(px(10.4), px(5.0)), 2.0), (20, 10, 3));
}

#[test]
fn sprites_are_built_once_and_a_steady_frame_allocates_nothing() {
    let mut sprites = HashMap::new();
    let keys: Vec<_> = (0..40).map(|glyph| key(glyph, (glyph % 4) as u8)).collect();
    for k in &keys {
        sprite_with(&mut sprites, *k, || build(&stem(), k)).expect("built");
    }
    let before = allocations();
    for _ in 0..100 {
        for k in &keys {
            let found = sprite_with(&mut sprites, *k, || panic!("built again"));
            std::hint::black_box(found);
        }
    }
    assert_eq!(allocations() - before, 0, "looking a built sprite up allocates");
}

#[test]
fn a_blank_glyph_is_remembered_too() {
    let mut sprites = HashMap::new();
    let mut builds = 0;
    for _ in 0..5 {
        let found = sprite_with(&mut sprites, key(1, 0), || {
            builds += 1;
            None
        });
        assert!(found.is_none());
    }
    assert_eq!(builds, 1);
}

#[test]
fn the_cache_starts_over_at_its_cap() {
    let mut sprites = HashMap::new();
    for glyph in 0..MAX_SPRITES as u32 + 10 {
        sprite_with(&mut sprites, key(glyph, 0), || None);
    }
    assert!(sprites.len() <= MAX_SPRITES);
}
