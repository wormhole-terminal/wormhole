//! Geometry for the Unicode "Box Drawing" block (U+2500–U+257F).
//!
//! Like the block elements (see `block_elements`), font glyphs for these fall
//! short of the row height whenever the line height is above 1.0, so vertical
//! lines and corners show a gap at every row boundary. Terminals paint them
//! themselves; this module computes the shapes in device pixels for one cell.
//!
//! Every coordinate is snapped to whole device pixels, and only the cell edges
//! (which the caller passes identically for neighbouring cells) decide where
//! lines end, so adjacent cells always join without seams.

/// `[left, top, right, bottom]` in device pixels.
pub type PxRect = [f32; 4];

/// One cell's edges in device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellBox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoxShape {
    Rect(PxRect),
    /// A stroked polyline with butt caps (rounded corners, diagonals).
    Stroke {
        points: Vec<[f32; 2]>,
        width: f32,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Weight {
    None,
    Light,
    Heavy,
    Double,
}

/// Arms of each character in U+2500..=U+257F as `[left, right, up, down]`:
/// 0 none, 1 light, 2 heavy, 3 double. `-` marks the ones drawn another way
/// (dashed, rounded, diagonal).
#[rustfmt::skip]
const ARMS: [&[u8; 4]; 128] = [
    b"1100", b"2200", b"0011", b"0022", b"----", b"----", b"----", b"----",
    b"----", b"----", b"----", b"----", b"0101", b"0201", b"0102", b"0202",
    b"1001", b"2001", b"1002", b"2002", b"0110", b"0210", b"0120", b"0220",
    b"1010", b"2010", b"1020", b"2020", b"0111", b"0211", b"0121", b"0112",
    b"0122", b"0221", b"0212", b"0222", b"1011", b"2011", b"1021", b"1012",
    b"1022", b"2021", b"2012", b"2022", b"1101", b"2101", b"1201", b"2201",
    b"1102", b"2102", b"1202", b"2202", b"1110", b"2110", b"1210", b"2210",
    b"1120", b"2120", b"1220", b"2220", b"1111", b"2111", b"1211", b"2211",
    b"1121", b"1112", b"1122", b"2121", b"1221", b"2112", b"1212", b"2221",
    b"2212", b"2122", b"1222", b"2222", b"----", b"----", b"----", b"----",
    b"3300", b"0033", b"0301", b"0103", b"0303", b"3001", b"1003", b"3003",
    b"0310", b"0130", b"0330", b"3010", b"1030", b"3030", b"0311", b"0133",
    b"0333", b"3011", b"1033", b"3033", b"3301", b"1103", b"3303", b"3310",
    b"1130", b"3330", b"3311", b"1133", b"3333", b"----", b"----", b"----",
    b"----", b"----", b"----", b"----", b"1000", b"0010", b"0100", b"0001",
    b"2000", b"0020", b"0200", b"0002", b"1200", b"0012", b"2100", b"0021",
];

fn arms(code: u32) -> Option<[Weight; 4]> {
    let spec = ARMS[(code - 0x2500) as usize];
    if spec[0] == b'-' {
        return None;
    }
    let weight = |digit: u8| match digit {
        b'1' => Weight::Light,
        b'2' => Weight::Heavy,
        b'3' => Weight::Double,
        _ => Weight::None,
    };
    Some([weight(spec[0]), weight(spec[1]), weight(spec[2]), weight(spec[3])])
}

/// The shapes to fill with the foreground color for `c`, or `None` if it isn't
/// a box-drawing character. `light` is the thickness of a light line in whole
/// device pixels; heavy lines are twice that, double lines two strokes with a
/// light-sized gap.
pub fn box_shapes(c: char, cell: CellBox, light: f32) -> Option<Vec<BoxShape>> {
    let mut shapes = Vec::new();
    box_shapes_into(c, cell, light, &mut shapes).then_some(shapes)
}

/// [`box_shapes`], appended to `out`; returns whether `c` is a box-drawing
/// character. The painter calls this for every such cell it builds, so straight
/// lines and corners allocate nothing once `out` has grown (only the curves and
/// diagonals carry a point list).
pub fn box_shapes_into(c: char, cell: CellBox, light: f32, out: &mut Vec<BoxShape>) -> bool {
    let code = c as u32;
    if !(0x2500..=0x257F).contains(&code) {
        return false;
    }
    let light = light.round().max(1.0);
    let cell = CellBox {
        left: cell.left.round(),
        top: cell.top.round(),
        right: cell.right.round(),
        bottom: cell.bottom.round(),
    };
    match arms(code) {
        Some(arms) => solid(arms, cell, light, out),
        None => match code {
            0x2504..=0x250B | 0x254C..=0x254F => dashed(code, cell, light, out),
            0x256D..=0x2570 => rounded(code, cell, light, out),
            _ => diagonals(code, cell, light, out),
        },
    }
    true
}

/// Rounded centre of a line of thickness `t` around `c`.
fn lo(c: f32, t: f32) -> f32 {
    (c - t / 2.0).round()
}

fn solid(arms: [Weight; 4], cell: CellBox, light: f32, out: &mut Vec<BoxShape>) {
    let [left, right, up, down] = arms;
    let band = |w: Weight| match w {
        Weight::None => 0.0,
        Weight::Light => light,
        Weight::Heavy => light * 2.0,
        Weight::Double => light * 3.0,
    };
    let cx = (cell.left + cell.right) / 2.0;
    let cy = (cell.top + cell.bottom) / 2.0;
    let hband = band(left).max(band(right));
    let vband = band(up).max(band(down));
    // Only all-double glyphs (╔ ╬ …) have strokes that pair up into nested
    // corners; double meeting single lines just crosses or joins them.
    let all_double =
        hband > 0.0 && vband > 0.0 && arms.iter().all(|w| matches!(w, Weight::None | Weight::Double));
    let double = |w: Weight| w == Weight::Double;
    let mut push = |rect: PxRect| out.push(BoxShape::Rect(rect));

    for (dir, w) in [(-1.0_f32, left), (1.0, right)] {
        let own = band(w);
        if own == 0.0 {
            continue;
        }
        // With no vertical line to meet, the two halves just overlap in the middle.
        let perp = if vband > 0.0 { vband } else { own };
        let vlo = lo(cx, perp);
        let plain_end = if dir > 0.0 { vlo } else { vlo + perp };
        let span = |end: f32| if dir > 0.0 { (end, cell.right) } else { (cell.left, end) };
        if w == Weight::Double {
            let ylo = lo(cy, own);
            for (i, side) in [-1.0_f32, 1.0].into_iter().enumerate() {
                let end = if all_double {
                    let turns = (double(up) && side < 0.0) || (double(down) && side > 0.0);
                    match (turns, dir > 0.0) {
                        (true, true) => vlo + 2.0 * light,
                        (true, false) => vlo + light,
                        (false, true) => vlo,
                        (false, false) => vlo + 3.0 * light,
                    }
                } else {
                    plain_end
                };
                let y0 = ylo + 2.0 * light * i as f32;
                let (a, b) = span(end);
                push([a, y0, b, y0 + light]);
            }
        } else {
            let y0 = lo(cy, own);
            let (a, b) = span(plain_end);
            push([a, y0, b, y0 + own]);
        }
    }
    for (dir, w) in [(-1.0_f32, up), (1.0, down)] {
        let own = band(w);
        if own == 0.0 {
            continue;
        }
        let perp = if hband > 0.0 { hband } else { own };
        let hlo = lo(cy, perp);
        let plain_end = if dir > 0.0 { hlo } else { hlo + perp };
        let span = |end: f32| if dir > 0.0 { (end, cell.bottom) } else { (cell.top, end) };
        if w == Weight::Double {
            let xlo = lo(cx, own);
            for (i, side) in [-1.0_f32, 1.0].into_iter().enumerate() {
                let end = if all_double {
                    let turns = (double(left) && side < 0.0) || (double(right) && side > 0.0);
                    match (turns, dir > 0.0) {
                        (true, true) => hlo + 2.0 * light,
                        (true, false) => hlo + light,
                        (false, true) => hlo,
                        (false, false) => hlo + 3.0 * light,
                    }
                } else {
                    plain_end
                };
                let x0 = xlo + 2.0 * light * i as f32;
                let (a, b) = span(end);
                push([x0, a, x0 + light, b]);
            }
        } else {
            let x0 = lo(cx, own);
            let (a, b) = span(plain_end);
            push([x0, a, x0 + own, b]);
        }
    }
}

/// ┄ ┅ ┆ ┇ ┈ ┉ ┊ ┋ ╌ ╍ ╎ ╏: evenly spaced dashes with half a gap at each cell edge,
/// so the pattern continues unbroken into the next cell.
fn dashed(code: u32, cell: CellBox, light: f32, out: &mut Vec<BoxShape>) {
    let (horizontal, heavy, count) = match code {
        0x2504 => (true, false, 3),
        0x2505 => (true, true, 3),
        0x2506 => (false, false, 3),
        0x2507 => (false, true, 3),
        0x2508 => (true, false, 4),
        0x2509 => (true, true, 4),
        0x250A => (false, false, 4),
        0x250B => (false, true, 4),
        0x254C => (true, false, 2),
        0x254D => (true, true, 2),
        0x254E => (false, false, 2),
        _ => (false, true, 2),
    };
    let thickness = if heavy { light * 2.0 } else { light };
    let (start, end) = if horizontal { (cell.left, cell.right) } else { (cell.top, cell.bottom) };
    let pitch = (end - start) / count as f32;
    let across = if horizontal {
        lo((cell.top + cell.bottom) / 2.0, thickness)
    } else {
        lo((cell.left + cell.right) / 2.0, thickness)
    };
    out.extend((0..count).map(|i| {
        let a = (start + pitch * (i as f32 + 0.25)).round();
        let b = (start + pitch * (i as f32 + 0.75)).round().max(a + 1.0);
        BoxShape::Rect(if horizontal {
            [a, across, b, across + thickness]
        } else {
            [across, a, across + thickness, b]
        })
    }));
}

/// ╭ ╮ ╯ ╰: straight stubs to the cell edges joined by a quarter-circle stroke.
fn rounded(code: u32, cell: CellBox, light: f32, out: &mut Vec<BoxShape>) {
    // Which side of the cell the horizontal and vertical arms leave from.
    let (dx, dy) = match code {
        0x256D => (1.0_f32, 1.0_f32),
        0x256E => (-1.0, 1.0),
        0x256F => (-1.0, -1.0),
        _ => (1.0, -1.0),
    };
    let xlo = lo((cell.left + cell.right) / 2.0, light);
    let ylo = lo((cell.top + cell.bottom) / 2.0, light);
    // Centre lines of the two strokes.
    let (px, py) = (xlo + light / 2.0, ylo + light / 2.0);
    let room_x = if dx > 0.0 { cell.right - px } else { px - cell.left };
    let room_y = if dy > 0.0 { cell.bottom - py } else { py - cell.top };
    let r = room_x.min(room_y).floor().max(1.0);

    // The stubs reach one pixel into the curve so no hairline shows at the join.
    let vertical = if dy > 0.0 {
        [xlo, py + r - 1.0, xlo + light, cell.bottom]
    } else {
        [xlo, cell.top, xlo + light, py - r + 1.0]
    };
    let horizontal = if dx > 0.0 {
        [px + r - 1.0, ylo, cell.right, ylo + light]
    } else {
        [cell.left, ylo, px - r + 1.0, ylo + light]
    };

    const STEPS: usize = 8;
    let (cx, cy) = (px + dx * r, py + dy * r);
    let points = (0..=STEPS)
        .map(|k| {
            let phi = k as f32 / STEPS as f32 * std::f32::consts::FRAC_PI_2;
            [cx - dx * r * phi.cos(), cy - dy * r * phi.sin()]
        })
        .collect();
    out.extend([
        BoxShape::Rect(vertical),
        BoxShape::Rect(horizontal),
        BoxShape::Stroke { points, width: light },
    ]);
}

/// ╱ ╲ ╳: corner to corner.
fn diagonals(code: u32, cell: CellBox, light: f32, out: &mut Vec<BoxShape>) {
    let rising =
        BoxShape::Stroke { points: vec![[cell.right, cell.top], [cell.left, cell.bottom]], width: light };
    let falling =
        BoxShape::Stroke { points: vec![[cell.left, cell.top], [cell.right, cell.bottom]], width: light };
    match code {
        0x2571 => out.push(rising),
        0x2572 => out.push(falling),
        _ => out.extend([rising, falling]),
    }
}

#[cfg(test)]
mod tests {
    use super::{BoxShape, CellBox, PxRect, box_shapes};

    /// Odd width on purpose: the centre falls between pixels.
    const CELL: CellBox = CellBox { left: 34.0, top: 72.0, right: 51.0, bottom: 108.0 };
    const LIGHT: f32 = 2.0;

    fn rects(c: char) -> Vec<PxRect> {
        box_shapes(c, CELL, LIGHT)
            .unwrap_or_else(|| panic!("{c} is a box-drawing character"))
            .into_iter()
            .filter_map(|shape| match shape {
                BoxShape::Rect(rect) => Some(rect),
                BoxShape::Stroke { .. } => None,
            })
            .collect()
    }

    fn touches(a: &PxRect, b: &PxRect) -> bool {
        a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
    }

    fn is_connected(rects: &[PxRect]) -> bool {
        let mut reached = vec![false; rects.len()];
        let mut stack = vec![0];
        reached[0] = true;
        while let Some(i) = stack.pop() {
            for j in 0..rects.len() {
                if !reached[j] && touches(&rects[i], &rects[j]) {
                    reached[j] = true;
                    stack.push(j);
                }
            }
        }
        reached.into_iter().all(|r| r)
    }

    fn all_box_chars() -> impl Iterator<Item = char> {
        (0x2500..=0x257F_u32).map(|code| char::from_u32(code).expect("valid char"))
    }

    #[test]
    fn only_handles_the_box_drawing_range() {
        assert!(box_shapes('\u{24FF}', CELL, LIGHT).is_none());
        assert!(box_shapes('\u{2580}', CELL, LIGHT).is_none());
        assert!(box_shapes('-', CELL, LIGHT).is_none());
        for c in all_box_chars() {
            let shapes = box_shapes(c, CELL, LIGHT).unwrap_or_default();
            assert!(!shapes.is_empty(), "U+{:04X} draws nothing", c as u32);
        }
    }

    #[test]
    fn everything_stays_inside_the_cell() {
        for c in all_box_chars() {
            for rect in rects(c) {
                assert!(
                    rect[0] >= CELL.left
                        && rect[1] >= CELL.top
                        && rect[2] <= CELL.right
                        && rect[3] <= CELL.bottom,
                    "U+{:04X} spills out of the cell: {rect:?}",
                    c as u32
                );
                assert!(rect[2] > rect[0] && rect[3] > rect[1], "U+{:04X} has an empty rect", c as u32);
            }
        }
    }

    #[test]
    fn straight_lines_span_the_whole_cell() {
        // Light ─ and │ touch both edges and are one light line thick.
        let h = rects('─');
        assert_eq!(h.len(), 2);
        assert_eq!(h.iter().map(|r| r[0]).fold(f32::MAX, f32::min), CELL.left);
        assert_eq!(h.iter().map(|r| r[2]).fold(f32::MIN, f32::max), CELL.right);
        assert!(h.iter().all(|r| r[3] - r[1] == LIGHT));
        let v = rects('│');
        assert_eq!(v.iter().map(|r| r[1]).fold(f32::MAX, f32::min), CELL.top);
        assert_eq!(v.iter().map(|r| r[3]).fold(f32::MIN, f32::max), CELL.bottom);
        assert!(v.iter().all(|r| r[2] - r[0] == LIGHT));
    }

    #[test]
    fn vertical_lines_do_not_depend_on_the_row() {
        // The x position comes from the column only, so stacked cells line up.
        let below = CellBox { top: CELL.bottom, bottom: CELL.bottom + 36.0, ..CELL };
        let here = rects('│');
        let there: Vec<PxRect> = box_shapes('│', below, LIGHT)
            .expect("box")
            .into_iter()
            .filter_map(|s| if let BoxShape::Rect(r) = s { Some(r) } else { None })
            .collect();
        assert_eq!(here[0][0], there[0][0]);
        assert_eq!(here[0][2], there[0][2]);
        // …and one cell ends exactly where the next begins.
        assert_eq!(here.iter().map(|r| r[3]).fold(f32::MIN, f32::max), there[0][1]);
    }

    #[test]
    fn heavy_is_twice_as_thick() {
        let light = rects('─')[0];
        let heavy = rects('━')[0];
        assert_eq!(heavy[3] - heavy[1], 2.0 * (light[3] - light[1]));
    }

    #[test]
    fn corners_and_junctions_are_filled() {
        // Every light/heavy glyph is one connected shape, so joints leave no notch.
        for c in all_box_chars() {
            let code = c as u32;
            let double = (0x2550..=0x256C).contains(&code);
            let odd = matches!(code, 0x2504..=0x250B | 0x254C..=0x254F | 0x256D..=0x2573);
            if double || odd {
                continue;
            }
            assert!(is_connected(&rects(c)), "U+{:04X} ({c}) has a gap: {:?}", code, rects(c));
        }
    }

    #[test]
    fn each_arm_reaches_its_cell_edge() {
        let reaches = |c: char, edge: usize, value: f32| rects(c).iter().any(|r| r[edge] == value);
        // ┌ opens right and down.
        assert!(reaches('┌', 2, CELL.right) && reaches('┌', 3, CELL.bottom));
        assert!(!reaches('┌', 0, CELL.left) && !reaches('┌', 1, CELL.top));
        // ┴ opens left, right and up.
        assert!(reaches('┴', 0, CELL.left) && reaches('┴', 2, CELL.right) && reaches('┴', 1, CELL.top));
        assert!(!reaches('┴', 3, CELL.bottom));
        // Half lines only go one way.
        assert!(reaches('╵', 1, CELL.top) && !reaches('╵', 3, CELL.bottom));
        assert!(reaches('╴', 0, CELL.left) && !reaches('╴', 2, CELL.right));
    }

    #[test]
    fn mixed_weights_use_the_matching_thickness() {
        // ┍: light down, heavy right.
        let shapes = rects('┍');
        let right = shapes.iter().find(|r| r[2] == CELL.right).expect("right arm");
        let down = shapes.iter().find(|r| r[3] == CELL.bottom).expect("down arm");
        assert_eq!(right[3] - right[1], 2.0 * LIGHT);
        assert_eq!(down[2] - down[0], LIGHT);
        // ╿: heavy up, light down.
        let shapes = rects('╿');
        let up = shapes.iter().find(|r| r[1] == CELL.top).expect("up arm");
        let down = shapes.iter().find(|r| r[3] == CELL.bottom).expect("down arm");
        assert_eq!(up[2] - up[0], 2.0 * LIGHT);
        assert_eq!(down[2] - down[0], LIGHT);
    }

    #[test]
    fn double_lines_are_two_strokes_with_a_light_gap() {
        let h = rects('═');
        assert_eq!(h.len(), 4);
        let mut tops: Vec<f32> = h.iter().map(|r| r[1]).collect();
        tops.sort_by(f32::total_cmp);
        tops.dedup();
        assert_eq!(tops.len(), 2);
        assert_eq!(tops[1] - tops[0], 2.0 * LIGHT); // stroke + gap
        assert!(h.iter().all(|r| r[3] - r[1] == LIGHT));

        let v = rects('║');
        let mut lefts: Vec<f32> = v.iter().map(|r| r[0]).collect();
        lefts.sort_by(f32::total_cmp);
        lefts.dedup();
        assert_eq!(lefts.len(), 2);
        assert_eq!(lefts[1] - lefts[0], 2.0 * LIGHT);
    }

    #[test]
    fn double_corners_nest() {
        // ╔: outer strokes meet at the top-left, the inner pair one step in.
        let shapes = rects('╔');
        assert_eq!(shapes.len(), 4);
        let outer_h = shapes.iter().filter(|r| r[2] == CELL.right).map(|r| r[0]).fold(f32::MAX, f32::min);
        let inner_h = shapes.iter().filter(|r| r[2] == CELL.right).map(|r| r[0]).fold(f32::MIN, f32::max);
        assert_eq!(inner_h - outer_h, 2.0 * LIGHT);
        let outer_v = shapes.iter().filter(|r| r[3] == CELL.bottom).map(|r| r[1]).fold(f32::MAX, f32::min);
        let inner_v = shapes.iter().filter(|r| r[3] == CELL.bottom).map(|r| r[1]).fold(f32::MIN, f32::max);
        assert_eq!(inner_v - outer_v, 2.0 * LIGHT);
        // Each horizontal stroke is joined by a vertical one.
        for h in shapes.iter().filter(|r| r[2] == CELL.right) {
            assert!(shapes.iter().filter(|r| r[3] == CELL.bottom).any(|v| touches(h, v)));
        }
    }

    #[test]
    fn double_t_junction_keeps_one_line_straight() {
        // ╠: the left stroke runs the full height, the right one turns into the arm.
        let shapes = rects('╠');
        assert_eq!(shapes.len(), 6);
        let left_edge = shapes.iter().map(|r| r[0]).fold(f32::MAX, f32::min);
        let straight: Vec<&PxRect> = shapes.iter().filter(|r| r[0] == left_edge).collect();
        assert!(straight.iter().any(|r| r[1] == CELL.top));
        assert!(straight.iter().any(|r| r[3] == CELL.bottom));
        assert!(is_connected(&straight.iter().map(|r| **r).collect::<Vec<_>>()));
    }

    #[test]
    fn single_line_crossing_double_is_continuous() {
        // ╪: a light vertical through two horizontal strokes.
        let shapes = rects('╪');
        let vertical: Vec<PxRect> =
            shapes.iter().copied().filter(|r| r[2] - r[0] == LIGHT && r[3] - r[1] > LIGHT).collect();
        assert!(is_connected(&vertical));
        assert_eq!(vertical.iter().map(|r| r[1]).fold(f32::MAX, f32::min), CELL.top);
        assert_eq!(vertical.iter().map(|r| r[3]).fold(f32::MIN, f32::max), CELL.bottom);
    }

    #[test]
    fn dashes_repeat_and_leave_gaps() {
        for (c, count) in [('┄', 3), ('┅', 3), ('┈', 4), ('┉', 4), ('╌', 2), ('╍', 2)] {
            let dashes = rects(c);
            assert_eq!(dashes.len(), count, "{c}");
            for pair in dashes.windows(2) {
                assert!(pair[0][2] < pair[1][0], "{c} dashes touch");
            }
            // Half a gap at each end keeps the rhythm across cells.
            assert!(dashes[0][0] > CELL.left && dashes[count - 1][2] < CELL.right);
        }
        for (c, count) in [('┆', 3), ('┊', 4), ('╎', 2), ('╏', 2), ('┇', 3), ('┋', 4)] {
            let dashes = rects(c);
            assert_eq!(dashes.len(), count, "{c}");
            for pair in dashes.windows(2) {
                assert!(pair[0][3] < pair[1][1], "{c} dashes touch");
            }
        }
    }

    #[test]
    fn rounded_corners_curve_between_their_arms() {
        for (c, right, down) in
            [('╭', true, true), ('╮', false, true), ('╯', false, false), ('╰', true, false)]
        {
            let shapes = box_shapes(c, CELL, LIGHT).expect("arc");
            let stroke = shapes.iter().find_map(|s| match s {
                BoxShape::Stroke { points, width } => Some((points, *width)),
                BoxShape::Rect(_) => None,
            });
            let (points, width) = stroke.expect("curve");
            assert_eq!(width, LIGHT);
            let (first, last) = (points[0], points[points.len() - 1]);
            // Starts on the vertical line, ends on the horizontal one.
            let cx = (CELL.left + CELL.right) / 2.0;
            let cy = (CELL.top + CELL.bottom) / 2.0;
            assert!((first[0] - cx).abs() <= 1.0, "{c} starts off the vertical line");
            assert!((last[1] - cy).abs() <= 1.0, "{c} ends off the horizontal line");
            // The curve bulges toward the corner of the cell it turns away from.
            assert_eq!(first[1] > cy, down, "{c}");
            assert_eq!(last[0] > cx, right, "{c}");
            // Stubs reach the edges the arms point at.
            let rects = rects(c);
            assert!(rects.iter().any(|r| if down { r[3] == CELL.bottom } else { r[1] == CELL.top }));
            assert!(rects.iter().any(|r| if right { r[2] == CELL.right } else { r[0] == CELL.left }));
        }
    }

    #[test]
    fn diagonals_run_corner_to_corner() {
        let count = |c: char| box_shapes(c, CELL, LIGHT).map(|s| s.len());
        assert_eq!(count('╱'), Some(1));
        assert_eq!(count('╲'), Some(1));
        assert_eq!(count('╳'), Some(2));
        let Some(BoxShape::Stroke { points, .. }) =
            box_shapes('╱', CELL, LIGHT).and_then(|s| s.into_iter().next())
        else {
            panic!("╱ is a stroke");
        };
        assert_eq!(points, vec![[CELL.right, CELL.top], [CELL.left, CELL.bottom]]);
    }

    /// Every character's exact output, folded into one number (pins the geometry
    /// across refactors of how it is produced).
    fn fingerprint(light: f32) -> (usize, u64) {
        let mut shapes = 0;
        let mut acc = 0u64;
        for c in all_box_chars() {
            for shape in box_shapes(c, CELL, light).expect("box") {
                shapes += 1;
                let values: Vec<f32> = match shape {
                    BoxShape::Rect(rect) => rect.to_vec(),
                    BoxShape::Stroke { points, width } => {
                        points.iter().flatten().copied().chain([width, -1.0]).collect()
                    }
                };
                for v in values {
                    acc = (acc.rotate_left(7) ^ u64::from(v.to_bits())).wrapping_mul(0x100_0000_01b3);
                }
            }
        }
        (shapes, acc)
    }

    #[test]
    fn geometry_is_unchanged() {
        assert_eq!(fingerprint(LIGHT), (396, 7172609075123315828));
        assert_eq!(fingerprint(1.0), (396, 5087087234883980434));
    }

    #[test]
    fn thickness_never_drops_below_one_pixel() {
        let shapes = box_shapes('─', CELL, 0.2).expect("box");
        let BoxShape::Rect(rect) = &shapes[0] else { panic!("rect") };
        assert_eq!(rect[3] - rect[1], 1.0);
    }
}
