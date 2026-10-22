//! The word cache's tests: what it shapes, when, and the glyphs it hands back.

use gpui_kit::{FontId, GlyphId, point, px};
use wormhole_core::terminal::types::{Cell, Color, Colors, Flags, NamedColor, RowExtras};
use wormhole_core::theme;

use std::time::Instant;

use super::words::{MAX_SHAPED_CELLS, Shaped, WordCache};
use crate::palette::Palette;
use crate::terminal_view::render::rows::{Marks, Segment, build_row};

fn segments(text: &str, red: &[usize], bold: &[usize]) -> Vec<Segment> {
    let cells: Vec<Cell> = text
        .chars()
        .enumerate()
        .map(|(col, c)| {
            let mut cell = Cell { c, ..Cell::default() };
            if red.contains(&col) {
                cell.fg = Color::Named(NamedColor::Red);
            }
            if bold.contains(&col) {
                cell.flags = Flags::BOLD;
            }
            cell
        })
        .collect();
    let palette = Palette::from_theme(&theme::builtin()[0]);
    build_row(&cells, &RowExtras::default(), &palette, &Colors::default(), &Marks::default()).segments
}

/// One glyph per character, one cell apart.
fn fake_shape(segment: &Segment) -> Vec<Shaped> {
    segment
        .text
        .char_indices()
        .enumerate()
        .map(|(k, (index, c))| Shaped {
            position: point(px(8.0 * k as f32), px(0.0)),
            index,
            font_id: FontId(0),
            id: GlyphId(c as u32),
            emoji: false,
        })
        .collect()
}

#[test]
fn a_word_is_shaped_once_whatever_its_color_and_column() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let shaped = std::cell::Cell::new(0);
    let glyphs = |cache: &mut WordCache<u8>, segment: &Segment| {
        cache.glyphs(segment, px(8.0), |segment| {
            shaped.set(shaped.get() + 1);
            fake_shape(segment)
        })
    };
    let plain = segments("ab ab", &[], &[]);
    let red = segments("  ab", &[2, 3], &[]);
    let first = glyphs(&mut cache, &plain[0]);
    glyphs(&mut cache, &plain[1]);
    let colored = glyphs(&mut cache, &red[0]);
    assert_eq!(shaped.get(), 1, "the same word in another place and color isn't shaped again");
    let palette = Palette::from_theme(&theme::builtin()[0]);
    assert_eq!(first[1].color, palette.foreground);
    assert_eq!(colored[1].color, palette.ansi[1]);
    assert_eq!(colored[1].position, first[1].position);

    glyphs(&mut cache, &segments("ab", &[], &[1])[0]);
    assert_eq!(shaped.get(), 2, "a bold letter is another font");
    cache.sync(1);
    glyphs(&mut cache, &plain[0]);
    assert_eq!(shaped.get(), 3, "another font or size shapes again");
}

#[test]
fn past_the_budget_a_new_word_is_drawn_character_by_character_until_a_frame_shapes_it() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let asked = std::cell::RefCell::new(Vec::new());
    let segment = &segments("=>=", &[1], &[])[0];
    let glyphs = |cache: &mut WordCache<u8>| {
        cache.glyphs(segment, px(8.0), |segment| {
            asked.borrow_mut().push(segment.text.clone());
            fake_shape(segment)
        })
    };
    // Spent before it started.
    cache.start_frame(Some(Instant::now()));
    let by_char = glyphs(&mut cache);
    assert!(cache.take_deferred(), "the row has to be built again");
    assert!(!cache.take_deferred(), "asked once");
    // The next frame has time: the word is shaped whole, in the same cells and colors.
    cache.start_frame(None);
    let whole = glyphs(&mut cache);
    assert!(!cache.take_deferred());
    assert_eq!(*asked.borrow(), ["=", ">", "=>="], "its characters once each, then the word");
    assert_eq!(by_char, whole);
    // Seen words don't wait, whatever is left of the budget; characters already shaped aren't
    // shaped again.
    cache.start_frame(Some(Instant::now()));
    glyphs(&mut cache);
    assert!(!cache.take_deferred());
    assert_eq!(asked.borrow().len(), 3);
}

#[test]
fn a_word_longer_than_max_shaped_cells_is_shaped_in_parts_of_at_most_that_many_cells() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let text: String = (0..MAX_SHAPED_CELLS * 2 + 5).map(|k| char::from(b'!' + (k % 90) as u8)).collect();
    let (asked, glyphs) = shaped_row(&mut cache, &row(&text), &RowExtras::default());
    let lengths: Vec<usize> = asked.iter().map(|part| part.chars().count()).collect();
    assert_eq!(lengths, [MAX_SHAPED_CELLS, MAX_SHAPED_CELLS, 5]);
    assert_eq!(asked.concat(), text);
    // Each glyph still in its own cell, in order.
    let at: Vec<(f32, u32)> = glyphs.iter().map(|g| (g.0, g.1)).collect();
    let want: Vec<(f32, u32)> =
        text.chars().enumerate().map(|(k, c)| (8.0 * k as f32, u32::from(c))).collect();
    assert_eq!(at, want);
    // The parts are cached like words.
    let (asked, _) = shaped_row(&mut cache, &row(&text), &RowExtras::default());
    assert!(asked.is_empty(), "{asked:?}");
    // Exactly that long: one piece.
    let exact = "=".repeat(MAX_SHAPED_CELLS);
    let (asked, _) = shaped_row(&mut cache, &row(&exact), &RowExtras::default());
    assert_eq!(asked, [exact]);
}

#[test]
fn past_the_budget_a_long_word_uses_the_parts_it_has_and_waits_for_the_rest() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let first: String = "=".repeat(MAX_SHAPED_CELLS);
    shaped_row(&mut cache, &row(&format!("{first}ab")), &RowExtras::default());
    cache.start_frame(Some(Instant::now()));
    // The first part is known; the new rest is drawn character by character for now.
    let (asked, _) = shaped_row(&mut cache, &row(&format!("{first}xy")), &RowExtras::default());
    assert_eq!(asked, ["x", "y"]);
    assert!(cache.take_deferred());
}

#[test]
fn a_word_with_a_mark_is_shaped_whatever_the_budget() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let mut extras = RowExtras::default();
    extras.push_zerowidth(0, '\u{0301}');
    cache.start_frame(Some(Instant::now()));
    // Drawn by character the mark would lose its letter.
    let (asked, _) = shaped_row(&mut cache, &row("ab"), &extras);
    assert_eq!(asked, ["a\u{0301}b"]);
    assert!(!cache.take_deferred());
}

#[test]
fn each_glyph_takes_the_color_of_its_run() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let segment = &segments("abc", &[1], &[])[0];
    let colors: Vec<_> = cache.glyphs(segment, px(8.0), fake_shape).into_iter().map(|g| g.color).collect();
    let palette = Palette::from_theme(&theme::builtin()[0]);
    assert_eq!(colors, [palette.foreground, palette.ansi[1], palette.foreground]);
}

/// The words `shape` was asked for, and the glyphs, for a row of cells.
fn shaped_row(
    cache: &mut WordCache<u8>,
    cells: &[Cell],
    extras: &RowExtras,
) -> (Vec<String>, Vec<(f32, u32, gpui_kit::Hsla)>) {
    let palette = Palette::from_theme(&theme::builtin()[0]);
    let mut asked = Vec::new();
    let mut glyphs = Vec::new();
    for segment in build_row(cells, extras, &palette, &Colors::default(), &Marks::default()).segments {
        let col = segment.col as f32 * 8.0;
        let found = cache.glyphs(&segment, px(8.0), |segment| {
            asked.push(segment.text.clone());
            fake_shape(segment)
        });
        glyphs.extend(found.into_iter().map(|g| (col + f32::from(g.position.x), g.id.0, g.color)));
    }
    (asked, glyphs)
}

fn row(text: &str) -> Vec<Cell> {
    let mut cells = Vec::new();
    for c in text.chars() {
        if matches!(c, '日' | '本') {
            cells.push(Cell { c, flags: Flags::WIDE_CHAR, ..Cell::default() });
            cells.push(Cell { c: ' ', flags: Flags::WIDE_CHAR_SPACER, ..Cell::default() });
        } else {
            cells.push(Cell { c, ..Cell::default() });
        }
    }
    cells
}

#[test]
fn characters_that_stand_alone_are_shaped_once_each_whatever_word_they_are_in() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let (asked, glyphs) = shaped_row(&mut cache, &row("aﾊﾞxyｱｲ"), &RowExtras::default());
    // ﾞ (a voiced mark) isn't a letter that stands alone: it is shaped with what follows it.
    assert_eq!(asked, ["a", "ﾊ", "ﾞxy", "ｱ", "ｲ"]);
    let at: Vec<(f32, u32)> = glyphs.iter().map(|g| (g.0, g.1)).collect();
    let ids: Vec<u32> = "aﾊﾞxyｱｲ".chars().map(u32::from).collect();
    assert_eq!(at, ids.iter().enumerate().map(|(k, id)| (8.0 * k as f32, *id)).collect::<Vec<_>>());

    // The same characters in new words: nothing new to shape.
    let (asked, _) = shaped_row(&mut cache, &row("ｲｱﾊ ｱa"), &RowExtras::default());
    assert!(asked.is_empty(), "{asked:?}");
}

#[test]
fn wide_characters_stand_alone_two_cells_apart() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let (asked, glyphs) = shaped_row(&mut cache, &row("日本日"), &RowExtras::default());
    assert_eq!(asked, ["日", "本"]);
    let at: Vec<f32> = glyphs.iter().map(|g| g.0).collect();
    assert_eq!(at, [0.0, 16.0, 32.0]);
}

#[test]
fn a_character_with_a_combining_mark_is_shaped_with_its_word() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let cells = row("ｱｲ");
    let mut extras = RowExtras::default();
    extras.push_zerowidth(1, '\u{0301}');
    let (asked, _) = shaped_row(&mut cache, &cells, &extras);
    assert_eq!(asked, ["ｱｲ\u{0301}"]);
}

#[test]
fn every_character_keeps_its_own_color_and_font() {
    let mut cache = WordCache::default();
    cache.sync(0u8);
    let mut cells = row("ｱｱxｱ");
    cells[1].fg = Color::Named(NamedColor::Red);
    cells[3].flags = Flags::BOLD;
    let (asked, glyphs) = shaped_row(&mut cache, &cells, &RowExtras::default());
    assert_eq!(asked, ["ｱ", "x", "ｱ"], "a bold ｱ is another font");
    let palette = Palette::from_theme(&theme::builtin()[0]);
    let colors: Vec<_> = glyphs.iter().map(|g| g.2).collect();
    assert_eq!(colors[0], palette.foreground);
    assert_eq!(colors[1], palette.ansi[1]);
    assert_eq!(colors[2], palette.foreground);
}
