//! Everything at once: wide characters, marks, links, protection, selective erase, resizes, pins,
//! the view and the snapshot copy, in random order. No model, only the invariants and a few sanity
//! checks. Pages are tiny and histories small, so every path is taken often.

use super::random::{random_pen, random_region};
use super::{BG, Rng, seeds};
use crate::grid::Grid;
use crate::types::{Color, Line};

fn run_everything(seed: u64, steps: usize) {
    use crate::grid::{Content, ViewRow};
    use crate::types::{Column, Hyperlink, Point, Scroll};
    let mut rng = Rng::new(seed);
    let (mut cols, mut rows) = (rng.range(2, 16), rng.range(1, 9));
    let limit = [0, 4096, 1 << 20][rng.below(3)];
    let mut grid = Grid::with_page_rows(cols, rows, limit, rng.range(1, 7));
    let mut pins = Vec::new();
    let mut view: Vec<ViewRow> = Vec::new();
    let mut resized;
    let links = [
        Hyperlink { id: "a".into(), uri: "https://a".into() },
        Hyperlink { id: "b".into(), uri: "https://b".into() },
    ];
    let marks = ['\u{301}', '\u{200D}', '\u{FE0F}'];
    for step in 0..steps {
        let pen = {
            let mut pen = random_pen(&mut rng);
            if rng.chance(25) {
                pen.link = Some(links[rng.below(2)].clone());
            }
            if rng.chance(25) {
                pen.content = [Content::Prompt, Content::Command, Content::Output][rng.below(3)];
                pen.block = 1 + rng.below(3) as u32;
            }
            if rng.chance(10) {
                pen.style.underline_color = Some(Color::Indexed(rng.below(8) as u8));
            }
            pen
        };
        let (y, x) = (rng.below(rows), rng.below(cols));
        let desc = match rng.below(20) {
            0..=2 => {
                let bytes: Vec<u8> = (0..rng.range(1, cols)).map(|_| b'a' + rng.below(26) as u8).collect();
                grid.write_ascii(y, x, &bytes, &pen);
                "ascii"
            }
            3..=5 => {
                let wide = rng.chance(50);
                let extra: Vec<char> = (0..rng.below(3)).map(|_| marks[rng.below(3)]).collect();
                grid.write_cluster(
                    y,
                    x,
                    if wide { '日' } else { 'e' },
                    &extra,
                    if wide { 2 } else { 1 },
                    &pen,
                );
                "cluster"
            }
            6 => {
                grid.push_mark(y, x, marks[rng.below(3)]);
                "mark"
            }
            7 => {
                grid.set_width(y, x, 1 + rng.below(2) as u8);
                "width"
            }
            8 => {
                grid.write_spacer_head(y, &pen);
                grid.set_wrapped(y, true);
                "spacer head"
            }
            9 => {
                let (a, b) = (rng.below(cols + 1), rng.below(cols + 1));
                grid.erase(y, a.min(b)..a.max(b), bg_random(&mut rng), rng.chance(40));
                "erase"
            }
            10 => {
                grid.erase_rows(y..y + rng.below(3), bg_random(&mut rng), rng.chance(40));
                "erase rows"
            }
            11 | 12 => {
                let (a, b) = (rng.below(cols), rng.below(cols));
                if rng.chance(50) {
                    grid.insert_cells(y, a.min(b), a.max(b), rng.below(cols), bg_random(&mut rng));
                } else {
                    grid.delete_cells(y, a.min(b), a.max(b), rng.below(cols), bg_random(&mut rng));
                }
                "shift cells"
            }
            13 | 14 => {
                let region = random_region(&mut rng, cols, rows);
                let n = rng.below(rows + 1);
                match rng.below(4) {
                    0 => grid.scroll_up(region, n, bg_random(&mut rng)),
                    1 => grid.scroll_down(region, n, bg_random(&mut rng)),
                    2 => grid.insert_lines(
                        rng.range(region.top, region.bottom),
                        region,
                        n,
                        bg_random(&mut rng),
                    ),
                    _ => grid.delete_lines(
                        rng.range(region.top, region.bottom),
                        region,
                        n,
                        bg_random(&mut rng),
                    ),
                }
                "scroll"
            }
            15 => {
                let (new_cols, new_rows) = (rng.range(2, 18), rng.range(1, 10));
                let cursor = Point::new(Line(rng.below(rows) as i32), Column(rng.below(cols)));
                let reflow = rng.chance(70);
                let at = grid.resize(new_cols, new_rows, cursor, reflow);
                assert!((at.line.0 as usize) < new_rows && at.column.0 < new_cols, "cursor {at:?}");
                resized = format!(
                    "resize {cols}x{rows} -> {new_cols}x{new_rows} reflow {reflow} cursor {cursor:?}"
                );
                (cols, rows) = (new_cols, new_rows);
                resized.as_str()
            }
            16 => {
                match rng.below(5) {
                    0 => grid.clear_scrollback(),
                    1 => grid.scroll_view(Scroll::Delta(rng.below(7) as i32 - 3)),
                    2 => grid.scroll_view(Scroll::Top),
                    3 => grid.scroll_screen_into_history(BG),
                    _ => grid.set_scrollback_limit([0, 2048, 1 << 20][rng.below(3)]),
                }
                "view and history"
            }
            17 => {
                match rng.below(5) {
                    3 => {
                        let placement = crate::grid::NewPlacement {
                            image: crate::types::ImageHandle(1 + rng.below(4) as u64),
                            image_id: 0,
                            id: rng.below(3) as u32,
                            cols: rng.range(1, 9),
                            rows: rng.range(1, 7),
                            source: crate::types::SourceRect { x: 0, y: 0, width: 40, height: 40 },
                            offset_x: 0,
                            offset_y: 0,
                            extent: crate::types::Extent::Fill,
                            z: rng.below(3) as i32 - 1,
                        };
                        grid.place_image(Line(rng.below(rows + 3) as i32 - 3), rng.below(cols), placement);
                    }
                    4 if rng.chance(30) => grid.clear_placements(),
                    0 => pins.push(
                        grid.pin(Point::new(Line(rng.below(rows + 4) as i32 - 4), Column(rng.below(cols)))),
                    ),
                    1 if !pins.is_empty() => grid.unpin(pins.swap_remove(rng.below(pins.len()))),
                    _ => {
                        if let Some(&pin) = pins.first() {
                            grid.move_pin(
                                pin,
                                Point::new(Line(rng.below(rows) as i32), Column(rng.below(cols))),
                            );
                        }
                    }
                }
                "pins"
            }
            18 => {
                grid.copy_viewport(&mut view, rng.chance(20));
                let mut placed = Vec::new();
                grid.copy_placements(&mut placed, (8, 16));
                assert!(placed.iter().all(|p| p.column < cols && p.column + p.cols <= cols));
                assert_eq!(view.len(), rows);
                assert!(view.iter().all(|r| r.cells.len() == cols));
                "copy"
            }
            _ => {
                grid.set_wrapped(y, rng.chance(50));
                grid.set_row_block(y, rng.below(3) as u32);
                "wrap"
            }
        };
        if let Err(e) = grid.check_invariants() {
            panic!("{e}: seed {seed} step {step} ({desc}) on {cols}x{rows}");
        }
        assert_eq!((grid.cols(), grid.rows()), (cols, rows));
        for &pin in &pins {
            if let Some(p) = grid.pin_point(pin) {
                assert!(p.line.0 < rows as i32 && p.column.0 < cols, "pin {p:?} seed {seed} step {step}");
            }
        }
    }
}

fn bg_random(rng: &mut Rng) -> Color {
    if rng.chance(60) { BG } else { Color::Indexed(rng.below(4) as u8) }
}

#[test]
fn everything_keeps_the_invariants() {
    for seed in 1..=seeds(300) {
        run_everything(seed * 6151, 150);
    }
}

#[test]
fn long_runs_of_everything_keep_the_invariants() {
    for seed in 1..=seeds(8) {
        run_everything(seed * 1_299_709, 4000);
    }
}
