//! Geometry for the Unicode "Block Elements" (U+2580–U+259F).
//!
//! Fonts draw these glyphs at ascent+descent height, which is shorter than a
//! terminal row whenever the line height is above 1.0. The gaps show up as
//! stripes in block art (the Claude Code mascot, progress bars). Terminals
//! paint them as rectangles that fill the cell exactly instead.

/// A rectangle as fractions of the cell: `[left, top, right, bottom]`.
pub type CellRect = [f32; 4];

/// The filled area of one block-element character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockElement {
    rects: [CellRect; 3],
    len: usize,
    /// Opacity of the foreground over the cell (the shade characters ░▒▓).
    pub alpha: f32,
}

impl BlockElement {
    const fn one(rect: CellRect) -> Self {
        Self { rects: [rect, [0.0; 4], [0.0; 4]], len: 1, alpha: 1.0 }
    }

    pub fn rects(&self) -> &[CellRect] {
        &self.rects[..self.len]
    }
}

const QUADRANTS: [CellRect; 4] = [
    [0.0, 0.0, 0.5, 0.5], // upper left
    [0.5, 0.0, 1.0, 0.5], // upper right
    [0.0, 0.5, 0.5, 1.0], // lower left
    [0.5, 0.5, 1.0, 1.0], // lower right
];

/// Quadrant bitmasks (UL=1, UR=2, LL=4, LR=8) for U+2596..=U+259F.
const QUADRANT_MASKS: [u8; 10] = [
    0b0100, // ▖
    0b1000, // ▗
    0b0001, // ▘
    0b1101, // ▙
    0b1001, // ▚
    0b0111, // ▛
    0b1011, // ▜
    0b0010, // ▝
    0b0110, // ▞
    0b1110, // ▟
];

/// The rectangles to fill for `c`, or `None` if it isn't a block element.
pub fn block_element(c: char) -> Option<BlockElement> {
    let eighths = |n: u32| n as f32 / 8.0;
    Some(match c {
        '\u{2580}' => BlockElement::one([0.0, 0.0, 1.0, 0.5]),
        // ▁ … █: lower one eighth up to the full cell.
        '\u{2581}'..='\u{2588}' => BlockElement::one([0.0, 1.0 - eighths(c as u32 - 0x2580), 1.0, 1.0]),
        // ▉ … ▏: left seven eighths down to one eighth.
        '\u{2589}'..='\u{258F}' => BlockElement::one([0.0, 0.0, eighths(0x2590 - c as u32), 1.0]),
        '\u{2590}' => BlockElement::one([0.5, 0.0, 1.0, 1.0]),
        '\u{2591}'..='\u{2593}' => {
            let mut shade = BlockElement::one([0.0, 0.0, 1.0, 1.0]);
            shade.alpha = (c as u32 - 0x2590) as f32 * 0.25;
            shade
        }
        '\u{2594}' => BlockElement::one([0.0, 0.0, 1.0, 0.125]),
        '\u{2595}' => BlockElement::one([0.875, 0.0, 1.0, 1.0]),
        '\u{2596}'..='\u{259F}' => {
            let mask = QUADRANT_MASKS[(c as u32 - 0x2596) as usize];
            let mut element = BlockElement::one([0.0; 4]);
            element.len = 0;
            for (i, quadrant) in QUADRANTS.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    element.rects[element.len] = *quadrant;
                    element.len += 1;
                }
            }
            element
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::block_element;

    fn area(c: char) -> f32 {
        let element = block_element(c).expect("block element");
        element.rects().iter().map(|[l, t, r, b]| (r - l) * (b - t)).sum()
    }

    #[test]
    fn covers_exactly_the_block_range() {
        for code in 0x2580..=0x259F_u32 {
            let c = char::from_u32(code).expect("valid char");
            assert!(block_element(c).is_some(), "U+{code:04X} missing");
        }
        assert!(block_element('\u{257F}').is_none());
        assert!(block_element('\u{25A0}').is_none());
        assert!(block_element('a').is_none());
    }

    #[test]
    fn halves_and_full_block() {
        assert_eq!(block_element('█').map(|e| e.rects().to_vec()), Some(vec![[0.0, 0.0, 1.0, 1.0]]));
        assert_eq!(block_element('▀').map(|e| e.rects().to_vec()), Some(vec![[0.0, 0.0, 1.0, 0.5]]));
        assert_eq!(block_element('▄').map(|e| e.rects().to_vec()), Some(vec![[0.0, 0.5, 1.0, 1.0]]));
        assert_eq!(block_element('▌').map(|e| e.rects().to_vec()), Some(vec![[0.0, 0.0, 0.5, 1.0]]));
        assert_eq!(block_element('▐').map(|e| e.rects().to_vec()), Some(vec![[0.5, 0.0, 1.0, 1.0]]));
    }

    #[test]
    fn eighths_grow_in_the_right_direction() {
        assert_eq!(area('▁'), 0.125);
        assert_eq!(area('▇'), 0.875);
        assert_eq!(area('▏'), 0.125);
        assert_eq!(area('▉'), 0.875);
        assert_eq!(area('▔'), 0.125);
        assert_eq!(area('▕'), 0.125);
    }

    #[test]
    fn quadrants_match_their_names() {
        assert_eq!(block_element('▘').map(|e| e.rects().to_vec()), Some(vec![[0.0, 0.0, 0.5, 0.5]]));
        assert_eq!(block_element('▗').map(|e| e.rects().to_vec()), Some(vec![[0.5, 0.5, 1.0, 1.0]]));
        // ▛ is everything but the lower right.
        assert_eq!(area('▛'), 0.75);
        assert_eq!(
            block_element('▛').map(|e| e.rects().to_vec()),
            Some(vec![[0.0, 0.0, 0.5, 0.5], [0.5, 0.0, 1.0, 0.5], [0.0, 0.5, 0.5, 1.0]])
        );
        // ▞ is the anti-diagonal: upper right and lower left.
        assert_eq!(
            block_element('▞').map(|e| e.rects().to_vec()),
            Some(vec![[0.5, 0.0, 1.0, 0.5], [0.0, 0.5, 0.5, 1.0]])
        );
        assert_eq!(area('▙'), 0.75);
        assert_eq!(area('▟'), 0.75);
        assert_eq!(area('▚'), 0.5);
        assert_eq!(area('▜'), 0.75);
    }

    #[test]
    fn shades_are_translucent_full_cells() {
        for (c, alpha) in [('░', 0.25), ('▒', 0.5), ('▓', 0.75)] {
            let element = block_element(c).expect("shade");
            assert_eq!(element.alpha, alpha);
            assert_eq!(area(c), 1.0);
        }
        assert_eq!(block_element('█').map(|e| e.alpha), Some(1.0));
    }
}
