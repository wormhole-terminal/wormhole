//! The engines the tools run against, one file each.
//!
//! To add an engine: write its adapter here, implementing [`Engine`], and add it to
//! [`for_each_engine`], [`visit_engine`] and [`ENGINE_NAMES`]. Nothing else changes: the
//! conformance suite, vtbench and the differential tests pick it up. (`tools/vt-compare` adds
//! other terminals' engines on top of this one, in its own workspace.)

mod wormhole;

pub use wormhole::{WormholeEngine, host_terminal};

use crate::engine::Engine;

/// Code that runs for an engine chosen at run time. Rust has no generic closures, so the code
/// that needs an engine type implements this and receives each engine as a type parameter.
pub trait EngineVisitor {
    fn visit<E: Engine>(&mut self);
}

/// The names of every engine, as `known-fail:` lines and `vtbench --engine` spell them.
pub const ENGINE_NAMES: &[&str] = &[WormholeEngine::NAME];

/// The names a `known-fail:` line may also give: engines that only `tools/vt-compare` can run,
/// whose failures the suite records.
pub const OTHER_ENGINE_NAMES: &[&str] = &["alacritty", "ghostty", "vt100", "wezterm"];

/// Run `visitor` once for every engine.
pub fn for_each_engine(visitor: &mut impl EngineVisitor) {
    visitor.visit::<WormholeEngine>();
}

/// Run `visitor` for the engine called `name`; false when there is none.
pub fn visit_engine(name: &str, visitor: &mut impl EngineVisitor) -> bool {
    match name {
        WormholeEngine::NAME => visitor.visit::<WormholeEngine>(),
        _ => return false,
    }
    true
}

/// xterm's default palette: the 16 named colors, the 6x6x6 cube and the 24 grays. Adapters answer
/// color queries with it so every engine says the same.
pub fn xterm_palette(index: u8) -> (u8, u8, u8) {
    const NAMED: [(u8, u8, u8); 16] = [
        (0x00, 0x00, 0x00),
        (0xcd, 0x00, 0x00),
        (0x00, 0xcd, 0x00),
        (0xcd, 0xcd, 0x00),
        (0x00, 0x00, 0xee),
        (0xcd, 0x00, 0xcd),
        (0x00, 0xcd, 0xcd),
        (0xe5, 0xe5, 0xe5),
        (0x7f, 0x7f, 0x7f),
        (0xff, 0x00, 0x00),
        (0x00, 0xff, 0x00),
        (0xff, 0xff, 0x00),
        (0x5c, 0x5c, 0xff),
        (0xff, 0x00, 0xff),
        (0x00, 0xff, 0xff),
        (0xff, 0xff, 0xff),
    ];
    match index {
        0..=15 => NAMED[index as usize],
        16..=231 => {
            let i = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
            (level(i / 36), level(i / 6 % 6), level(i % 6))
        }
        _ => {
            let v = 8 + 10 * (index - 232);
            (v, v, v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::xterm_palette;

    #[test]
    fn palette_matches_xterms_cube_and_grays() {
        assert_eq!(xterm_palette(16), (0, 0, 0));
        assert_eq!(xterm_palette(21), (0, 0, 255));
        assert_eq!(xterm_palette(196), (255, 0, 0));
        assert_eq!(xterm_palette(231), (255, 255, 255));
        assert_eq!(xterm_palette(232), (8, 8, 8));
        assert_eq!(xterm_palette(255), (238, 238, 238));
    }
}
